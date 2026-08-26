use drizzle_migrations::mysql::{
    CheckConstraint, Column, ForeignKey, Generated, Index, IndexAlgorithm, IndexColumn, IndexLock,
    IndexMethod, InlineEnum, InlineType, MySQLEntity, MySQLSnapshot, PrimaryKey, ReferentialAction,
    Table, UniqueConstraint,
};
use drizzle_migrations::{DiffOptions, Snapshot, diff, diff_with};
use drizzle_types::Dialect;

fn in_database(entity: &mut MySQLEntity, database: &'static str) {
    match entity {
        MySQLEntity::Table(value) => value.database = Some(database.into()),
        MySQLEntity::Column(value) => value.database = Some(database.into()),
        MySQLEntity::Index(value) => value.database = Some(database.into()),
        MySQLEntity::PrimaryKey(value) => value.database = Some(database.into()),
        MySQLEntity::UniqueConstraint(value) => value.database = Some(database.into()),
        MySQLEntity::ForeignKey(value) => {
            value.database = Some(database.into());
            value.foreign_database = Some(database.into());
        }
        MySQLEntity::CheckConstraint(value) => value.database = Some(database.into()),
        MySQLEntity::View(value) => value.database = Some(database.into()),
    }
}

fn mysql_snapshot(entities: Vec<MySQLEntity>) -> Snapshot {
    let mut snapshot = MySQLSnapshot::new();
    for mut entity in entities {
        in_database(&mut entity, "app`db");
        snapshot.add_entity(entity);
    }
    Snapshot::MySQL(snapshot)
}

fn comprehensive_schema() -> Snapshot {
    let mut accounts = Table::new("accounts");
    accounts.engine = Some("InnoDB".into());
    accounts.charset = Some("utf8mb4".into());
    accounts.collation = Some("utf8mb4_0900_ai_ci".into());

    let mut account_id = Column::new("accounts", "id", "bigint unsigned");
    account_id.not_null = true;
    account_id.autoincrement = true;

    let mut jobs = Table::new("job`s");
    jobs.engine = Some("InnoDB".into());
    jobs.charset = Some("utf8mb4".into());
    jobs.collation = Some("utf8mb4_0900_ai_ci".into());

    let mut job_id = Column::new("job`s", "id", "bigint unsigned");
    job_id.not_null = true;
    job_id.autoincrement = true;
    let mut owner_id = Column::new("job`s", "owner_id", "bigint unsigned");
    owner_id.not_null = true;
    let mut status = Column::new("job`s", "status", "enum");
    status.not_null = true;
    status.inline_type = Some(InlineType::Enum(InlineEnum::new([
        "queued",
        "in,flight",
        "done",
    ])));
    status.default = Some("'queued'".into());
    let mut slug_source = Column::new("job`s", "slug_source", "varchar(255)");
    slug_source.not_null = true;
    let mut slug = Column::new("job`s", "slug", "varchar(255)");
    slug.generated = Some(Generated::stored("concat(slug_source, '-job')"));
    slug.charset = Some("utf8mb4".into());
    slug.collation = Some("utf8mb4_bin".into());

    let mut owner_index = Index::new(
        "job`s",
        "job`s_owner_idx",
        vec![IndexColumn::column("owner_id")],
    );
    owner_index.using = Some(IndexMethod::Btree);
    owner_index.algorithm = Some(IndexAlgorithm::Inplace);
    owner_index.lock = Some(IndexLock::None);

    let mut owner_fk = ForeignKey::new("job`s", "job`s_owner_fk", ["owner_id"], "accounts", ["id"]);
    owner_fk.on_delete = Some(ReferentialAction::Cascade);
    owner_fk.on_update = Some(ReferentialAction::Restrict);

    mysql_snapshot(vec![
        MySQLEntity::Table(accounts),
        MySQLEntity::Column(account_id),
        MySQLEntity::PrimaryKey(PrimaryKey::new("accounts", ["id"])),
        MySQLEntity::Table(jobs),
        MySQLEntity::Column(job_id),
        MySQLEntity::Column(owner_id),
        MySQLEntity::Column(status),
        MySQLEntity::Column(slug_source),
        MySQLEntity::Column(slug),
        MySQLEntity::PrimaryKey(PrimaryKey::new("job`s", ["id"])),
        MySQLEntity::UniqueConstraint(UniqueConstraint::new(
            "job`s",
            "job`s_slug_unique",
            ["slug"],
        )),
        MySQLEntity::Index(owner_index),
        MySQLEntity::ForeignKey(owner_fk),
        MySQLEntity::CheckConstraint(CheckConstraint::new(
            "job`s",
            "job`s_owner_positive",
            "owner_id > 0",
        )),
    ])
}

#[test]
fn mysql_snapshot_round_trip_preserves_rich_ddl() {
    let Snapshot::MySQL(snapshot) = comprehensive_schema() else {
        unreachable!()
    };
    let json = snapshot.to_json().unwrap();
    let round_trip = MySQLSnapshot::from_json(&json).unwrap();
    assert_eq!(round_trip.ddl, snapshot.ddl);
}

#[test]
fn shared_diff_entrypoint_generates_mysql_sql_without_transaction_claims() {
    let plan = diff(&Snapshot::empty(Dialect::MySQL), &comprehensive_schema()).unwrap();
    assert!(plan.warnings.is_empty());
    assert!(plan.statements.first().unwrap().starts_with("CREATE TABLE"));
    assert!(
        plan.to_sql()
            .contains("CREATE INDEX `job``s_owner_idx` USING BTREE")
    );
    assert!(plan.to_sql().contains("ALGORITHM=INPLACE LOCK=NONE"));
    assert!(
        plan.to_sql()
            .contains("enum('queued', 'in,flight', 'done')")
    );
    assert!(
        plan.to_sql()
            .contains("ON DELETE CASCADE ON UPDATE RESTRICT")
    );
    assert!(!plan.to_sql().contains("BEGIN"));
    assert!(!plan.to_sql().contains("COMMIT"));
}

#[test]
fn shared_rename_hints_are_database_scoped_and_non_destructive() {
    let previous = mysql_snapshot(vec![
        MySQLEntity::Table(Table::new("users_old")),
        MySQLEntity::Column(Column::new("users_old", "display_name", "varchar(80)")),
    ]);
    let current = mysql_snapshot(vec![
        MySQLEntity::Table(Table::new("users")),
        MySQLEntity::Column(Column::new("users", "name", "varchar(80)")),
    ]);
    let options = DiffOptions::new()
        .rename_table_in("app`db", "users_old", "users")
        .rename_column_in("app`db", "users", "display_name", "name")
        .strict_renames(true);
    let plan = diff_with(&previous, &current, &options).unwrap();
    assert_eq!(
        plan.statements,
        [
            "RENAME TABLE `app``db`.`users_old` TO `app``db`.`users`;",
            "ALTER TABLE `app``db`.`users` RENAME COLUMN `display_name` TO `name`;",
        ]
    );
    assert!(plan.warnings.is_empty());
}

#[test]
fn destructive_changes_surface_structural_warnings() {
    let previous = comprehensive_schema();
    let mut accounts = Table::new("accounts");
    accounts.engine = Some("InnoDB".into());
    accounts.charset = Some("utf8mb4".into());
    accounts.collation = Some("utf8mb4_0900_ai_ci".into());
    let current = mysql_snapshot(vec![
        MySQLEntity::Table(accounts),
        MySQLEntity::Column(Column::new("accounts", "display_name", "varchar(255)")),
    ]);
    let plan = diff(&previous, &current).unwrap();
    assert!(
        plan.warnings
            .iter()
            .any(|warning| warning.contains("dropping table"))
    );
    assert!(
        plan.warnings
            .iter()
            .any(|warning| warning.contains("dropping column"))
    );
}

#[test]
fn build_generation_preserves_mysql_schema_semantics() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("schema.rs");
    let output = temp.path().join("drizzle");
    std::fs::write(
        &source,
        r#"
#[derive(MySQLEnum)]
enum State { Draft, Published }

#[MySQLTable(DATABASE = "app", NAME = "documents", ENGINE = "InnoDB", CHARSET = "utf8mb4")]
struct Documents {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(ENUM)]
    state: State,
    #[column(generated(STORED, "sequence + 1"))]
    next_sequence: u32,
    sequence: u32,
}

#[MySQLIndex(unique, using = "BTREE", algorithm = "INPLACE", lock = "NONE")]
struct DocumentsState(Documents::state);

#[derive(MySQLSchema)]
struct AppSchema {
    documents: Documents,
    documents_state: DocumentsState,
}
"#,
    )
    .unwrap();

    let config = drizzle_migrations::build::Config::new(Dialect::MySQL)
        .file(&source)
        .out(&output)
        .name("mysql_schema");
    let generated = drizzle_migrations::build::run(&config).unwrap();
    let drizzle_migrations::build::Output::Generated { path, .. } = generated else {
        panic!("expected the initial MySQL migration")
    };
    let sql = std::fs::read_to_string(path.join("migration.sql")).unwrap();
    assert!(sql.contains("CREATE TABLE `app`.`documents`"));
    assert!(sql.contains("enum('Draft', 'Published')"));
    assert!(sql.contains("GENERATED ALWAYS AS (sequence + 1) STORED"));
    assert!(sql.contains("USING BTREE"));
    assert!(path.join("snapshot.json").is_file());
}
