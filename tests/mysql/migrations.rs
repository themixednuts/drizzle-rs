//! MySQL-specific live migration SQL coverage.

use crate::common::helpers::mysql_sync_setup;
use drizzle::Dialect;
use drizzle::migrations::{DiffOptions, Schema as MigrationSchema, Snapshot, diff, diff_with};
use mysql::prelude::Queryable as _;

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

fn apply(connection: &mut impl mysql::prelude::Queryable, statements: &[String]) {
    for statement in statements {
        connection.query_drop(statement).unwrap_or_else(|error| {
            panic!("MySQL rejected generated migration SQL `{statement}`: {error}")
        });
    }
}

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
