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

use drizzle_core::error::{DrizzleError, Result};

fn cleanup_error(
    scope: &str,
    original: DrizzleError,
    action: &str,
    cleanup: DrizzleError,
) -> DrizzleError {
    DrizzleError::TransactionError(
        format!("{scope} callback failed: {original}; {action} failed: {cleanup}").into(),
    )
}

fn trace_panic_cleanup_error(scope: &str, name: &str, action: &str, err: &DrizzleError) {
    #[cfg(feature = "tracing")]
    tracing::error!(
        scope,
        name,
        action,
        error = %err,
        "transaction cleanup failed after panic"
    );

    #[cfg(not(feature = "tracing"))]
    let _ = (scope, name, action, err);
}

/// Run a synchronous transaction around `body`.
///
/// The driver owns how a transaction is begun and supplies commit/rollback
/// closures for its transaction type. This helper centralizes the shared
/// callback protocol: commit on `Ok`, rollback on `Err`, and rollback before
/// resuming a panic.
pub fn sync_transaction<Tx, R>(
    transaction: Tx,
    trace_name: &'static str,
    trace_commit: impl Fn(),
    trace_rollback: impl Fn(),
    body: impl FnOnce(&Tx) -> Result<R>,
    commit: impl FnOnce(Tx) -> Result<()>,
    rollback: impl FnOnce(Tx) -> Result<()>,
) -> Result<R> {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(&transaction)));

    match outcome {
        Ok(Ok(value)) => {
            trace_commit();
            commit(transaction)?;
            Ok(value)
        }
        Ok(Err(e)) => {
            trace_rollback();
            match rollback(transaction) {
                Ok(()) => Err(e),
                Err(rollback_err) => Err(cleanup_error("transaction", e, "rollback", rollback_err)),
            }
        }
        Err(panic_payload) => {
            trace_rollback();
            if let Err(err) = rollback(transaction) {
                trace_panic_cleanup_error("transaction", trace_name, "rollback", &err);
            }
            std::panic::resume_unwind(panic_payload);
        }
    }
}

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
            if let Err(rollback_err) = execute_raw(&format!("ROLLBACK TO SAVEPOINT {sp}")) {
                return Err(cleanup_error(
                    "savepoint",
                    e,
                    "rollback to savepoint",
                    rollback_err,
                ));
            }
            if let Err(release_err) = execute_raw(&format!("RELEASE SAVEPOINT {sp}")) {
                return Err(cleanup_error(
                    "savepoint",
                    e,
                    "release savepoint after rollback",
                    release_err,
                ));
            }
            Err(e)
        }
        Err(panic_payload) => {
            if let Err(err) = execute_raw(&format!("ROLLBACK TO SAVEPOINT {sp}")) {
                trace_panic_cleanup_error("savepoint", &sp, "rollback to savepoint", &err);
            }
            if let Err(err) = execute_raw(&format!("RELEASE SAVEPOINT {sp}")) {
                trace_panic_cleanup_error(
                    "savepoint",
                    &sp,
                    "release savepoint after rollback",
                    &err,
                );
            }
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
            if let Err(rollback_err) = execute_raw(format!("ROLLBACK TO SAVEPOINT {sp}")).await {
                return Err(cleanup_error(
                    "savepoint",
                    e,
                    "rollback to savepoint",
                    rollback_err,
                ));
            }
            if let Err(release_err) = execute_raw(format!("RELEASE SAVEPOINT {sp}")).await {
                return Err(cleanup_error(
                    "savepoint",
                    e,
                    "release savepoint after rollback",
                    release_err,
                ));
            }
            Err(e)
        }
    }
}
