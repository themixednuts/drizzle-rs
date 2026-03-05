//! Snapshot conversion helpers reused from `drizzle-migrations`.

pub use drizzle_migrations::parse_result_to_snapshot;

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_migrations::schema::Snapshot;
    use drizzle_types::{Casing, Dialect};

    /// Test that changing a column from Option<String> to String generates table recreation
    #[test]
    fn test_nullable_to_not_null_generates_migration() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::collection::SQLiteDDL;
        use drizzle_migrations::sqlite::diff::compute_migration;

        let prev_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
}
"#;

        let cur_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub name: String,
    pub email: String,
}
"#;

        let prev_result = SchemaParser::parse(prev_code);
        let cur_result = SchemaParser::parse(cur_code);

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite, None);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite, None);

        let (prev_ddl, cur_ddl) = match (&prev_snapshot, &cur_snapshot) {
            (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => (
                SQLiteDDL::from_entities(p.ddl.clone()),
                SQLiteDDL::from_entities(c.ddl.clone()),
            ),
            _ => panic!("Expected SQLite snapshots"),
        };

        let prev_email = prev_ddl
            .columns
            .one("user", "email")
            .expect("email column in prev");
        let cur_email = cur_ddl
            .columns
            .one("user", "email")
            .expect("email column in cur");
        assert!(!prev_email.not_null, "Previous email should be nullable");
        assert!(cur_email.not_null, "Current email should be NOT NULL");

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate migration SQL for nullable change"
        );

        assert_eq!(migration.sql_statements[0], "PRAGMA foreign_keys=OFF;");
        assert!(
            migration.sql_statements[1].starts_with("CREATE TABLE `__new_user`"),
            "Expected CREATE TABLE `__new_user`, got: {}",
            migration.sql_statements[1]
        );
        assert!(
            migration.sql_statements[1].contains("`email` TEXT NOT NULL"),
            "New table should have NOT NULL on email: {}",
            migration.sql_statements[1]
        );
        assert_eq!(
            migration.sql_statements[2],
            "INSERT INTO `__new_user`(`id`, `name`, `email`) SELECT `id`, `name`, `email` FROM `user`;"
        );
        assert_eq!(migration.sql_statements[3], "DROP TABLE `user`;");
        assert_eq!(
            migration.sql_statements[4],
            "ALTER TABLE `__new_user` RENAME TO `user`;"
        );
        assert_eq!(migration.sql_statements[5], "PRAGMA foreign_keys=ON;");
    }

    /// Test that changing a column from String to Option<String> generates table recreation
    #[test]
    fn test_not_null_to_nullable_generates_migration() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::collection::SQLiteDDL;
        use drizzle_migrations::sqlite::diff::compute_migration;

        let prev_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub email: String,
}
"#;

        let cur_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub email: Option<String>,
}
"#;

        let prev_result = SchemaParser::parse(prev_code);
        let cur_result = SchemaParser::parse(cur_code);

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite, None);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite, None);

        let (prev_ddl, cur_ddl) = match (&prev_snapshot, &cur_snapshot) {
            (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => (
                SQLiteDDL::from_entities(p.ddl.clone()),
                SQLiteDDL::from_entities(c.ddl.clone()),
            ),
            _ => panic!("Expected SQLite snapshots"),
        };

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate migration SQL for nullable change"
        );

        assert_eq!(migration.sql_statements[0], "PRAGMA foreign_keys=OFF;");
        assert!(
            migration.sql_statements[1].starts_with("CREATE TABLE `__new_user`"),
            "Expected CREATE TABLE `__new_user`, got: {}",
            migration.sql_statements[1]
        );
        assert_eq!(migration.sql_statements[3], "DROP TABLE `user`;");
        assert_eq!(
            migration.sql_statements[4],
            "ALTER TABLE `__new_user` RENAME TO `user`;"
        );
        assert_eq!(migration.sql_statements[5], "PRAGMA foreign_keys=ON;");
    }

    #[test]
    fn test_postgres_schema_and_index_options_are_preserved() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::postgres::ddl::PostgresEntity;

        let code = r#"
#[PostgresTable(schema = "auth")]
pub struct Users {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable(schema = "app")]
pub struct Sessions {
    #[column(primary)]
    pub id: i32,
    #[column(references = Users::id)]
    pub user_id: i32,
}

#[PostgresIndex(concurrent, method = "gin", where = "user_id > 0")]
pub struct SessionsUserIdx(Sessions::user_id);
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::PostgreSQL, None);

        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            _ => panic!("Expected Postgres snapshot"),
        };

        let has_auth_schema = snap
            .ddl
            .iter()
            .any(|e| matches!(e, PostgresEntity::Schema(s) if s.name.as_ref() == "auth"));
        let has_app_schema = snap
            .ddl
            .iter()
            .any(|e| matches!(e, PostgresEntity::Schema(s) if s.name.as_ref() == "app"));
        assert!(has_auth_schema, "missing auth schema entity");
        assert!(has_app_schema, "missing app schema entity");

        let fk = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::ForeignKey(fk) = e {
                Some(fk)
            } else {
                None
            }
        });
        let fk = fk.expect("expected foreign key");
        assert_eq!(fk.schema.as_ref(), "app");
        assert_eq!(fk.schema_to.as_ref(), "auth");

        let idx = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let idx = idx.expect("expected index");
        assert!(idx.concurrently);
        assert_eq!(idx.method.as_deref(), Some("gin"));
        assert_eq!(idx.where_clause.as_deref(), Some("user_id > 0"));
        assert_eq!(idx.schema.as_ref(), "app");
    }

    #[test]
    fn test_sqlite_table_options_and_pk_name_are_preserved() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable(strict, without_rowid)]
pub struct Accounts {
    #[column(primary)]
    pub id: i64,
}
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::SQLite, None);
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            _ => panic!("Expected SQLite snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected sqlite table");
        assert!(table.strict, "strict should be preserved");
        assert!(table.without_rowid, "without_rowid should be preserved");

        let pk = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::PrimaryKey(pk) = e {
                Some(pk)
            } else {
                None
            }
        });
        let pk = pk.expect("expected sqlite primary key");
        assert_eq!(pk.name.as_ref(), "accounts_pkey");
    }

    #[test]
    fn test_sqlite_casing_preserves_explicit_names() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable(name = "users_tbl")]
pub struct UsersTable {
    #[column(name = "user_id", primary)]
    pub userId: i64,
    pub emailAddress: String,
}

#[SQLiteIndex(name = "users_tbl_email_idx")]
pub struct UsersEmailIdx(UsersTable::emailAddress);
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::SQLite, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            _ => panic!("Expected SQLite snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected sqlite table");
        assert_eq!(table.name.as_ref(), "users_tbl");

        let user_id = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Column(c) = e
                && c.name.as_ref() == "user_id"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(user_id.is_some(), "expected explicit column name user_id");

        let email_col = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Column(c) = e
                && c.name.as_ref() == "email_address"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(
            email_col.is_some(),
            "expected inferred snake_case column name"
        );

        let index = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let index = index.expect("expected sqlite index");
        assert_eq!(index.name.as_ref(), "users_tbl_email_idx");
    }

    #[test]
    fn test_postgres_casing_preserves_explicit_names() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::postgres::ddl::PostgresEntity;

        let code = r#"
#[PostgresTable(schema = "auth", name = "users_tbl")]
pub struct UsersTable {
    #[column(name = "user_id", primary)]
    pub userId: i32,
    pub createdAt: String,
}

#[PostgresIndex(name = "users_tbl_created_idx")]
pub struct UsersCreatedIdx(UsersTable::createdAt);
"#;

        let result = SchemaParser::parse(code);
        let snapshot =
            parse_result_to_snapshot(&result, Dialect::PostgreSQL, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            _ => panic!("Expected Postgres snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected postgres table");
        assert_eq!(table.schema.as_ref(), "auth");
        assert_eq!(table.name.as_ref(), "users_tbl");

        let user_id = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Column(c) = e
                && c.name.as_ref() == "user_id"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(user_id.is_some(), "expected explicit column name user_id");

        let created_at = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Column(c) = e
                && c.name.as_ref() == "created_at"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(
            created_at.is_some(),
            "expected inferred snake_case column name created_at"
        );

        let index = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let index = index.expect("expected postgres index");
        assert_eq!(index.name.as_ref(), "users_tbl_created_idx");
        assert_eq!(index.schema.as_ref(), "auth");
    }
}
