//! Live acceptance tests for the blocking MySQL adapter.

use drizzle::{
    core::expr::{count, eq},
    error::DrizzleError,
    mysql::{mysql_sync::Drizzle, prelude::*},
};
use mysql::prelude::Queryable as _;
use std::sync::Mutex;

static MYSQL_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, PartialEq, Eq, MySQLEnum)]
enum UserRole {
    Member,
    Admin,
}

#[MySQLTable(NAME = "sync_users")]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
    active: bool,
    #[column(ENUM)]
    role: UserRole,
    note: Option<String>,
    payload: Vec<u8>,
    balance: i64,
    score: f64,
}

#[MySQLTable(NAME = "sync_posts")]
struct Posts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(REFERENCES = Users::id)]
    user_id: u64,
    #[column(VARCHAR(255))]
    title: String,
}

#[MySQLIndex(unique)]
struct UsersNameIdx(Users::name);

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    users_name_idx: UsersNameIdx,
    posts: Posts,
}

fn opts() -> mysql::Opts {
    let url = std::env::var("DRIZZLE_MYSQL_URL")
        .unwrap_or_else(|_| "mysql://drizzle:drizzle@127.0.0.1:3307/drizzle_test".to_owned());
    mysql::Opts::from_url(&url).expect("valid DRIZZLE_MYSQL_URL")
}

fn reset(connection: &mut impl mysql::prelude::Queryable) {
    connection
        .query_drop("DROP TABLE IF EXISTS `sync_posts`")
        .unwrap();
    connection
        .query_drop("DROP TABLE IF EXISTS `sync_users`")
        .unwrap();
}

macro_rules! insert_user {
    ($name:expr, $role:expr) => {
        InsertUsers::new($name, true, $role, vec![1, 2, 3], -42, 9.5).with_note(None::<String>)
    };
}

#[test]
fn connection_crud_joins_aggregates_prepared_and_session_invariants() -> drizzle::Result<()> {
    let _test_guard = MYSQL_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut connection =
        mysql::Conn::new(opts()).map_err(|error| DrizzleError::driver("MySQL", error))?;
    reset(&mut connection);
    let (mut db, Schema { users, posts, .. }) = Drizzle::new(connection, Schema::new());
    db.create()?;

    let inserted = db
        .insert(users)
        .value(insert_user!("Alice", UserRole::Admin))
        .execute()?;
    let alice_id = inserted.last_insert_id().expect("AUTO_INCREMENT id");
    assert_eq!(inserted.affected_rows(), 1);

    db.insert(users)
        .value(insert_user!("Bob", UserRole::Member))
        .execute()?;
    db.insert(posts)
        .value(InsertPosts::new(alice_id, "Hello"))
        .execute()?;

    let alice: SelectUsers = db
        .select(())
        .from(users)
        .r#where(eq(users.id, alice_id))
        .get()?;
    assert_eq!(alice.name, "Alice");
    assert_eq!(alice.role, UserRole::Admin);
    assert_eq!(alice.note, None);
    assert_eq!(alice.payload, vec![1, 2, 3]);
    assert_eq!(alice.balance, -42);
    assert_eq!(alice.score, 9.5);

    let joined: Vec<(SelectUsers, SelectPosts)> = db
        .select(())
        .from(users)
        .inner_join((posts, eq(posts.user_id, users.id)))
        .all()?;
    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].1.title, "Hello");

    let total: i64 = db.select(count(users.id)).from(users).get()?;
    assert_eq!(total, 2);

    let direct_query = db.select(()).from(users).detach();
    let direct_users: Vec<SelectUsers> = db.all(direct_query)?;
    assert_eq!(direct_users.len(), 2);
    let direct_query = db
        .select(())
        .from(users)
        .r#where(eq(users.id, alice_id))
        .detach();
    let direct_alice: SelectUsers = db.get(direct_query)?;
    assert_eq!(direct_alice.name, "Alice");

    let bob_ids = db
        .select(users.id)
        .from(users)
        .r#where(eq(users.name, "Bob"))
        .detach();
    let selected_ids: Vec<u64> = db
        .select(users.id)
        .from(users)
        .r#where(eq(users.name, "Alice"))
        .union_all(bob_ids)
        .all()?;
    assert_eq!(selected_ids.len(), 2);

    let name = users.name.placeholder("name");
    let find = db
        .select(())
        .from(users)
        .r#where(eq(users.name, name))
        .prepare()
        .into_owned();
    assert_eq!(find.param_count(), 1);
    let prepared: Vec<SelectUsers> = find.all(db.conn_mut(), [name.bind("Bob")])?;
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].name, "Bob");

    let new_name = users.name.placeholder("new_name");
    let user_id = users.id.placeholder("user_id");
    let rename = db
        .update(users)
        .set(UpdateUsers::default().with_name(new_name))
        .r#where(eq(users.id, user_id))
        .prepare()
        .into_owned();
    let renamed = rename.execute(
        db.conn_mut(),
        [new_name.bind("Bobby"), user_id.bind(prepared[0].id)],
    )?;
    assert_eq!(renamed.affected_rows(), 1);

    db.update(users)
        .set(UpdateUsers::default().with_note("updated"))
        .r#where(eq(users.id, alice_id))
        .execute()?;
    let updated: SelectUsers = db
        .select(())
        .from(users)
        .r#where(eq(users.id, alice_id))
        .get()?;
    assert_eq!(updated.note.as_deref(), Some("updated"));

    db.delete(posts)
        .r#where(eq(posts.user_id, alice_id))
        .execute()?;

    db.conn_mut()
        .query_drop(
            "SET SESSION time_zone = '+01:00', \
             sql_mode = CONCAT_WS(',', @@SESSION.sql_mode, 'NO_UNSIGNED_SUBTRACTION')",
        )
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let _: i64 = db.select(count(users.id)).from(users).get()?;

    let mode: Option<String> = db
        .conn_mut()
        .query_first("SELECT @@SESSION.sql_mode")
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    assert!(!mode.unwrap_or_default().contains("NO_UNSIGNED_SUBTRACTION"));
    let timezone: Option<String> = db
        .conn_mut()
        .query_first("SELECT @@SESSION.time_zone")
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    assert_eq!(timezone.as_deref(), Some("+00:00"));
    Ok(())
}

#[test]
fn pooled_checkout_transactions_savepoints_and_drop_rollback() -> drizzle::Result<()> {
    let _test_guard = MYSQL_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pool_options =
        mysql::PoolOpts::default().with_constraints(mysql::PoolConstraints::new_const::<1, 1>());
    let pool = mysql::Pool::new(mysql::OptsBuilder::from_opts(opts()).pool_opts(pool_options))
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    let mut connection = pool
        .get_conn()
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    reset(&mut connection);
    let (mut db, Schema { users, .. }) = Drizzle::new(connection, Schema::new());
    db.create()?;

    let callback_error: drizzle::Result<()> =
        db.transaction(MySQLTransactionConfig::default(), |tx| {
            tx.insert(users)
                .value(insert_user!("rolled-back", UserRole::Member))
                .execute()?;
            Err(DrizzleError::Other("rollback".into()))
        });
    assert!(callback_error.is_err());
    assert_eq!(
        db.select(count(users.id)).from(users).get::<i64, _, _>()?,
        0
    );

    db.transaction(MySQLTransactionConfig::default(), |tx| {
        tx.insert(users)
            .value(insert_user!("kept", UserRole::Admin))
            .execute()?;

        tx.execute(drizzle::sql!("SET SESSION time_zone = '+01:00'"))?;
        let timezone: String = tx.get(drizzle::sql!("SELECT @@SESSION.time_zone"))?;
        assert_eq!(timezone, "+00:00");

        let direct_query = tx.select(()).from(users).detach();
        let direct_users: Vec<SelectUsers> = tx.all(direct_query)?;
        assert_eq!(direct_users.len(), 1);
        let savepoint_error: drizzle::Result<()> = tx.savepoint(|savepoint| {
            savepoint
                .insert(users)
                .value(insert_user!("discarded", UserRole::Member))
                .execute()?;
            Err(DrizzleError::Other("savepoint rollback".into()))
        });
        assert!(savepoint_error.is_err());
        assert_eq!(
            tx.select(count(users.id)).from(users).get::<i64, _, _>()?,
            1
        );
        Ok(())
    })?;

    let snapshot = MySQLTransactionConfig::default()
        .isolation_level(MySQLIsolationLevel::RepeatableRead)
        .access_mode(MySQLAccessMode::ReadOnly)
        .with_consistent_snapshot();
    db.transaction(snapshot, |tx| {
        assert_eq!(
            tx.select(count(users.id)).from(users).get::<i64, _, _>()?,
            1
        );
        Ok(())
    })?;

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: drizzle::Result<()> = db.transaction(MySQLTransactionConfig::default(), |tx| {
            tx.insert(users)
                .value(insert_user!("panic rollback", UserRole::Member))
                .execute()?;
            panic!("rollback transaction after callback panic");
        });
    }));
    assert!(panic.is_err());
    assert_eq!(
        db.select(count(users.id)).from(users).get::<i64, _, _>()?,
        1
    );

    {
        let tx = db.begin_transaction(MySQLTransactionConfig::default())?;
        tx.insert(users)
            .value(insert_user!("drop rollback", UserRole::Member))
            .execute()?;
    }
    assert_eq!(
        db.select(count(users.id)).from(users).get::<i64, _, _>()?,
        1
    );

    let tx = db.begin_transaction(MySQLTransactionConfig::default())?;
    tx.insert(users)
        .value(insert_user!("explicit commit", UserRole::Member))
        .execute()?;
    tx.commit()?;
    assert_eq!(
        db.select(count(users.id)).from(users).get::<i64, _, _>()?,
        2
    );

    let tx = db.begin_transaction(MySQLTransactionConfig::default())?;
    tx.insert(users)
        .value(insert_user!("explicit rollback", UserRole::Member))
        .execute()?;
    tx.rollback()?;
    assert_eq!(
        db.select(count(users.id)).from(users).get::<i64, _, _>()?,
        2
    );

    drop(db.into_inner());
    let returned = pool
        .get_conn()
        .map_err(|error| DrizzleError::driver("MySQL", error))?;
    drop(returned);
    Ok(())
}
