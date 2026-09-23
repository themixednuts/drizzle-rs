//! A Cloudflare Worker whose Durable Object builds its drizzle handle once,
//! migrates in its constructor, and writes through a transaction.
//!
//! Run it locally with `npx wrangler dev` (after `cargo install worker-build`),
//! then `curl "http://localhost:8787/?name=Alice"`: each request adds a user
//! and returns every name stored so far.

use drizzle::error::DrizzleError;
use drizzle::migrations::Tracking;
use drizzle::sqlite::durable::{Drizzle, DurableStorage};
use drizzle::sqlite::prelude::*;
use worker::{
    Context, DurableObject, Env, Request, Response, Result, State, durable_object, event,
};

#[SQLiteTable]
pub struct User {
    #[column(primary)]
    id: i32,
    name: String,
}

#[derive(SQLiteSchema)]
pub struct AppSchema {
    user: User,
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let stub = env
        .durable_object("COUNTER")?
        .id_from_name("example")?
        .get_stub()?;
    stub.fetch_with_request(req).await
}

#[durable_object]
pub struct Counter {
    db: Drizzle<AppSchema>,
}

impl DurableObject for Counter {
    fn new(mut state: State, _env: Env) -> Self {
        // Runs once per instantiation, before the object receives any event.
        let migrations = drizzle::include_migrations!("./drizzle");
        let (db, _) = Drizzle::new(DurableStorage::new(&mut state));
        db.migrate(&migrations, Tracking::SQLITE)
            .expect("durable migrations failed");
        Self { db }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        let name = req
            .url()?
            .query_pairs()
            .find(|(key, _)| key == "name")
            .map_or_else(|| "Alice".to_owned(), |(_, value)| value.into_owned());
        let AppSchema { user } = *self.db.schema();

        // `worker::Error` has no `From<DrizzleError>`, so convert before `?`.
        let users: Vec<SelectUser> = self
            .db
            .transaction(|tx| {
                tx.insert(user)
                    .values([InsertUser::new(name.as_str())])
                    .execute()?;
                // A savepoint that fails undoes only its own writes.
                let undone = tx.savepoint(|sp| {
                    sp.insert(user)
                        .values([InsertUser::new("undone")])
                        .execute()?;
                    Err::<(), _>(DrizzleError::Other("undo".into()))
                });
                debug_assert!(undone.is_err());
                tx.select(()).from(user).all()
            })
            .map_err(|e| worker::Error::RustError(e.to_string()))?;

        let names: Vec<String> = users.into_iter().map(|user| user.name).collect();
        Response::from_json(&names)
    }
}
