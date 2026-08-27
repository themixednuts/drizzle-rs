use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt as _;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

pub trait DialectCase {
    const DIALECT: &'static str;
    const DRIVER: &'static str;

    fn database_url(root: &Path) -> String;
    fn render_table(rust_name: &str, sql_name: &str, fields: &str) -> String;
    fn quoted(name: &str) -> String;
    fn id_type() -> &'static str;
}

pub trait LiveDriverCase: DialectCase {
    fn lock_database() -> Option<MutexGuard<'static, ()>> {
        None
    }

    fn execute_batch(root: &Path, sql: &str);
    fn drop_tables(root: &Path, tables: &[&str]);
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
static POSTGRES_DATABASE_LOCK: Mutex<()> = Mutex::new(());

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
static MYSQL_DATABASE_LOCK: Mutex<()> = Mutex::new(());

struct TableCleanup<'a, B: LiveDriverCase> {
    root: &'a Path,
    tables: &'a [&'a str],
    _backend: PhantomData<B>,
}

impl<'a, B: LiveDriverCase> TableCleanup<'a, B> {
    fn new(root: &'a Path, tables: &'a [&'a str]) -> Self {
        Self {
            root,
            tables,
            _backend: PhantomData,
        }
    }
}

impl<B: LiveDriverCase> Drop for TableCleanup<'_, B> {
    fn drop(&mut self) {
        B::drop_tables(self.root, self.tables);
    }
}

#[cfg(feature = "rusqlite")]
pub struct Sqlite;

#[cfg(feature = "rusqlite")]
impl DialectCase for Sqlite {
    const DIALECT: &'static str = "sqlite";
    const DRIVER: &'static str = "rusqlite";

    fn database_url(root: &Path) -> String {
        root.join("dev.db").to_string_lossy().into_owned()
    }

    fn render_table(rust_name: &str, sql_name: &str, fields: &str) -> String {
        format!("#[SQLiteTable(name = \"{sql_name}\")]\npub struct {rust_name} {{\n{fields}\n}}\n")
    }

    fn quoted(name: &str) -> String {
        format!("`{name}`")
    }

    fn id_type() -> &'static str {
        "i64"
    }
}

#[cfg(feature = "rusqlite")]
impl LiveDriverCase for Sqlite {
    fn execute_batch(root: &Path, sql: &str) {
        rusqlite::Connection::open(Self::database_url(root))
            .expect("open sqlite parity database")
            .execute_batch(sql)
            .expect("execute sqlite parity SQL");
    }

    fn drop_tables(root: &Path, tables: &[&str]) {
        let statements = tables
            .iter()
            .map(|table| format!("DROP TABLE IF EXISTS `{table}`;"))
            .collect::<String>();
        Self::execute_batch(root, &statements);
    }
}

#[cfg(feature = "postgres-sync")]
pub struct PostgresSync;

#[cfg(feature = "postgres-sync")]
impl DialectCase for PostgresSync {
    const DIALECT: &'static str = "postgresql";
    const DRIVER: &'static str = "postgres-sync";

    fn database_url(_: &Path) -> String {
        postgres_url()
    }

    fn render_table(rust_name: &str, sql_name: &str, fields: &str) -> String {
        postgres_table(rust_name, sql_name, fields)
    }

    fn quoted(name: &str) -> String {
        format!("\"{name}\"")
    }

    fn id_type() -> &'static str {
        "i32"
    }
}

#[cfg(feature = "postgres-sync")]
impl LiveDriverCase for PostgresSync {
    fn lock_database() -> Option<MutexGuard<'static, ()>> {
        Some(
            POSTGRES_DATABASE_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    fn execute_batch(_: &Path, sql: &str) {
        postgres::Client::connect(&postgres_url(), postgres::NoTls)
            .expect("connect postgres parity database")
            .batch_execute(sql)
            .expect("execute postgres parity SQL");
    }

    fn drop_tables(root: &Path, tables: &[&str]) {
        let statements = tables
            .iter()
            .map(|table| format!("DROP TABLE IF EXISTS \"{table}\" CASCADE;"))
            .collect::<String>();
        Self::execute_batch(root, &statements);
    }
}

#[cfg(feature = "tokio-postgres")]
pub struct PostgresAsync;

#[cfg(feature = "tokio-postgres")]
impl DialectCase for PostgresAsync {
    const DIALECT: &'static str = "postgresql";
    const DRIVER: &'static str = "tokio-postgres";

    fn database_url(_: &Path) -> String {
        postgres_url()
    }

    fn render_table(rust_name: &str, sql_name: &str, fields: &str) -> String {
        postgres_table(rust_name, sql_name, fields)
    }

    fn quoted(name: &str) -> String {
        format!("\"{name}\"")
    }

    fn id_type() -> &'static str {
        "i32"
    }
}

#[cfg(feature = "tokio-postgres")]
impl LiveDriverCase for PostgresAsync {
    fn lock_database() -> Option<MutexGuard<'static, ()>> {
        Some(
            POSTGRES_DATABASE_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    fn execute_batch(_: &Path, sql: &str) {
        block_on(async {
            let (client, connection) =
                tokio_postgres::connect(&postgres_url(), tokio_postgres::NoTls)
                    .await
                    .expect("connect async postgres parity database");
            let connection = tokio::spawn(connection);
            client
                .batch_execute(sql)
                .await
                .expect("execute async postgres parity SQL");
            drop(client);
            connection
                .await
                .expect("join postgres connection task")
                .expect("drive postgres connection");
        });
    }

    fn drop_tables(root: &Path, tables: &[&str]) {
        let statements = tables
            .iter()
            .map(|table| format!("DROP TABLE IF EXISTS \"{table}\" CASCADE;"))
            .collect::<String>();
        Self::execute_batch(root, &statements);
    }
}

#[cfg(feature = "mysql-sync")]
pub struct MySqlSync;

#[cfg(feature = "mysql-sync")]
impl DialectCase for MySqlSync {
    const DIALECT: &'static str = "mysql";
    const DRIVER: &'static str = "mysql-sync";

    fn database_url(_: &Path) -> String {
        mysql_url()
    }

    fn render_table(rust_name: &str, sql_name: &str, fields: &str) -> String {
        mysql_table(rust_name, sql_name, fields)
    }

    fn quoted(name: &str) -> String {
        format!("`{name}`")
    }

    fn id_type() -> &'static str {
        "i32"
    }
}

#[cfg(feature = "mysql-sync")]
impl LiveDriverCase for MySqlSync {
    fn lock_database() -> Option<MutexGuard<'static, ()>> {
        Some(
            MYSQL_DATABASE_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    fn execute_batch(_: &Path, sql: &str) {
        use mysql::prelude::Queryable as _;

        let opts = mysql::Opts::from_url(&mysql_url()).expect("parse MySQL parity URL");
        let mut connection = mysql::Conn::new(opts).expect("connect sync MySQL parity database");
        for statement in sql.split(';').map(str::trim).filter(|sql| !sql.is_empty()) {
            connection
                .query_drop(statement)
                .expect("execute sync MySQL parity SQL");
        }
    }

    fn drop_tables(root: &Path, tables: &[&str]) {
        let statements = tables
            .iter()
            .map(|table| format!("DROP TABLE IF EXISTS `{table}`;"))
            .collect::<String>();
        Self::execute_batch(root, &statements);
    }
}

#[cfg(feature = "mysql-async")]
pub struct MySqlAsync;

#[cfg(feature = "mysql-async")]
impl DialectCase for MySqlAsync {
    const DIALECT: &'static str = "mysql";
    const DRIVER: &'static str = "mysql-async";

    fn database_url(_: &Path) -> String {
        mysql_url()
    }

    fn render_table(rust_name: &str, sql_name: &str, fields: &str) -> String {
        mysql_table(rust_name, sql_name, fields)
    }

    fn quoted(name: &str) -> String {
        format!("`{name}`")
    }

    fn id_type() -> &'static str {
        "i32"
    }
}

#[cfg(feature = "mysql-async")]
impl LiveDriverCase for MySqlAsync {
    fn lock_database() -> Option<MutexGuard<'static, ()>> {
        Some(
            MYSQL_DATABASE_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    fn execute_batch(_: &Path, sql: &str) {
        use mysql_async::prelude::Queryable as _;

        block_on(async {
            let opts = mysql_async::Opts::from_url(&mysql_url()).expect("parse MySQL parity URL");
            let pool = mysql_async::Pool::new(opts);
            let mut connection = pool
                .get_conn()
                .await
                .expect("connect async MySQL parity database");
            for statement in sql.split(';').map(str::trim).filter(|sql| !sql.is_empty()) {
                connection
                    .query_drop(statement)
                    .await
                    .expect("execute async MySQL parity SQL");
            }
            drop(connection);
            pool.disconnect()
                .await
                .expect("disconnect MySQL parity pool");
        });
    }

    fn drop_tables(root: &Path, tables: &[&str]) {
        let statements = tables
            .iter()
            .map(|table| format!("DROP TABLE IF EXISTS `{table}`;"))
            .collect::<String>();
        Self::execute_batch(root, &statements);
    }
}

pub fn generate_and_export_honor_overrides<B: DialectCase>() {
    let dir = tempdir().expect("create parity temp directory");
    let root = dir.path();
    let config = root.join("drizzle.config.toml");
    let users_schema = root.join("users.rs");
    let posts_schema = root.join("posts.rs");
    let out = root.join("generated");
    let exported = root.join("export.sql");

    fs::write(
        &users_schema,
        B::render_table(
            "Users",
            "parity_users",
            &format!(
                "    #[column(primary)]\n    pub id: {},\n    pub email: String,",
                B::id_type()
            ),
        ),
    )
    .expect("write users schema");
    fs::write(
        &posts_schema,
        B::render_table(
            "Posts",
            "parity_posts",
            &format!(
                "    #[column(primary)]\n    pub id: {},\n    pub user_id: {},",
                B::id_type(),
                B::id_type()
            ),
        ),
    )
    .expect("write posts schema");
    write_config::<B>(root, &config, root.join("missing.rs"), root.join("unused"));

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "generate",
            "--dialect",
            B::DIALECT,
            "--driver",
            B::DRIVER,
            "--schema",
            &format!(
                "{},{}",
                users_schema.to_string_lossy(),
                posts_schema.to_string_lossy()
            ),
            "--out",
            &out.to_string_lossy(),
            "--name",
            "shared_contract",
            "--breakpoints",
            "false",
        ])
        .assert()
        .success();

    let migration_sql = fs::read_to_string(first_migration_dir(&out).join("migration.sql"))
        .expect("read generated migration");
    assert!(migration_sql.contains(&B::quoted("parity_users")));
    assert!(migration_sql.contains(&B::quoted("parity_posts")));
    assert!(!migration_sql.contains("--> statement-breakpoint"));

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "export",
            "--dialect",
            B::DIALECT,
            "--schema",
            &format!(
                "{},{}",
                users_schema.to_string_lossy(),
                posts_schema.to_string_lossy()
            ),
            "--sql",
            &exported.to_string_lossy(),
        ])
        .assert()
        .success();

    let exported_sql = fs::read_to_string(exported).expect("read exported SQL");
    assert!(exported_sql.contains(&B::quoted("parity_users")));
    assert!(exported_sql.contains(&B::quoted("parity_posts")));
    assert_eq!(normalize_sql(&migration_sql), normalize_sql(&exported_sql));
}

pub fn push_honors_table_filters_and_driver<B: LiveDriverCase>() {
    let _database = B::lock_database();
    let dir = tempdir().expect("create parity temp directory");
    let root = dir.path();
    let suffix = unique_suffix();
    let wanted = format!("parity_users_live_{suffix}");
    let skipped = format!("parity_users_tmp_{suffix}");
    let configured = format!("parity_audit_{suffix}");
    let config = root.join("drizzle.config.toml");
    let schema = root.join("schema.rs");
    let tables = [&*wanted, &*skipped, &*configured];
    B::drop_tables(root, &tables);
    let _cleanup = TableCleanup::<B>::new(root, &tables);

    let source = [
        B::render_table(
            "Users",
            &wanted,
            &format!("    #[column(primary)]\n    pub id: {},", B::id_type()),
        ),
        B::render_table(
            "UsersTmp",
            &skipped,
            &format!("    #[column(primary)]\n    pub id: {},", B::id_type()),
        ),
        B::render_table(
            "Audit",
            &configured,
            &format!("    #[column(primary)]\n    pub id: {},", B::id_type()),
        ),
    ]
    .join("\n");
    fs::write(&schema, source).expect("write push schema");
    write_config::<B>(root, &config, &schema, root.join("out"));
    let configured_contents = fs::read_to_string(&config).expect("read config").replace(
        "\n[dbCredentials]",
        &format!("\ntablesFilter = [\"{configured}\"]\n\n[dbCredentials]"),
    );
    fs::write(&config, configured_contents).expect("add configured table filter");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "push",
            "--dialect",
            B::DIALECT,
            "--driver",
            B::DRIVER,
            "--schema",
            &schema.to_string_lossy(),
            "--url",
            &B::database_url(root),
            "--tablesFilter",
            &format!("parity_users_*_{suffix},!{skipped}"),
            "--verbose",
            "--force",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(&wanted))
        .stdout(predicates::str::contains(&skipped).not())
        .stdout(predicates::str::contains(&configured).not());

    B::execute_batch(
        root,
        &format!("INSERT INTO {} (id) VALUES (1);", B::quoted(&wanted),),
    );

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "push",
            "--dialect",
            B::DIALECT,
            "--driver",
            B::DRIVER,
            "--schema",
            &schema.to_string_lossy(),
            "--url",
            &B::database_url(root),
            "--tablesFilter",
            &format!("parity_users_*_{suffix},!{skipped}"),
            "--explain",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("No schema changes detected."));
}

pub fn pull_honors_filters_casing_breakpoints_and_driver<B: LiveDriverCase>() {
    let _database = B::lock_database();
    let dir = tempdir().expect("create parity temp directory");
    let root = dir.path();
    let suffix = unique_suffix();
    let included = format!("parity_audit_logs_{suffix}");
    let included_second = format!("parity_audit_meta_{suffix}");
    let skipped = format!("parity_skip_logs_{suffix}");
    let config = root.join("drizzle.config.toml");
    let out = root.join("pulled");
    let tables = [&*included, &*included_second, &*skipped];
    B::drop_tables(root, &tables);
    let _cleanup = TableCleanup::<B>::new(root, &tables);
    B::execute_batch(
        root,
        &seed_tables_sql::<B>(&included, &included_second, &skipped),
    );
    write_config::<B>(root, &config, root.join("unused.rs"), &out);

    let output = cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "pull",
            "--dialect",
            B::DIALECT,
            "--driver",
            B::DRIVER,
            "--url",
            &B::database_url(root),
            "--tablesFilter",
            &format!("parity_audit_*_{suffix}"),
            "--casing",
            "camel",
            "--breakpoints",
            "true",
        ])
        .output()
        .expect("run pull parity command");
    assert!(
        output.status.success(),
        "pull failed for {}:\nstdout:\n{}\nstderr:\n{}",
        B::DRIVER,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let schema = fs::read_to_string(out.join("schema.rs")).expect("read pulled schema");
    assert!(schema.contains("pub userName: String"));
    assert!(schema.contains(&included));
    assert!(schema.contains(&included_second));
    assert!(!schema.contains(&skipped));
    let migration = fs::read_to_string(first_migration_dir(&out).join("migration.sql"))
        .expect("read pulled migration");
    assert!(migration.contains(&B::quoted(&included)));
    assert!(migration.contains(&B::quoted(&included_second)));
    assert!(!migration.contains(&skipped));
    assert!(migration.contains("--> statement-breakpoint"));

    let generated_schema = out.join("schema.rs");
    let push_output = cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "push",
            "--dialect",
            B::DIALECT,
            "--driver",
            B::DRIVER,
            "--schema",
            &generated_schema.to_string_lossy(),
            "--url",
            &B::database_url(root),
            "--tablesFilter",
            &format!("parity_audit_*_{suffix}"),
            "--explain",
        ])
        .output()
        .expect("run pull round-trip push command");
    assert!(
        push_output.status.success(),
        "pull round-trip push failed for {}:\nstdout:\n{}\nstderr:\n{}",
        B::DRIVER,
        String::from_utf8_lossy(&push_output.stdout),
        String::from_utf8_lossy(&push_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&push_output.stdout).contains("No schema changes detected."),
        "pull round-trip push planned changes for {}:\nstdout:\n{}\nstderr:\n{}",
        B::DRIVER,
        String::from_utf8_lossy(&push_output.stdout),
        String::from_utf8_lossy(&push_output.stderr)
    );
}

pub fn migrate_applies_generated_migration_with_configured_driver<B: LiveDriverCase>() {
    let _database = B::lock_database();
    let dir = tempdir().expect("create parity temp directory");
    let root = dir.path();
    let suffix = unique_suffix();
    let table = format!("parity_migrate_{suffix}");
    let tracking_table = format!("parity_migrations_{suffix}");
    let config = root.join("drizzle.config.toml");
    let schema = root.join("schema.rs");
    let migrations = root.join("migrations");

    fs::write(
        &schema,
        "// custom migration contract does not parse this file\n",
    )
    .expect("write migration schema placeholder");
    B::drop_tables(root, &[&table, &tracking_table]);
    write_migration_config::<B>(root, &config, &schema, &migrations, &tracking_table);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "generate",
            "--custom",
            "--name",
            &format!("shared_migrate_{suffix}"),
            "--dialect",
            B::DIALECT,
            "--driver",
            B::DRIVER,
        ])
        .assert()
        .success();

    fs::write(
        first_migration_dir(&migrations).join("migration.sql"),
        format!(
            "CREATE TABLE {} (id INTEGER PRIMARY KEY);\n",
            B::quoted(&table),
        ),
    )
    .expect("write shared migration SQL");

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["--config", &config.to_string_lossy(), "migrate"])
        .assert()
        .success();

    B::execute_batch(
        root,
        &format!("INSERT INTO {} (id) VALUES (1);", B::quoted(&table)),
    );

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args(["--config", &config.to_string_lossy(), "migrate"])
        .assert()
        .success();

    B::drop_tables(root, &[&table, &tracking_table]);
}

#[cfg(any(feature = "rusqlite", feature = "mysql-sync", feature = "mysql-async",))]
pub fn non_postgres_filters_warn_and_are_ignored<B: LiveDriverCase>() {
    let _database = B::lock_database();
    let dir = tempdir().expect("create parity temp directory");
    let root = dir.path();
    let suffix = unique_suffix();
    let table = format!("parity_filter_warning_{suffix}");
    let config = root.join("drizzle.config.toml");
    let schema = root.join("schema.rs");
    fs::write(
        &schema,
        B::render_table(
            "Users",
            &table,
            &format!("    #[column(primary)]\n    pub id: {},", B::id_type()),
        ),
    )
    .expect("write filter warning schema");
    write_config::<B>(root, &config, &schema, root.join("out"));
    B::drop_tables(root, &[&table]);

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "push",
            "--driver",
            B::DRIVER,
            "--url",
            &B::database_url(root),
            "--explain",
            "--schemaFilters",
            "public",
            "--extensionsFilters",
            "postgis",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("Ignoring --schemaFilters: only supported for postgresql")
                .and(predicates::str::contains(
                    "Ignoring --extensionsFilters: only supported for postgresql",
                )),
        );

    cargo_bin_cmd!("drizzle")
        .current_dir(root)
        .args([
            "--config",
            &config.to_string_lossy(),
            "introspect",
            "--driver",
            B::DRIVER,
            "--url",
            &B::database_url(root),
            "--out",
            &root.join("introspected").to_string_lossy(),
            "--tablesFilter",
            &table,
            "--schemaFilters",
            "public",
            "--extensionsFilters",
            "postgis",
        ])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("Ignoring --schemaFilters: only supported for postgresql")
                .and(predicates::str::contains(
                    "Ignoring --extensionsFilters: only supported for postgresql",
                )),
        );
}

fn write_config<B: DialectCase>(
    root: &Path,
    path: &Path,
    schema: impl AsRef<Path>,
    out: impl AsRef<Path>,
) {
    fs::write(
        path,
        format!(
            "dialect = \"{}\"\ndriver = \"{}\"\nschema = '{}'\nout = '{}'\n\n[dbCredentials]\nurl = '{}'\n",
            B::DIALECT,
            B::DRIVER,
            schema.as_ref().to_string_lossy(),
            out.as_ref().to_string_lossy(),
            B::database_url(root),
        ),
    )
    .expect("write parity config");
}

fn write_migration_config<B: DialectCase>(
    root: &Path,
    path: &Path,
    schema: impl AsRef<Path>,
    out: impl AsRef<Path>,
    migrations_table: &str,
) {
    fs::write(
        path,
        format!(
            "dialect = \"{}\"\ndriver = \"{}\"\nschema = '{}'\nout = '{}'\n\n[migrations]\ntable = \"{migrations_table}\"\nschema = \"public\"\n\n[dbCredentials]\nurl = '{}'\n",
            B::DIALECT,
            B::DRIVER,
            schema.as_ref().to_string_lossy(),
            out.as_ref().to_string_lossy(),
            B::database_url(root),
        ),
    )
    .expect("write migration parity config");
}

fn first_migration_dir(out: &Path) -> PathBuf {
    fs::read_dir(out)
        .expect("read migration output")
        .filter_map(Result::ok)
        .find_map(|entry| {
            (entry.file_type().ok()?.is_dir() && entry.file_name().to_string_lossy() != "meta")
                .then(|| entry.path())
        })
        .expect("find migration directory")
}

fn normalize_sql(sql: &str) -> Vec<String> {
    let mut statements = sql
        .replace("--> statement-breakpoint", "")
        .split(';')
        .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|statement| !statement.is_empty())
        .collect::<Vec<_>>();
    statements.sort();
    statements
}

fn seed_tables_sql<B: DialectCase>(included: &str, included_second: &str, skipped: &str) -> String {
    format!(
        "CREATE TABLE {} (id INTEGER PRIMARY KEY, user_name VARCHAR(255) NOT NULL);\nCREATE TABLE {} (id INTEGER PRIMARY KEY, detail VARCHAR(255));\nCREATE TABLE {} (id INTEGER PRIMARY KEY, body VARCHAR(255));",
        B::quoted(included),
        B::quoted(included_second),
        B::quoted(skipped),
    )
}

fn unique_suffix() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos() as u64;
    nanos ^ u64::from(std::process::id()) ^ COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn postgres_url() -> String {
    std::env::var("DRIZZLE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/drizzle_test".into())
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn postgres_table(rust_name: &str, sql_name: &str, fields: &str) -> String {
    format!(
        "#[PostgresTable(name = \"{sql_name}\", schema = \"public\")]\npub struct {rust_name} {{\n{fields}\n}}\n"
    )
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn mysql_url() -> String {
    std::env::var("DRIZZLE_MYSQL_URL")
        .unwrap_or_else(|_| "mysql://drizzle:drizzle@127.0.0.1:3307/drizzle_test".into())
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn mysql_table(rust_name: &str, sql_name: &str, fields: &str) -> String {
    format!("#[MySQLTable(NAME = \"{sql_name}\")]\npub struct {rust_name} {{\n{fields}\n}}\n")
}

#[cfg(any(feature = "tokio-postgres", feature = "mysql-async"))]
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build parity Tokio runtime")
        .block_on(future)
}

#[macro_export]
macro_rules! shared_dialect_contract {
    ($backend:ty) => {
        #[test]
        fn generate_and_export_honor_overrides() {
            $crate::parity::generate_and_export_honor_overrides::<$backend>();
        }
    };
}

#[macro_export]
macro_rules! shared_live_driver_contract {
    ($backend:ty) => {
        #[test]
        fn push_honors_table_filters_and_driver() {
            $crate::parity::push_honors_table_filters_and_driver::<$backend>();
        }

        #[test]
        fn pull_honors_filters_casing_breakpoints_and_driver() {
            $crate::parity::pull_honors_filters_casing_breakpoints_and_driver::<$backend>();
        }

        #[test]
        fn migrate_applies_generated_migration_with_configured_driver() {
            $crate::parity::migrate_applies_generated_migration_with_configured_driver::<$backend>(
            );
        }
    };
}

#[macro_export]
macro_rules! shared_non_postgres_contract {
    ($backend:ty) => {
        #[test]
        fn postgres_only_filters_warn_and_are_ignored() {
            $crate::parity::non_postgres_filters_warn_and_are_ignored::<$backend>();
        }
    };
}
