//! Shared savepoint orchestration for transaction drivers.
//!
//! Every driver implements its own `Transaction` type, but the savepoint
//! protocol (`SAVEPOINT N` → run callback → `RELEASE` / `ROLLBACK TO`) is
//! identical across SQLite and Postgres. The helpers in this module own
//! that protocol so individual drivers only supply the `execute_raw`
//! closure and the callback to invoke between bookends.
//!
//! Sync drivers use [`std::panic::catch_unwind`] so that a panic inside
//! the callback issues `ROLLBACK TO SAVEPOINT` before re-raising the
//! panic. Async drivers cannot reasonably catch panics across `.await`
//! points and simply propagate the `Err`.
//!
//! Drivers track nesting depth in an [`AtomicU32`] — the helper reads it,
//! increments before running the callback, and restores it before issuing
//! the release/rollback. Each nesting level therefore gets a unique
//! identifier (`drizzle_sp_0`, `drizzle_sp_1`, …).

use core::sync::atomic::{AtomicU32, Ordering};

use drizzle_core::error::Result;

/// Run a savepoint block synchronously around `body`.
///
/// `execute_raw` issues a raw, parameterless statement against the active
/// transaction; the helper invokes it three times: `SAVEPOINT`,
/// `RELEASE SAVEPOINT`, and `ROLLBACK TO SAVEPOINT`.
///
/// If `body` panics, the helper attempts `ROLLBACK TO SAVEPOINT` and
/// `RELEASE SAVEPOINT` (errors ignored) and re-raises the panic via
/// [`std::panic::resume_unwind`].
pub fn sync_savepoint<R>(
    depth: &AtomicU32,
    mut execute_raw: impl FnMut(&str) -> Result<()>,
    body: impl FnOnce() -> Result<R>,
) -> Result<R> {
    let level = depth.load(Ordering::Relaxed);
    let sp = format!("drizzle_sp_{level}");
    depth.store(level + 1, Ordering::Relaxed);

    execute_raw(&format!("SAVEPOINT {sp}"))?;

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));

    depth.store(level, Ordering::Relaxed);

    match outcome {
        Ok(Ok(value)) => {
            execute_raw(&format!("RELEASE SAVEPOINT {sp}"))?;
            Ok(value)
        }
        Ok(Err(e)) => {
            let _ = execute_raw(&format!("ROLLBACK TO SAVEPOINT {sp}"));
            let _ = execute_raw(&format!("RELEASE SAVEPOINT {sp}"));
            Err(e)
        }
        Err(panic_payload) => {
            let _ = execute_raw(&format!("ROLLBACK TO SAVEPOINT {sp}"));
            let _ = execute_raw(&format!("RELEASE SAVEPOINT {sp}"));
            std::panic::resume_unwind(panic_payload);
        }
    }
}

/// Run a savepoint block asynchronously around `body`.
///
/// Same protocol as [`sync_savepoint`] minus panic catching: async
/// drivers can't unwind across `.await` boundaries, so a panicking `body`
/// simply surfaces without rollback. Errors returned from `body` trigger
/// `ROLLBACK TO SAVEPOINT` followed by `RELEASE SAVEPOINT`.
pub async fn async_savepoint<R, Exec, ExecFut, BodyFut>(
    depth: &AtomicU32,
    mut execute_raw: Exec,
    body: BodyFut,
) -> Result<R>
where
    Exec: FnMut(String) -> ExecFut,
    ExecFut: core::future::Future<Output = Result<()>>,
    BodyFut: core::future::Future<Output = Result<R>>,
{
    let level = depth.load(Ordering::Relaxed);
    let sp = format!("drizzle_sp_{level}");
    depth.store(level + 1, Ordering::Relaxed);

    execute_raw(format!("SAVEPOINT {sp}")).await?;

    let outcome = body.await;

    depth.store(level, Ordering::Relaxed);

    match outcome {
        Ok(value) => {
            execute_raw(format!("RELEASE SAVEPOINT {sp}")).await?;
            Ok(value)
        }
        Err(e) => {
            let _ = execute_raw(format!("ROLLBACK TO SAVEPOINT {sp}")).await;
            let _ = execute_raw(format!("RELEASE SAVEPOINT {sp}")).await;
            Err(e)
        }
    }
}
