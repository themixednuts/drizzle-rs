//! mysql-sync-only connection, session, and pool ownership contracts.

use crate::common::{helpers::mysql_sync_setup, schema::mysql::*};
use drizzle::{
    core::expr::count,
    error::DrizzleError,
    migrations::Snapshot,
    mysql::{mysql_sync::Drizzle, prelude::*},
};
use mysql::prelude::Queryable as _;

#[test]
fn direct_connection_access_reestablishes_session_invariants() -> drizzle::Result<()> {
    let _guard = mysql_sync_setup::acquire_lock();
    let schema = TestSchema::new();
    let mut connection = mysql::Conn::new(mysql_sync_setup::options())
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    mysql_sync_setup::reset_schema(&mut connection, &schema);
    let (mut db, TestSchema { users, .. }) = Drizzle::new(connection, schema);
    db.create()?;

    db.conn_mut().query_drop(
        "SET SESSION time_zone = '+01:00', sql_mode = CONCAT_WS(',', @@SESSION.sql_mode, 'NO_UNSIGNED_SUBTRACTION', 'REAL_AS_FLOAT')",
    ).map_err(|error| DrizzleError::driver("MySQL", error))?;
    let _: i64 = db.select(count(users.id)).from(users).get()?;

    let mode: Option<String> = db
        .conn_mut()
        .query_first("SELECT @@SESSION.sql_mode")
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let timezone: Option<String> = db
        .conn_mut()
        .query_first("SELECT @@SESSION.time_zone")
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let mode = mode.unwrap_or_default();
    assert!(!mode.contains("NO_UNSIGNED_SUBTRACTION"));
    assert!(!mode.contains("REAL_AS_FLOAT"));
    assert_eq!(timezone.as_deref(), Some("+00:00"));
    mysql_sync_setup::reset_schema(db.conn_mut(), &TestSchema::new());
    Ok(())
}

#[test]
fn pooled_connection_is_returned_after_transaction_use() -> drizzle::Result<()> {
    let _guard = mysql_sync_setup::acquire_lock();
    let pool_options =
        mysql::PoolOpts::default().with_constraints(mysql::PoolConstraints::new_const::<1, 1>());
    let pool = mysql::Pool::new(
        mysql::OptsBuilder::from_opts(mysql_sync_setup::options()).pool_opts(pool_options),
    )
    .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let mut connection = pool
        .get_conn()
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let schema = TestSchema::new();
    mysql_sync_setup::reset_schema(&mut connection, &schema);
    let (mut db, TestSchema { users, .. }) = Drizzle::new(connection, schema);
    db.create()?;

    db.transaction(MySQLTransactionConfig::default(), |tx| {
        tx.insert(users)
            .value(
                InsertUser::new("pooled", true, Role::Member, vec![], 0, 0.0)
                    .with_note(None::<String>),
            )
            .execute()?;
        Ok(())
    })?;
    mysql_sync_setup::reset_schema(db.conn_mut(), &TestSchema::new());
    drop(db.into_inner());

    let returned = pool
        .get_conn()
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    drop(returned);
    Ok(())
}

#[test]
fn pooled_connection_introspects_and_pushes_on_its_checkout() -> drizzle::Result<()> {
    let _guard = mysql_sync_setup::acquire_lock();
    let pool_options =
        mysql::PoolOpts::default().with_constraints(mysql::PoolConstraints::new_const::<1, 1>());
    let pool = mysql::Pool::new(
        mysql::OptsBuilder::from_opts(mysql_sync_setup::options()).pool_opts(pool_options),
    )
    .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let mut connection = pool
        .get_conn()
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let schema = TestSchema::new();
    mysql_sync_setup::reset_schema(&mut connection, &schema);
    let (mut db, schema) = Drizzle::new(connection, schema);

    db.create()?;
    db.execute(SQL::raw("DROP TABLE test_posts"))?;
    db.push(&schema)?;
    let Snapshot::MySQL(snapshot) = db.introspect()? else {
        panic!("MySQL pooled introspection returned another dialect");
    };
    let ddl = drizzle::migrations::mysql::MySQLDDL::try_from_entities(snapshot.ddl)
        .expect("pooled introspection returns a valid MySQL snapshot");
    assert!(ddl.tables.one(None, "test_users").is_some());
    db.push(&schema)?;

    mysql_sync_setup::reset_schema(db.conn_mut(), &schema);
    drop(db.into_inner());
    let returned = pool
        .get_conn()
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    drop(returned);
    Ok(())
}
