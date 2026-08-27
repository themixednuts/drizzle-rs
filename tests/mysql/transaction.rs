//! MySQL-specific transaction lifecycle, configuration, and session contracts.

use crate::common::schema::mysql::*;
use drizzle::core::expr::count;
use drizzle::mysql::prelude::*;

macro_rules! user {
    ($name:expr) => {
        InsertUser::new($name, true, Role::Member, vec![], 0, 0.0).with_note(None::<String>)
    };
}

#[drizzle::test]
fn explicit_completion_drop_and_panic_have_raii_semantics(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;

    {
        let tx = result!(db.begin_transaction(MySQLTransactionConfig::default()))?;
        result!(tx.insert(users).value(user!("drop rollback")).execute())?;
    }
    let after_drop: i64 = db.select(count(users.id)).from(users).get();
    assert_eq!(after_drop, 0);

    let tx = result!(db.begin_transaction(MySQLTransactionConfig::default()))?;
    result!(tx.insert(users).value(user!("explicit commit")).execute())?;
    result!(tx.commit())?;
    let after_commit: i64 = db.select(count(users.id)).from(users).get();
    assert_eq!(after_commit, 1);

    let tx = result!(db.begin_transaction(MySQLTransactionConfig::default()))?;
    result!(tx.insert(users).value(user!("explicit rollback")).execute())?;
    result!(tx.rollback())?;
    let after_rollback: i64 = db.select(count(users.id)).from(users).get();
    assert_eq!(after_rollback, 1);

    let panic: Result<drizzle::Result<()>, _> =
        catch!(db.transaction(MySQLTransactionConfig::default(), |tx| {
            result!(tx.insert(users).value(user!("panic rollback")).execute())?;
            panic!("rollback transaction after callback panic");
        },));
    assert!(panic.is_err());
    let after_panic: i64 = db.select(count(users.id)).from(users).get();
    assert_eq!(after_panic, 1);
}

#[drizzle::test]
fn consistent_snapshot_options_execute(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;
    db.insert(users).value(user!("snapshot")).execute();

    let config = MySQLTransactionConfig::default()
        .isolation_level(MySQLIsolationLevel::RepeatableRead)
        .access_mode(MySQLAccessMode::ReadOnly)
        .with_consistent_snapshot();
    db.transaction(config, |tx| {
        assert_eq!(
            result!(tx.select(count(users.id)).from(users).get::<i64, _, _>())?,
            1
        );
        Ok(())
    });
}

#[drizzle::test]
fn transaction_session_changes_are_repaired_on_parent_reuse(db: &mut TestDb<TestSchema>) {
    db.transaction(MySQLTransactionConfig::default(), |tx| {
        result!(tx.execute(SQL::raw("SET SESSION time_zone = '+01:00'")))?;
        Ok(())
    });

    let timezone: String = db.get(SQL::raw("SELECT @@SESSION.time_zone"));
    assert_eq!(timezone, "+00:00");
}
