//! MySQL migration generation and runtime coverage.

#[cfg(feature = "mysql-sync")]
use crate::common::helpers::mysql_sync_setup;
use drizzle::migrations::{Migration, Snapshot, Tracking};
#[cfg(feature = "mysql-sync")]
use drizzle::{
    Dialect,
    migrations::{DiffOptions, Schema as MigrationSchema, diff, diff_with},
};
use drizzle::{
    core::{SQL, expr::eq},
    mysql::prelude::*,
};
#[cfg(feature = "mysql-sync")]
use mysql::prelude::Queryable as _;

#[MySQLTable(NAME = "mysql_migration_harness")]
struct MigrationHarness {
    #[column(PRIMARY)]
    id: i32,
}

#[MySQLTable(NAME = "mysql_numeric_ddl")]
struct NumericDdl {
    #[column(PRIMARY)]
    id: i32,
    #[column(DECIMAL_UNSIGNED(20, 8))]
    amount: String,
    #[column(FLOAT_UNSIGNED(10, 2))]
    ratio: f32,
    #[column(DOUBLE_UNSIGNED(10, 2))]
    estimate: f64,
    #[column(REAL(10, 2))]
    measurement: f64,
    #[column(REAL_UNSIGNED(10, 2))]
    unsigned_measurement: f64,
}

#[MySQLView(
    NAME = "mysql_migration_harness_view",
    DEFINITION = "SELECT id FROM mysql_migration_harness"
)]
struct MigrationHarnessView {
    id: i32,
}

#[derive(MySQLSchema)]
struct RuntimeMigrationSchema {
    harness: MigrationHarness,
    harness_view: MigrationHarnessView,
}

#[drizzle::test]
fn numeric_ddl_is_accepted_and_introspected(db: &mut TestDb<RuntimeMigrationSchema>) {
    result!(db.execute(SQL::raw("DROP TABLE IF EXISTS mysql_numeric_ddl")))
        .expect("clean up the numeric DDL fixture");

    let create_sql = NumericDdl::create_table_sql();
    result!(db.execute(SQL::raw(&create_sql))).expect("MySQL accepts generated numeric DDL");

    let snapshot = result!(db.introspect()).expect("introspect generated numeric DDL");
    let Snapshot::MySQL(snapshot) = snapshot else {
        panic!("MySQL introspection returned another dialect");
    };
    let ddl = drizzle::migrations::mysql::MySQLDDL::try_from_entities(snapshot.ddl)
        .expect("introspection returns valid MySQL numeric DDL");
    let column_type = |name| {
        ddl.columns
            .one(None, "mysql_numeric_ddl", name)
            .unwrap_or_else(|| panic!("introspection omitted mysql_numeric_ddl.{name}"))
            .sql_type
            .to_ascii_lowercase()
    };
    assert_eq!(column_type("amount"), "decimal(20,8) unsigned");
    assert_eq!(column_type("ratio"), "float(10,2) unsigned");
    assert_eq!(column_type("estimate"), "double(10,2) unsigned");
    assert!(matches!(
        column_type("measurement").as_str(),
        "real(10,2)" | "double(10,2)"
    ));
    assert!(matches!(
        column_type("unsigned_measurement").as_str(),
        "real(10,2) unsigned" | "double(10,2) unsigned"
    ));

    result!(db.execute(SQL::raw("DROP TABLE mysql_numeric_ddl")))
        .expect("clean up the numeric DDL fixture");
}

#[drizzle::test]
fn introspect_and_push_work_across_connection_adapters(db: &mut TestDb<RuntimeMigrationSchema>) {
    let RuntimeMigrationSchema { harness, .. } = schema;
    for statement in [
        "DROP VIEW IF EXISTS mysql_migration_harness_view",
        "DROP TABLE IF EXISTS mysql_migration_harness",
        "DROP TABLE IF EXISTS mysql_migration_harness_view",
        "DROP VIEW IF EXISTS mysql_push_unmanaged_view",
        "DROP TABLE IF EXISTS mysql_push_unmanaged",
        "CREATE TABLE mysql_push_unmanaged (id INT NOT NULL PRIMARY KEY)",
        "CREATE VIEW mysql_push_unmanaged_view AS SELECT id FROM mysql_push_unmanaged",
        "CREATE TABLE mysql_migration_harness_view (id INT NOT NULL PRIMARY KEY)",
    ] {
        result!(db.execute(SQL::raw(statement))).expect("prepare MySQL push test schema");
    }

    result!(db.push(&schema)).expect_err("the unmanaged table blocks the desired view");
    result!(db.execute(SQL::raw(
        "INSERT INTO mysql_migration_harness (id) VALUES (5)"
    )))
    .expect("MySQL keeps DDL applied before a later push statement fails");
    result!(db.execute(SQL::raw("DROP TABLE mysql_migration_harness_view")))
        .expect("remove the unmanaged view-name blocker");
    result!(db.push(&schema)).expect("push the MySQL schema");
    let snapshot = result!(db.introspect()).expect("introspect the pushed MySQL schema");
    let Snapshot::MySQL(snapshot) = snapshot else {
        panic!("MySQL introspection returned another dialect");
    };
    let ddl = drizzle::migrations::mysql::MySQLDDL::try_from_entities(snapshot.ddl)
        .expect("introspection returns a valid MySQL snapshot");
    assert!(ddl.tables.one(None, "mysql_migration_harness").is_some());
    assert!(
        ddl.views
            .one(None, "mysql_migration_harness_view")
            .is_some()
    );

    result!(db.push(&schema)).expect("repeated MySQL push is a no-op");
    result!(db.execute(SQL::raw(
        "INSERT INTO mysql_push_unmanaged (id) VALUES (11)"
    )))
    .expect("push preserves unmanaged tables");
    let unmanaged: i32 = result!(db.get(SQL::raw(
        "SELECT id FROM mysql_push_unmanaged_view WHERE id = 11"
    )))
    .expect("push preserves unmanaged views");
    assert_eq!(unmanaged, 11);

    db.insert(harness)
        .value(InsertMigrationHarness::new(7))
        .execute();
    let selected: SelectMigrationHarness =
        db.select(()).from(harness).r#where(eq(harness.id, 7)).get();
    assert_eq!(selected.id, 7);

    for statement in [
        "DROP VIEW mysql_migration_harness_view",
        "DROP VIEW mysql_push_unmanaged_view",
        "DROP TABLE mysql_push_unmanaged",
    ] {
        result!(db.execute(SQL::raw(statement))).expect("clean up unmanaged MySQL push fixture");
    }
}

#[cfg(feature = "mysql-sync")]
mod v1 {
    use drizzle::mysql::prelude::*;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, MySQLEnum)]
    pub enum Status {
        #[default]
        Queued,
        Done,
    }

    #[MySQLTable(NAME = "migration_accounts", ENGINE = "InnoDB", CHARSET = "utf8mb4")]
    pub struct Account {
        #[column(PRIMARY, AUTO_INCREMENT)]
        pub id: u64,
    }

    #[MySQLTable(NAME = "migration_jobs", ENGINE = "InnoDB", CHARSET = "utf8mb4")]
    pub struct Job {
        #[column(PRIMARY, AUTO_INCREMENT)]
        pub id: u64,
        #[column(REFERENCES = Account::id, ON_DELETE = CASCADE)]
        pub account_id: u64,
        pub attempts: u64,
        #[column(ENUM)]
        pub state: Status,
    }

    #[derive(MySQLSchema)]
    pub struct Schema {
        pub accounts: Account,
        pub jobs: Job,
        pub active_jobs: ActiveJobs,
    }

    #[MySQLView(
        NAME = "migration_active_jobs",
        DEFINITION = "SELECT id, account_id FROM migration_jobs WHERE account_id > 0",
        ALGORITHM = "MERGE",
        SQL_SECURITY = "INVOKER",
        WITH_CHECK_OPTION
    )]
    pub struct ActiveJobs {
        pub id: u64,
        pub account_id: u64,
    }
}

#[cfg(feature = "mysql-sync")]
mod v2 {
    use drizzle::mysql::prelude::*;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, MySQLEnum)]
    pub enum Status {
        #[default]
        Queued,
        Done,
        Archived,
    }

    #[MySQLTable(NAME = "migration_accounts", ENGINE = "InnoDB", CHARSET = "utf8mb4")]
    pub struct Account {
        #[column(PRIMARY, AUTO_INCREMENT)]
        pub id: u64,
    }

    #[MySQLTable(NAME = "migration_jobs", ENGINE = "InnoDB", CHARSET = "utf8mb4")]
    pub struct Job {
        #[column(PRIMARY, AUTO_INCREMENT)]
        pub id: u64,
        #[column(REFERENCES = Account::id, ON_DELETE = CASCADE)]
        pub account_id: u64,
        pub attempts: u64,
        #[column(ENUM)]
        pub status: Status,
        #[column(generated(STORED, "attempts + 1"))]
        pub account_key: u64,
    }

    #[MySQLIndex(using = "BTREE", algorithm = "INPLACE", lock = "NONE")]
    pub struct JobAccountIndex(Job::account_id);

    #[derive(MySQLSchema)]
    pub struct Schema {
        pub accounts: Account,
        pub jobs: Job,
        pub jobs_account_index: JobAccountIndex,
        pub active_jobs: ActiveJobs,
    }

    #[MySQLView(
        NAME = "migration_active_jobs",
        DEFINITION = "SELECT id, account_id FROM migration_jobs WHERE account_id > 0",
        ALGORITHM = "MERGE",
        SQL_SECURITY = "INVOKER",
        WITH_CHECK_OPTION
    )]
    pub struct ActiveJobs {
        pub id: u64,
        pub account_id: u64,
    }
}

#[cfg(feature = "mysql-sync")]
fn clean(connection: &mut impl mysql::prelude::Queryable) {
    connection
        .query_drop("SET FOREIGN_KEY_CHECKS = 0")
        .expect("disable foreign-key checks for migration cleanup");
    connection
        .query_drop("DROP VIEW IF EXISTS `migration_active_jobs`")
        .expect("drop migration active-jobs view");
    connection
        .query_drop("DROP TABLE IF EXISTS `migration_jobs`")
        .expect("drop migration jobs table");
    connection
        .query_drop("DROP TABLE IF EXISTS `migration_accounts`")
        .expect("drop migration accounts table");
    connection
        .query_drop("SET FOREIGN_KEY_CHECKS = 1")
        .expect("restore foreign-key checks after migration cleanup");
}

#[cfg(feature = "mysql-sync")]
fn apply(connection: &mut impl mysql::prelude::Queryable, statements: &[String]) {
    for statement in statements {
        connection.query_drop(statement).unwrap_or_else(|error| {
            panic!("MySQL rejected generated migration SQL `{statement}`: {error}")
        });
    }
}

#[cfg(feature = "mysql-sync")]
#[test]
fn generated_create_and_alter_sql_runs_on_supported_mysql() {
    let _guard = mysql_sync_setup::acquire_lock();
    let mut connection = mysql::Conn::new(mysql_sync_setup::options())
        .expect("connect to the MySQL integration-test database");
    clean(&mut connection);

    let previous = v1::Schema::new().to_snapshot();
    let create = diff(&Snapshot::empty(Dialect::MySQL), &previous)
        .expect("generate the initial MySQL migration");
    apply(&mut connection, &create.statements);

    let current = v2::Schema::new().to_snapshot();
    let options = DiffOptions::new()
        .rename_column("migration_jobs", "state", "status")
        .strict_renames(true);
    let alter = diff_with(&previous, &current, &options).expect("generate the MySQL alteration");
    apply(&mut connection, &alter.statements);

    let (_, create_sql): (String, String) = connection
        .query_first("SHOW CREATE TABLE `migration_jobs`")
        .expect("inspect migrated table")
        .expect("migration_jobs exists");
    assert!(create_sql.contains("`status` enum('Queued','Done','Archived')"));
    assert!(create_sql.contains("`account_key` bigint unsigned GENERATED ALWAYS AS"));
    assert!(create_sql.contains("`job_account_index` (`account_id`)"));
    assert!(create_sql.contains("FOREIGN KEY (`account_id`)"));

    let view: Option<(String, String, String)> = connection
        .exec_first(
            "SELECT VIEW_DEFINITION, SECURITY_TYPE, CHECK_OPTION \
             FROM information_schema.VIEWS \
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?",
            ("migration_active_jobs",),
        )
        .expect("inspect migrated view");
    let (definition, security, check_option) = view.expect("migration_active_jobs exists");
    assert!(definition.contains("migration_jobs"));
    assert_eq!(security, "INVOKER");
    assert_eq!(check_option, "CASCADED");

    let empty = Snapshot::empty(Dialect::MySQL);
    let drop_plan = diff(&current, &empty).expect("generate the MySQL teardown migration");
    apply(&mut connection, &drop_plan.statements);
    let remaining: Option<String> = connection
        .query_first("SHOW TABLES LIKE 'migration_jobs'")
        .expect("inspect teardown result");
    assert!(remaining.is_none());

    clean(&mut connection);
}

#[drizzle::test]
fn runtime_migrations_apply_in_order_and_only_once(db: &mut TestDb<RuntimeMigrationSchema>) {
    let tracking = Tracking::MYSQL.table("__drizzle_runtime_order");
    for table in [
        "mysql_runtime_second",
        "mysql_runtime_first",
        "__drizzle_runtime_order",
    ] {
        result!(db.execute(SQL::raw(format!("DROP TABLE IF EXISTS `{table}`"))))
            .expect("clear prior migration state");
    }

    let created_at = 1_787_788_800_000;
    let migrations = [
        Migration::with_hash(
            "20260827000000_mysql_runtime_first",
            "mysql-runtime-first",
            created_at,
            vec![
                "CREATE TABLE `mysql_runtime_first` (`id` INT PRIMARY KEY)".to_owned(),
                "INSERT INTO `mysql_runtime_first` (`id`) VALUES (1)".to_owned(),
            ],
        ),
        Migration::with_hash(
            "20260827000000_mysql_runtime_second",
            "mysql-runtime-second",
            created_at,
            vec![
                "CREATE TABLE `mysql_runtime_second` (`id` INT PRIMARY KEY)".to_owned(),
                "INSERT INTO `mysql_runtime_second` (`id`) SELECT `id` + 1 FROM `mysql_runtime_first`"
                    .to_owned(),
            ],
        ),
    ];

    let outcome =
        result!(db.migrate(&migrations, tracking.clone())).expect("apply MySQL runtime migrations");
    assert_eq!(
        outcome.applied_tags(),
        [
            "20260827000000_mysql_runtime_first",
            "20260827000000_mysql_runtime_second",
        ]
    );
    let copied: i64 = result!(db.get(SQL::raw("SELECT `id` FROM `mysql_runtime_second`")))
        .expect("read the second migration's effect");
    assert_eq!(copied, 2);

    let outcome = result!(db.migrate(&migrations, tracking.clone()))
        .expect("recheck MySQL runtime migrations");
    assert!(outcome.is_up_to_date());

    for table in [
        "mysql_runtime_second",
        "mysql_runtime_first",
        "__drizzle_runtime_order",
    ] {
        result!(db.execute(SQL::raw(format!("DROP TABLE IF EXISTS `{table}`"))))
            .expect("clean migration state");
    }
}

#[drizzle::test]
fn runtime_migrations_keep_the_first_failure_dirty(db: &mut TestDb<RuntimeMigrationSchema>) {
    let tracking = Tracking::MYSQL.table("__drizzle_runtime_dirty");
    result!(db.execute(SQL::raw("DROP TABLE IF EXISTS `__drizzle_runtime_dirty`")))
        .expect("clear prior dirty state");

    let migration = Migration::new(
        "20260827000001_mysql_runtime_dirty",
        "THIS IS NOT VALID MYSQL SQL",
    );
    let error = result!(db.migrate(std::slice::from_ref(&migration), tracking.clone()))
        .expect_err("the invalid migration must fail");
    assert!(error.to_string().contains("dirty marker"), "{error}");

    let dirty: i64 = result!(db.get(SQL::raw(
        "SELECT COUNT(*) FROM `__drizzle_runtime_dirty` WHERE `applied_at` IS NULL"
    )))
    .expect("count dirty migrations");
    assert_eq!(dirty, 1);

    let error = result!(db.migrate(std::slice::from_ref(&migration), tracking.clone()))
        .expect_err("an interrupted migration must block retries");
    let message = error.to_string();
    assert!(message.contains("20260827000001_mysql_runtime_dirty"));
    assert!(message.contains("interrupted mid-apply"), "{message}");

    result!(db.execute(SQL::raw("DROP TABLE IF EXISTS `__drizzle_runtime_dirty`")))
        .expect("clean dirty migration state");
}

#[drizzle::test]
fn runtime_migrations_reject_disabled_autocommit_before_writing(
    db: &mut TestDb<RuntimeMigrationSchema>,
) {
    let tracking = Tracking::MYSQL.table("__drizzle_runtime_transaction");
    result!(db.execute(SQL::raw(
        "DROP TABLE IF EXISTS `__drizzle_runtime_transaction`"
    )))
    .expect("clear prior tracking state");
    result!(db.execute(SQL::raw("SET autocommit = 0"))).expect("disable autocommit");

    let migration = Migration::new("20260827000002_mysql_runtime_transaction", "SELECT 1");
    let error = result!(db.migrate(std::slice::from_ref(&migration), tracking))
        .expect_err("migrate must reject disabled autocommit");
    assert!(error.to_string().contains("autocommit disabled"), "{error}");

    result!(db.execute(SQL::raw("SET autocommit = 1"))).expect("restore autocommit");
    let tracking_tables: i64 = result!(db.get(SQL::raw(
        "SELECT COUNT(*) FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_name = '__drizzle_runtime_transaction'"
    )))
    .expect("check tracking-table absence");
    assert_eq!(tracking_tables, 0, "preflight must not mutate the database");
}
