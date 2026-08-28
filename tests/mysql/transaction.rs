//! MySQL-specific transaction lifecycle, configuration, and session contracts.

use crate::common::schema::mysql::*;
use drizzle::core::expr::count;
use drizzle::mysql::prelude::*;

#[drizzle::test]
fn callback_panic_rolls_back(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;

    let panic: Result<drizzle::Result<()>, _> =
        catch!(db.transaction(TransactionConfig::default(), |tx| {
            result!(
                tx.insert(users)
                    .value(
                        InsertUser::new("panic rollback", true, Role::Member, vec![], 0, 0.0)
                            .with_note(None::<String>),
                    )
                    .execute()
            )?;
            panic!("rollback transaction after callback panic");
        },));
    assert!(panic.is_err());
    let after_panic: i64 = db.select(count(users.id)).from(users).get();
    assert_eq!(after_panic, 0);
}

#[drizzle::test]
fn consistent_snapshot_options_execute(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;
    db.insert(users)
        .value(
            InsertUser::new("snapshot", true, Role::Member, vec![], 0, 0.0)
                .with_note(None::<String>),
        )
        .execute();

    let config = TransactionConfig::builder()
        .repeatable_read()
        .read_only()
        .snapshot()
        .build();
    result!(db.transaction(config, |tx| {
        let isolation: String = result!(tx.get(SQL::raw("SELECT @@transaction_isolation")))?;
        assert_eq!(isolation, "REPEATABLE-READ");
        assert_eq!(
            result!(tx.select(count(users.id)).from(users).get::<i64, _, _>())?,
            1
        );

        let write: drizzle::Result<_> = result!(
            tx.insert(users)
                .value(
                    InsertUser::new("rejected", true, Role::Member, vec![], 0, 0.0)
                        .with_note(None::<String>),
                )
                .execute()
        );
        assert!(write.is_err());
        Ok(())
    }))?;
}

#[drizzle::test]
fn transaction_session_changes_are_repaired_on_parent_reuse(db: &mut TestDb<TestSchema>) {
    db.transaction(TransactionConfig::default(), |tx| {
        result!(tx.execute(SQL::raw("SET SESSION time_zone = '+01:00'")))?;
        Ok(())
    });

    let timezone: String = db.get(SQL::raw("SELECT @@SESSION.time_zone"));
    assert_eq!(timezone, "+00:00");
}
