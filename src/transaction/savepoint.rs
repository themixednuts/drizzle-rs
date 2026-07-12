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
//! Synchronous drivers track nesting depth in an [`AtomicU32`]. Async drivers
//! use [`AsyncSavepointState`], which assigns monotonic names, orders cleanup
//! in LIFO order when futures overlap, and poisons the transaction if a
//! savepoint future is cancelled.

use core::sync::atomic::{AtomicU32, Ordering};
use std::{
    collections::HashMap,
    future::poll_fn,
    sync::Mutex,
    task::{Poll, Waker},
};

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

#[derive(Default)]
struct AsyncSavepointInner {
    next_id: u64,
    stack: Vec<u64>,
    poisoned: bool,
    waiters: HashMap<u64, Waker>,
}

/// Shared ordering and cancellation state for async transaction savepoints.
#[derive(Default)]
pub struct AsyncSavepointState(Mutex<AsyncSavepointInner>);

impl std::fmt::Debug for AsyncSavepointState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        f.debug_struct("AsyncSavepointState")
            .field("active", &state.stack.len())
            .field("poisoned", &state.poisoned)
            .finish()
    }
}

impl AsyncSavepointState {
    pub fn new() -> Self {
        Self(Mutex::new(AsyncSavepointInner {
            next_id: 0,
            stack: Vec::new(),
            poisoned: false,
            waiters: HashMap::new(),
        }))
    }

    /// Reject use after a savepoint future was cancelled or cleanup failed.
    pub fn ensure_usable(&self) -> Result<()> {
        let state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.poisoned {
            Err(DrizzleError::TransactionError(
                "transaction is unusable after cancelled or failed savepoint cleanup".into(),
            ))
        } else {
            Ok(())
        }
    }

    fn begin(&self) -> Result<u64> {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.poisoned {
            return Err(DrizzleError::TransactionError(
                "transaction is unusable after cancelled or failed savepoint cleanup".into(),
            ));
        }
        let id = state.next_id;
        state.next_id = state.next_id.wrapping_add(1);
        state.stack.push(id);
        Ok(id)
    }

    async fn wait_until_top(&self, id: u64) -> Result<()> {
        poll_fn(|context| {
            let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
            if state.poisoned {
                return Poll::Ready(Err(DrizzleError::TransactionError(
                    "transaction is unusable after cancelled or failed savepoint cleanup".into(),
                )));
            }
            if state.stack.last() == Some(&id) {
                state.waiters.remove(&id);
                Poll::Ready(Ok(()))
            } else if state.stack.contains(&id) {
                state.waiters.insert(id, context.waker().clone());
                Poll::Pending
            } else {
                Poll::Ready(Err(DrizzleError::TransactionError(
                    "savepoint ordering state was lost".into(),
                )))
            }
        })
        .await
    }

    fn finish(&self, id: u64) -> Result<()> {
        let (result, waiters) = {
            let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
            let result = if state.stack.pop() == Some(id) {
                Ok(())
            } else {
                state.poisoned = true;
                Err(DrizzleError::TransactionError(
                    "savepoint cleanup completed out of order".into(),
                ))
            };
            let waiters = state
                .waiters
                .drain()
                .map(|(_, waker)| waker)
                .collect::<Vec<_>>();
            (result, waiters)
        };
        for waker in waiters {
            waker.wake();
        }
        result
    }

    fn poison(&self) {
        let waiters = {
            let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
            state.poisoned = true;
            state
                .waiters
                .drain()
                .map(|(_, waker)| waker)
                .collect::<Vec<_>>()
        };
        for waker in waiters {
            waker.wake();
        }
    }
}

struct AsyncSavepointGuard<'a> {
    state: &'a AsyncSavepointState,
    armed: bool,
}

impl AsyncSavepointGuard<'_> {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for AsyncSavepointGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.state.poison();
        }
    }
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
/// Overlapping futures may run their bodies concurrently, but cleanup waits
/// for LIFO order. Dropping this future poisons `state`; transaction drivers
/// must call [`AsyncSavepointState::ensure_usable`] before subsequent work so
/// the outer transaction is forced to roll back.
pub async fn async_savepoint<R, Exec, ExecFut, BodyFut>(
    state: &AsyncSavepointState,
    mut execute_raw: Exec,
    body: BodyFut,
) -> Result<R>
where
    Exec: FnMut(String) -> ExecFut,
    ExecFut: core::future::Future<Output = Result<()>>,
    BodyFut: core::future::Future<Output = Result<R>>,
{
    let id = state.begin()?;
    let sp = format!("drizzle_sp_{id}");
    let mut guard = AsyncSavepointGuard { state, armed: true };

    execute_raw(format!("SAVEPOINT {sp}")).await?;

    let outcome = body.await;
    state.wait_until_top(id).await?;

    match outcome {
        Ok(value) => {
            execute_raw(format!("RELEASE SAVEPOINT {sp}")).await?;
            state.finish(id)?;
            guard.disarm();
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
            state.finish(id)?;
            guard.disarm();
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AsyncSavepointState, async_savepoint};
    use core::{future::Future, pin::Pin, task::Context};

    #[test]
    fn overlapping_async_savepoints_wait_for_lifo_cleanup() {
        let state = AsyncSavepointState::new();
        let outer = state.begin().expect("outer savepoint");
        let inner = state.begin().expect("inner savepoint");

        let mut outer_turn = Box::pin(state.wait_until_top(outer));
        let mut context = Context::from_waker(std::task::Waker::noop());
        assert!(Pin::new(&mut outer_turn).poll(&mut context).is_pending());

        state.finish(inner).expect("finish inner");
        assert!(Pin::new(&mut outer_turn).poll(&mut context).is_ready());
        state.finish(outer).expect("finish outer");
        state.ensure_usable().expect("state remains usable");
    }

    #[test]
    fn dropping_async_savepoint_future_poisons_state() {
        let state = AsyncSavepointState::new();
        let mut future = Box::pin(async_savepoint(
            &state,
            |_| std::future::ready(Ok(())),
            std::future::pending::<drizzle_core::error::Result<()>>(),
        ));
        let mut context = Context::from_waker(std::task::Waker::noop());
        assert!(Pin::new(&mut future).poll(&mut context).is_pending());

        drop(future);
        assert!(state.ensure_usable().is_err());
        assert!(state.begin().is_err());
    }
}
