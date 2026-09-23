//! The storage handle the Durable Objects driver runs on.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use ::worker::worker_sys::DurableObjectState;
use ::worker::{SqlStorage, State};
use drizzle_core::error::DrizzleError;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// A `DurableObjectStorage`, seen through the method `worker` doesn't bind.
    #[wasm_bindgen(extends = js_sys::Object)]
    #[derive(Clone)]
    type SyncTransactionStorage;

    /// Runs `callback` inside a savepoint: released when it returns, rolled
    /// back (and the exception rethrown) when it throws. Nested calls nest
    /// savepoints.
    #[wasm_bindgen(method, catch, js_name = transactionSync)]
    fn transaction_sync(
        this: &SyncTransactionStorage,
        callback: &mut dyn FnMut() -> Result<JsValue, JsValue>,
    ) -> Result<JsValue, JsValue>;
}

/// A Durable Object's storage: its SQL database and the runtime's
/// transaction entry point.
///
/// Durable Object SQL rejects `BEGIN`, `COMMIT` and `SAVEPOINT` statements, so
/// [`Drizzle::transaction`](super::Drizzle::transaction) and
/// [`Transaction::savepoint`](crate::transaction::sqlite::durable::Transaction::savepoint)
/// run through the storage object's `transactionSync` instead. The `worker`
/// crate doesn't bind that method, which is why the driver is built from the
/// object's [`State`] rather than from its [`SqlStorage`].
#[derive(Clone)]
pub struct DurableStorage {
    sql: SqlStorage,
    storage: SyncTransactionStorage,
}

impl std::fmt::Debug for DurableStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DurableStorage").finish_non_exhaustive()
    }
}

impl DurableStorage {
    /// Takes the storage of the Durable Object that owns `state`.
    ///
    /// `worker` hands the underlying storage object out only by value, so this
    /// moves `state` out for the length of the call and puts it back. `state`
    /// is unchanged afterwards. Build the driver once, in
    /// `DurableObject::new`, and keep it on the object.
    ///
    /// # Panics
    ///
    /// If the runtime gives the object no storage, as [`State::storage`] does.
    pub fn new(state: &mut State) -> Self {
        let sql = state.storage().sql();
        let placeholder = State::from(JsValue::UNDEFINED.unchecked_into::<DurableObjectState>());
        let raw = std::mem::replace(state, placeholder)._inner();
        let storage = raw.storage();
        *state = State::from(raw);
        let storage = storage
            .expect("Durable Object state has no storage")
            .unchecked_into::<SyncTransactionStorage>();
        Self { sql, storage }
    }

    /// The object's SQL API.
    #[inline]
    pub fn sql(&self) -> &SqlStorage {
        &self.sql
    }

    /// Runs `body` in a transaction that commits when it returns `Ok` and
    /// rolls back when it returns `Err` or panics. A call made while another
    /// is running becomes a savepoint inside it.
    pub(crate) fn transaction<R>(
        &self,
        body: impl FnOnce() -> drizzle_core::error::Result<R>,
    ) -> drizzle_core::error::Result<R> {
        let mut body = Some(body);
        let mut outcome = None;
        let mut panic = None;
        let mut callback = || {
            // `transactionSync` calls this once; a second call finds no body.
            let Some(body) = body.take() else {
                return Err(JsValue::from_str("transaction callback called twice"));
            };
            match catch_unwind(AssertUnwindSafe(body)) {
                Ok(result) => {
                    // Throwing is how the callback asks for a rollback.
                    let signal = match &result {
                        Ok(_) => Ok(JsValue::UNDEFINED),
                        Err(error) => Err(js_sys::Error::new(&error.to_string()).into()),
                    };
                    outcome = Some(result);
                    signal
                }
                Err(payload) => {
                    panic = Some(payload);
                    Err(js_sys::Error::new("transaction callback panicked").into())
                }
            }
        };
        let completed = self.storage.transaction_sync(&mut callback);

        if let Some(payload) = panic {
            resume_unwind(payload);
        }
        match (outcome, completed) {
            (Some(Ok(value)), Ok(_)) => Ok(value),
            // The callback's own error, after the runtime rolled back.
            (Some(Err(error)), _) => Err(error),
            (Some(Ok(_)), Err(error)) => Err(DrizzleError::TransactionError(
                format!("commit failed: {}", js_message(&error)).into(),
            )),
            (None, Err(error)) => Err(DrizzleError::TransactionError(js_message(&error).into())),
            (None, Ok(_)) => Err(DrizzleError::TransactionError(
                "transactionSync returned without running the callback".into(),
            )),
        }
    }
}

fn js_message(value: &JsValue) -> String {
    match value.dyn_ref::<js_sys::Error>() {
        Some(error) => error.message().into(),
        None => value.as_string().unwrap_or_else(|| format!("{value:?}")),
    }
}
