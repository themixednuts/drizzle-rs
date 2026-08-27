//! `mysql_async`-only pool, runtime, and delayed-drop contracts.

use crate::common::{helpers::mysql_async_setup, schema::mysql::*};
use drizzle::{
    core::{
        SQL,
        expr::{count, eq},
    },
    migrations::{Migration, Tracking},
    mysql::{mysql_async::Drizzle, prelude::*},
};
use mysql_async::{Pool, PoolConstraints, PoolOpts, prelude::Queryable as _};

fn pool() -> Pool {
    let constraints = PoolConstraints::new(1, 1).expect("valid one-connection pool constraints");
    let options = mysql_async::OptsBuilder::from_opts(mysql_async_setup::options())
        .pool_opts(PoolOpts::default().with_constraints(constraints));
    Pool::new(options)
}

async fn setup_pool() -> (
    Drizzle<Pool, TestSchema>,
    TestSchema,
    tokio::sync::MutexGuard<'static, ()>,
) {
    let guard = mysql_async_setup::acquire_lock_async().await;
    let schema = TestSchema::new();
    let pool = pool();
    let mut connection = pool.get_conn().await.expect("checkout MySQL connection");
    mysql_async_setup::reset_schema(&mut connection, &schema).await;
    drop(connection);
    let (db, schema) = Drizzle::new(pool, schema);
    db.create().await.expect("create MySQL test schema");
    (db, schema, guard)
}

async fn cleanup(db: Drizzle<Pool, TestSchema>, schema: &TestSchema) {
    let mut connection = db
        .conn()
        .get_conn()
        .await
        .expect("checkout connection for cleanup");
    mysql_async_setup::reset_schema(&mut connection, schema).await;
    drop(connection);
    db.disconnect().await.expect("gracefully disconnect pool");
}

#[test]
fn pool_construction_is_lazy_and_runtime_owned_by_first_checkout() {
    let _guard = mysql_async_setup::acquire_lock();
    let pool = pool();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build Tokio runtime");
    runtime.block_on(async move {
        let mut connection = pool.get_conn().await.expect("first checkout binds runtime");
        connection
            .ping()
            .await
            .expect("connection works on owner runtime");
        drop(connection);
        pool.disconnect()
            .await
            .expect("disconnect on owner runtime");
    });
}

#[tokio::test]
async fn pooled_transactions_rollback_explicitly_and_before_reuse() -> drizzle::Result<()> {
    let (db, TestSchema { users, .. }, _guard) = setup_pool().await;

    let transaction = db
        .begin_transaction(MySQLTransactionConfig::default())
        .await?;
    transaction
        .insert(users)
        .value(
            InsertUser::new("explicit rollback", true, Role::Member, vec![], 0, 0.0)
                .with_note(None::<String>),
        )
        .execute()
        .await?;
    transaction.rollback().await?;
    let after_explicit: i64 = db.select(count(users.id)).from(users).get().await?;
    assert_eq!(after_explicit, 0);

    {
        let transaction = db
            .begin_transaction(MySQLTransactionConfig::default())
            .await?;
        transaction
            .insert(users)
            .value(
                InsertUser::new("drop rollback", true, Role::Member, vec![], 0, 0.0)
                    .with_note(None::<String>),
            )
            .execute()
            .await?;
    }
    // The pool has max=1, so this checkout cannot complete until the recycler
    // has rolled the dropped transaction back and made that same connection safe.
    let after_drop: i64 = db.select(count(users.id)).from(users).get().await?;
    assert_eq!(after_drop, 0);

    cleanup(db, &TestSchema::new()).await;
    drizzle::Result::Ok(())
}

#[tokio::test]
async fn prepared_queries_execute_through_one_pool_checkout() -> drizzle::Result<()> {
    let (db, schema, _guard) = setup_pool().await;
    let TestSchema { users, .. } = schema;
    db.insert(users)
        .values([
            InsertUser::new("Alice", true, Role::Member, vec![], 0, 0.0).with_note(None::<String>),
            InsertUser::new("Bob", true, Role::Member, vec![], 0, 0.0).with_note(None::<String>),
        ])
        .execute()
        .await?;

    let name = users.name.placeholder("name");
    let prepared = db
        .select(())
        .from(users)
        .r#where(eq(users.name, name))
        .prepare()
        .into_owned();
    let alice: SelectUser = prepared.get(db.conn(), [name.bind("Alice")]).await?;
    assert_eq!(alice.name, "Alice");

    cleanup(db, &schema).await;
    Ok(())
}

#[tokio::test]
async fn disconnect_closes_every_pool_clone() -> drizzle::Result<()> {
    let (db, schema, _guard) = setup_pool().await;
    let observer = db.conn().clone();
    let mut connection = observer
        .get_conn()
        .await
        .map_err(|error| drizzle::error::DrizzleError::driver("MySQL", error))?;
    mysql_async_setup::reset_schema(&mut connection, &schema).await;
    drop(connection);

    db.disconnect().await?;
    assert!(matches!(
        observer.get_conn().await,
        Err(mysql_async::Error::Driver(
            mysql_async::DriverError::PoolDisconnected
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn pooled_migrations_use_one_checkout() -> drizzle::Result<()> {
    let (db, schema, _guard) = setup_pool().await;
    let tracking = Tracking::MYSQL.table("__drizzle_pool_migrations");
    for table in ["mysql_pool_migration", "__drizzle_pool_migrations"] {
        db.execute(SQL::raw(format!("DROP TABLE IF EXISTS `{table}`")))
            .await?;
    }

    let migrations = [Migration::with_hash(
        "20260827000003_mysql_pool_migration",
        "mysql-pool-migration",
        1_787_788_800_003,
        vec![
            "CREATE TABLE `mysql_pool_migration` (`id` INT PRIMARY KEY)".to_owned(),
            "INSERT INTO `mysql_pool_migration` (`id`) VALUES (1)".to_owned(),
        ],
    )];

    let outcome = db.migrate(&migrations, tracking.clone()).await?;
    assert_eq!(
        outcome.applied_tags(),
        ["20260827000003_mysql_pool_migration"]
    );
    let id: i64 = db
        .get(SQL::raw("SELECT `id` FROM `mysql_pool_migration`"))
        .await?;
    assert_eq!(id, 1);
    assert!(
        db.migrate(&migrations, tracking.clone())
            .await?
            .is_up_to_date()
    );

    for table in ["mysql_pool_migration", "__drizzle_pool_migrations"] {
        db.execute(SQL::raw(format!("DROP TABLE IF EXISTS `{table}`")))
            .await?;
    }
    cleanup(db, &schema).await;
    Ok(())
}
