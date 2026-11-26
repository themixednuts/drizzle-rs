//! Pooler mode (`Drizzle::pooler`): queries for a transaction-mode pooler,
//! such as PgBouncer, which may run each one on another server connection.
//!
//! The test runs on a direct connection: it checks that pooler mode leaves
//! the session without named statements, which are what break behind a
//! pooler.
//! With `DRIZZLE_PG_POOLER_URL` set, every test handle (`setup_db`) goes
//! through that pooler in pooler mode, which runs the whole suite behind it.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::Priority;
use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;

#[PostgresTable(NAME = "pg_pooler_items")]
struct PoolerItem {
    #[column(serial, primary)]
    id: i32,
    name: String,
    note: Option<String>,
    tags: Vec<String>,
    scores: Vec<i32>,
    #[column(enum)]
    priority: Priority,
}

#[derive(PostgresSchema)]
struct PoolerSchema {
    priority: Priority,
    items: PoolerItem,
}

#[derive(PostgresFromRow)]
#[from(PoolerItem)]
struct ItemRow {
    name: String,
    note: Option<String>,
    tags: Vec<String>,
    scores: Vec<i32>,
}

#[derive(PostgresFromRow)]
struct NamedStatements {
    count: i64,
}

/// The client a prepared statement runs on, for either driver.
trait PreparedClient {
    type Client<'c>
    where
        Self: 'c;

    fn prepared_client(&mut self) -> Self::Client<'_>;
}

#[cfg(feature = "tokio-postgres")]
impl<S> PreparedClient for drizzle::postgres::tokio::Drizzle<S> {
    type Client<'c>
        = &'c tokio_postgres::Client
    where
        S: 'c;

    fn prepared_client(&mut self) -> Self::Client<'_> {
        self.conn()
    }
}

#[cfg(feature = "postgres-sync")]
impl<S> PreparedClient for drizzle::postgres::sync::Drizzle<S> {
    type Client<'c>
        = &'c mut postgres::Client
    where
        S: 'c;

    fn prepared_client(&mut self) -> Self::Client<'_> {
        self.conn_mut()
    }
}

const NAMED_STATEMENTS: &str = "SELECT count(*)::bigint AS count FROM pg_prepared_statements";

#[drizzle::test]
fn pooler_mode_runs_unnamed_statements(db: &mut TestDb<PoolerSchema>) {
    let PoolerSchema { items, .. } = schema;
    let mut pooled = result!(db.pooled());

    // NULLs, enum labels and arrays have no type of their own: the server
    // infers them, as it does for a named statement.
    pooled
        .insert(items)
        .values([InsertPoolerItem::new(
            "a",
            vec!["x".to_string(), "y\"z".to_string()],
            vec![1, 2],
            Priority::High,
        )])
        .execute();
    pooled
        .insert(items)
        .values([InsertPoolerItem::new(
            "b",
            Vec::<String>::new(),
            Vec::<i32>::new(),
            Priority::Low,
        )
        .with_note("kept")])
        .execute();
    pooled
        .update(items)
        .set(UpdatePoolerItem::default().with_note("set"))
        .r#where(eq(items.priority, Priority::High))
        .execute();
    pooled.execute(
        SQL::raw(r#"UPDATE "pg_pooler_items" SET "tags" = "#)
            .append(SQL::param(PostgresValue::Array(vec![
                PostgresValue::Text("x".into()),
                PostgresValue::Text("w".into()),
            ])))
            .append(SQL::raw(r#", "note" = "#))
            .append(SQL::param(PostgresValue::Null))
            .append(SQL::raw(r#" WHERE "name" = 'b'"#)),
    );

    // Not the enum column: tokio-postgres looks its type up with a named
    // statement of its own.
    let rows: Vec<ItemRow> = pooled
        .select(ItemRow::Select)
        .from(items)
        .order_by(asc(items.name))
        .all();
    let rows: Vec<_> = rows
        .into_iter()
        .map(|row| (row.name, row.note, row.tags, row.scores))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "a".to_string(),
                Some("set".to_string()),
                vec!["x".to_string(), "y\"z".to_string()],
                vec![1, 2],
            ),
            (
                "b".to_string(),
                None,
                vec!["x".to_string(), "w".to_string()],
                vec![],
            ),
        ]
    );

    let name = items.name.placeholder("pooler_name");
    let prepared = pooled
        .select(items.note)
        .from(items)
        .r#where(eq(items.name, name))
        .prepare()
        .into_owned();
    let note: Option<Option<String>> = prepared.get(pooled.prepared_client(), [name.bind("b")]);
    assert_eq!(note, Some(None));

    result!(pooled.transaction(TransactionConfig::default(), |tx| {
        result!(
            tx.delete(items)
                .r#where(eq(items.priority, Priority::Low))
                .execute()
        )?;
        Ok(())
    }))?;
    let names: Vec<String> = pooled.select(items.name).from(items).all();
    assert_eq!(names, ["a"]);

    let named: NamedStatements = result!(pooled.get(SQL::raw(NAMED_STATEMENTS)))?.expect("a row");
    assert_eq!(named.count, 0, "pooler mode prepared named statements");

    // Out of pooler mode, the same session names and caches its statements,
    // which is what the check above tells apart.
    pooled = pooled.pooler(false);
    let _: Vec<String> = pooled.select(items.name).from(items).all();
    let named: NamedStatements = result!(pooled.get(SQL::raw(NAMED_STATEMENTS)))?.expect("a row");
    assert!(named.count > 0);
}
