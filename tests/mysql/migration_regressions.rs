//! MySQL migration regressions that only show on a real server: generated
//! DDL must be accepted, and a database created by `db.create()`, by a
//! generated migration or by `push` must need no further push.

use drizzle::core::SQL;
use drizzle::migrations::mysql::MySQLCatalogDefaults;
use drizzle::migrations::{DiffOptions, Schema as MigrationSchema, Snapshot, diff_with};
use drizzle::mysql::prelude::*;

#[MySQLTable(NAME = "mysql_regress_parents")]
pub struct RegressParents {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
}

#[MySQLTable(NAME = "mysql_regress_defaults")]
pub struct RegressDefaults {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(TEXT, DEFAULT = "hello")]
    pub body: String,
    #[column(JSON, DEFAULT = "[]")]
    pub payload: String,
    #[column(VARCHAR(36), DEFAULT = UUID())]
    pub token: String,
    #[column(BLOB, DEFAULT = b"ab")]
    pub bytes: Vec<u8>,
    #[column(DEFAULT = -1)]
    pub negative: i32,
    #[column(DEFAULT = true)]
    pub flag: bool,
    #[column(DEFAULT = 1.5)]
    pub ratio: f64,
    #[column(VARCHAR(20), DEFAULT = "it's")]
    pub quoted: String,
    #[column(VARCHAR(20), DEFAULT = "back\\slash")]
    pub backslash: String,
    #[column(VARCHAR(20), DEFAULT = "null")]
    pub null_text: String,
    #[column(VARCHAR(20), DEFAULT = "now()")]
    pub now_text: String,
    #[column(VARCHAR(20), DEFAULT = "(none)")]
    pub parens: String,
    #[column(TIMESTAMP, DEFAULT = CURRENT_TIMESTAMP, ON_UPDATE = "CURRENT_TIMESTAMP")]
    pub updated_at: String,
    #[column(VARCHAR(255), UNIQUE)]
    pub email: String,
    #[column(CHECK = "low < high")]
    pub low: i32,
    pub high: i32,
    #[column(generated(STORED, "low * 2"))]
    pub doubled: i64,
    #[column(REFERENCES = RegressParents::id, ON_DELETE = CASCADE)]
    pub parent_id: u64,
}

#[MySQLTable(NAME = "mysql_regress_latin", DEFAULT_CHARSET = "latin1")]
pub struct RegressLatin {
    #[column(PRIMARY)]
    pub id: u64,
    #[column(VARCHAR(20))]
    pub name: String,
    #[column(VARCHAR(20), CHARSET = "ascii")]
    pub code: String,
}

#[MySQLIndex(using = "hash")]
pub struct MysqlRegressLatinNameIdx(RegressLatin::name);

/// The derived foreign-key name would be longer than 64 characters.
#[MySQLTable(NAME = "mysql_regress_organization_membership_invitations")]
pub struct RegressInvitations {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(REFERENCES = RegressParents::id)]
    pub inviting_organization_parent_id: u64,
}

#[MySQLTable(NAME = "mysql_regress_codes")]
pub struct RegressCodes {
    #[column(PRIMARY)]
    pub tenant: u64,
    #[column(PRIMARY)]
    pub code: u64,
}

#[MySQLTable(
    NAME = "mysql_regress_code_pairs",
    FOREIGN_KEY(columns(tenant, first_code), references(RegressCodes, tenant, code)),
    FOREIGN_KEY(columns(tenant, second_code), references(RegressCodes, tenant, code))
)]
pub struct RegressCodePairs {
    #[column(PRIMARY)]
    pub id: u64,
    pub tenant: u64,
    pub first_code: u64,
    pub second_code: u64,
}

#[MySQLView(
    NAME = "mysql_regress_positive_lows",
    DEFINITION = "SELECT id, low FROM mysql_regress_defaults WHERE low > 0"
)]
pub struct RegressPositiveLows {
    pub id: u64,
    pub low: i32,
}

#[derive(MySQLSchema)]
pub struct RegressionSchema {
    pub parents: RegressParents,
    pub defaults: RegressDefaults,
    pub latin: RegressLatin,
    pub latin_name_idx: MysqlRegressLatinNameIdx,
    pub invitations: RegressInvitations,
    pub codes: RegressCodes,
    pub code_pairs: RegressCodePairs,
    pub positive_lows: RegressPositiveLows,
}

/// The statements a push of `desired` would run against `live`, planned the
/// way `push` plans them (with the live catalog defaults).
fn push_plan(
    live: Snapshot,
    desired: &impl MigrationSchema,
    [database, engine, charset, collation]: [String; 4],
) -> Vec<String> {
    let desired = desired.to_snapshot();
    let (Snapshot::MySQL(live), Snapshot::MySQL(wanted)) = (&live, &desired) else {
        panic!("MySQL snapshots expected");
    };
    let live = live
        .prepare_for_push(wanted, &database)
        .expect("prepare the live snapshot for push");
    let defaults = MySQLCatalogDefaults::new()
        .engine(engine)
        .charset(charset)
        .collation(collation);
    diff_with(
        &Snapshot::MySQL(live),
        &desired,
        &DiffOptions::new().mysql_catalog_defaults(defaults),
    )
    .expect("plan push")
    .statements
}

const CATALOG_DEFAULTS: [&str; 4] = [
    "SELECT DATABASE()",
    "SELECT @@default_storage_engine",
    "SELECT DEFAULT_CHARACTER_SET_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME = DATABASE()",
    "SELECT DEFAULT_COLLATION_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME = DATABASE()",
];

#[drizzle::test]
fn created_and_pushed_schemas_need_no_further_push(db: &mut TestDb<RegressionSchema>) {
    // The fixture ran `db.create()`: a column CHECK naming another column
    // only works as a table constraint, and constraint names must be the
    // ones push and generate use.
    let mut defaults = Vec::new();
    for query in CATALOG_DEFAULTS {
        let value: String = result!(db.get(SQL::raw(query))).expect("read catalog defaults");
        defaults.push(value);
    }
    let defaults: [String; 4] = defaults.try_into().expect("four catalog values");

    let live = result!(db.introspect()).expect("introspect created schema");
    let pending = push_plan(live, &schema, defaults.clone());
    assert!(pending.is_empty(), "push after create(): {pending:#?}");

    result!(db.execute(SQL::raw("DROP VIEW IF EXISTS mysql_regress_positive_lows")))
        .expect("drop view");
    result!(db.execute(SQL::raw("SET FOREIGN_KEY_CHECKS = 0"))).expect("disable FK checks");
    for table in [
        "mysql_regress_code_pairs",
        "mysql_regress_codes",
        "mysql_regress_organization_membership_invitations",
        "mysql_regress_latin",
        "mysql_regress_defaults",
        "mysql_regress_parents",
    ] {
        result!(db.execute(SQL::raw(format!("DROP TABLE IF EXISTS `{table}`"))))
            .expect("drop table");
    }
    result!(db.execute(SQL::raw("SET FOREIGN_KEY_CHECKS = 1"))).expect("enable FK checks");

    result!(db.push(&schema)).expect("push the regression schema");
    let live = result!(db.introspect()).expect("introspect pushed schema");
    let pending = push_plan(live, &schema, defaults);
    assert!(pending.is_empty(), "second push: {pending:#?}");
    result!(db.push(&schema)).expect("repeated push");

    result!(db.execute(SQL::raw("DROP VIEW IF EXISTS mysql_regress_positive_lows")))
        .expect("drop view");
}

#[cfg(feature = "mysql-sync")]
mod sync {
    use super::*;
    use crate::common::helpers::mysql_sync_setup;
    use drizzle::Dialect;
    use drizzle::migrations::parser::SchemaParser;
    use drizzle::mysql::mysql_sync::Drizzle;
    use mysql::prelude::Queryable as _;

    /// `PRIMARY KEY (tenant, id)` does not lead with the AUTO_INCREMENT
    /// column, so its own index has to be part of CREATE TABLE.
    #[MySQLTable(NAME = "mysql_regress_tickets")]
    pub struct RegressTickets {
        #[column(PRIMARY)]
        pub tenant: u64,
        #[column(PRIMARY, AUTO_INCREMENT)]
        pub id: u64,
    }

    #[MySQLIndex]
    pub struct MysqlRegressTicketsId(RegressTickets::id);

    #[derive(MySQLSchema)]
    pub struct TicketSchema {
        pub tickets: RegressTickets,
        pub tickets_id: MysqlRegressTicketsId,
    }

    fn connect() -> mysql::Conn {
        mysql::Conn::new(mysql_sync_setup::options()).expect("connect to MySQL")
    }

    fn drop_all(connection: &mut mysql::Conn, tables: &[&str]) {
        connection
            .query_drop("SET FOREIGN_KEY_CHECKS = 0")
            .expect("disable FK checks");
        connection
            .query_drop("DROP VIEW IF EXISTS mysql_regress_positive_lows")
            .expect("drop view");
        for table in tables {
            connection
                .query_drop(format!("DROP TABLE IF EXISTS `{table}`"))
                .expect("drop table");
        }
        connection
            .query_drop("SET FOREIGN_KEY_CHECKS = 1")
            .expect("enable FK checks");
    }

    fn apply(connection: &mut mysql::Conn, statements: &[String]) {
        for statement in statements {
            connection
                .query_drop(statement)
                .unwrap_or_else(|error| panic!("MySQL rejected `{statement}`: {error}"));
        }
    }

    fn catalog_defaults(connection: &mut mysql::Conn) -> [String; 4] {
        CATALOG_DEFAULTS.map(|query| {
            connection
                .query_first::<String, _>(query)
                .expect("read catalog defaults")
                .expect("catalog value")
        })
    }

    fn pending<S>(desired: &S) -> Vec<String>
    where
        S: MigrationSchema,
    {
        let mut connection = connect();
        let defaults = catalog_defaults(&mut connection);
        let (mut db, ()) = Drizzle::<mysql::Conn, ()>::new(connection);
        let live = db.introspect().expect("introspect");
        push_plan(live, desired, defaults)
    }

    const REGRESSION_TABLES: [&str; 7] = [
        "mysql_regress_code_pairs",
        "mysql_regress_codes",
        "mysql_regress_organization_membership_invitations",
        "mysql_regress_latin",
        "mysql_regress_defaults",
        "mysql_regress_parents",
        "mysql_regress_tickets",
    ];

    fn generate_apply_and_push<S: MigrationSchema + Default>() {
        let schema = S::default();
        let mut connection = connect();
        drop_all(&mut connection, &REGRESSION_TABLES);

        let create = drizzle::migrations::diff(&Snapshot::empty(Dialect::MySQL), &schema.to_snapshot())
            .expect("generate the initial migration");
        apply(&mut connection, &create.statements);
        let after_generate = pending(&schema);
        assert!(
            after_generate.is_empty(),
            "push after the generated migration: {after_generate:#?}"
        );

        drop_all(&mut connection, &REGRESSION_TABLES);
        let (mut db, ()) = Drizzle::<mysql::Conn, ()>::new(connect());
        db.push(&schema).expect("push");
        let after_push = pending(&schema);
        assert!(after_push.is_empty(), "second push: {after_push:#?}");
        db.push(&schema).expect("repeated push");
        drop_all(&mut connection, &REGRESSION_TABLES);
    }

    #[test]
    fn generated_migrations_apply_and_push_has_nothing_left_to_do() {
        let _guard = mysql_sync_setup::acquire_lock();
        generate_apply_and_push::<RegressionSchema>();
        generate_apply_and_push::<TicketSchema>();
    }

    fn snapshot(source: &str) -> Snapshot {
        let parsed = SchemaParser::parse(source);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        Snapshot::from_parse_result(&parsed, Dialect::MySQL, None)
    }

    /// Applies each schema version's generated migration in turn and
    /// returns the last snapshot.
    fn migrate(
        connection: &mut mysql::Conn,
        mut previous: Snapshot,
        versions: &[(&str, DiffOptions)],
    ) -> Snapshot {
        for (source, options) in versions {
            let next = snapshot(source);
            let plan = diff_with(&previous, &next, options).expect("plan migration");
            apply(connection, &plan.statements);
            previous = next;
        }
        previous
    }

    fn index_count(connection: &mut mysql::Conn, table: &str) -> usize {
        connection
            .exec_first::<usize, _, _>(
                "SELECT COUNT(DISTINCT INDEX_NAME) FROM information_schema.STATISTICS \
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?",
                (table,),
            )
            .expect("count indexes")
            .unwrap_or_default()
    }

    #[test]
    fn generated_alterations_run_on_mysql() {
        let _guard = mysql_sync_setup::acquire_lock();
        let mut connection = connect();
        let tables = ["mysql_regress_alter", "mysql_regress_alter_parent"];
        drop_all(&mut connection, &tables);

        let none = DiffOptions::new;
        let current = migrate(
            &mut connection,
            Snapshot::empty(Dialect::MySQL),
            &[
                // AUTO_INCREMENT primary key with a generated column.
                (
                    r#"
#[MySQLTable(NAME = "mysql_regress_alter_parent")]
pub struct Parent { #[column(PRIMARY)] pub id: u64 }
#[MySQLTable(NAME = "mysql_regress_alter", CHECK(name = "mysql_regress_alter_chk", expr = "qty > 0"))]
pub struct Alter {
    #[column(PRIMARY, AUTO_INCREMENT)] pub id: u64,
    pub tenant: u64,
    pub qty: i32,
    #[column(generated(STORED, "qty + 1"))] pub qty_next: i32,
    #[column(VARCHAR(40))] pub email: String,
    #[column(REFERENCES = Parent::id)] pub parent_id: u64,
}
"#,
                    none(),
                ),
                // Rename qty -> amount while a new column takes the old name;
                // widen the key of the AUTO_INCREMENT column; make email
                // unique; drop the foreign key.
                (
                    r#"
#[MySQLTable(NAME = "mysql_regress_alter_parent")]
pub struct Parent { #[column(PRIMARY)] pub id: u64 }
#[MySQLTable(NAME = "mysql_regress_alter", CHECK(name = "mysql_regress_alter_chk", expr = "amount > 0 AND qty < 100"))]
pub struct Alter {
    #[column(PRIMARY, AUTO_INCREMENT)] pub id: u64,
    #[column(PRIMARY)] pub tenant: u64,
    pub amount: i32,
    #[column(DEFAULT = 0)] pub qty: i32,
    #[column(generated(STORED, "qty + 1"))] pub qty_next: i32,
    #[column(VARCHAR(40), UNIQUE)] pub email: String,
    pub parent_id: u64,
}
"#,
                    DiffOptions::new().rename_column("mysql_regress_alter", "qty", "amount"),
                ),
                // Changing the unique column must not add another index.
                (
                    r#"
#[MySQLTable(NAME = "mysql_regress_alter_parent")]
pub struct Parent { #[column(PRIMARY)] pub id: u64 }
#[MySQLTable(NAME = "mysql_regress_alter", CHECK(name = "mysql_regress_alter_chk", expr = "amount > 0 AND qty < 100"))]
pub struct Alter {
    #[column(PRIMARY, AUTO_INCREMENT)] pub id: u64,
    #[column(PRIMARY)] pub tenant: u64,
    pub amount: i32,
    #[column(DEFAULT = 0)] pub qty: i32,
    #[column(generated(STORED, "qty + 1"))] pub qty_next: i32,
    #[column(VARCHAR(60), UNIQUE)] pub email: String,
    pub parent_id: u64,
}
"#,
                    none(),
                ),
            ],
        );
        let (_, create): (String, String) = connection
            .query_first("SHOW CREATE TABLE `mysql_regress_alter`")
            .expect("inspect table")
            .expect("table exists");
        assert!(create.contains("PRIMARY KEY (`id`,`tenant`)"), "{create}");
        assert!(create.contains("(`qty` + 1)"), "{create}");
        assert!(!create.contains("`amount` + 1"), "{create}");
        assert!(create.contains("(`amount` > 0)"), "{create}");
        assert!(create.contains("(`qty` < 100)"), "{create}");
        // PRIMARY plus one unique index on email; the dropped foreign key
        // took the index InnoDB created for it along.
        assert_eq!(index_count(&mut connection, "mysql_regress_alter"), 2, "{create}");

        // Dropping UNIQUE drops its index.
        migrate(
            &mut connection,
            current,
            &[(
                r#"
#[MySQLTable(NAME = "mysql_regress_alter_parent")]
pub struct Parent { #[column(PRIMARY)] pub id: u64 }
#[MySQLTable(NAME = "mysql_regress_alter", CHECK(name = "mysql_regress_alter_chk", expr = "amount > 0 AND qty < 100"))]
pub struct Alter {
    #[column(PRIMARY, AUTO_INCREMENT)] pub id: u64,
    #[column(PRIMARY)] pub tenant: u64,
    pub amount: i32,
    #[column(DEFAULT = 0)] pub qty: i32,
    #[column(generated(STORED, "qty + 1"))] pub qty_next: i32,
    #[column(VARCHAR(60))] pub email: String,
    pub parent_id: u64,
}
"#,
                none(),
            )],
        );
        assert_eq!(index_count(&mut connection, "mysql_regress_alter"), 1);
        drop_all(&mut connection, &tables);
    }

    #[test]
    fn key_and_generated_column_transitions_run_on_mysql() {
        let _guard = mysql_sync_setup::acquire_lock();
        let mut connection = connect();
        let tables = ["mysql_regress_keys"];
        let versions = [
            // Composite key containing the AUTO_INCREMENT column.
            r#"#[MySQLTable(NAME = "mysql_regress_keys")]
pub struct Keys { #[column(PRIMARY, AUTO_INCREMENT)] pub id: u64, #[column(PRIMARY)] pub tenant: u64 }"#,
            // Back to a single-column AUTO_INCREMENT key.
            r#"#[MySQLTable(NAME = "mysql_regress_keys")]
pub struct Keys { #[column(PRIMARY, AUTO_INCREMENT)] pub id: u64, pub tenant: u64 }"#,
            // The key moves off the column, which stops auto-incrementing.
            r#"#[MySQLTable(NAME = "mysql_regress_keys")]
pub struct Keys { pub id: u64, #[column(PRIMARY)] pub tenant: u64 }"#,
            // A new AUTO_INCREMENT primary key column.
            r#"#[MySQLTable(NAME = "mysql_regress_keys")]
pub struct Keys { #[column(PRIMARY, AUTO_INCREMENT)] pub serial: u64, pub id: u64, pub tenant: u64 }"#,
            // Generated columns are added after the columns they read.
            r#"#[MySQLTable(NAME = "mysql_regress_keys")]
pub struct Keys {
    #[column(PRIMARY, AUTO_INCREMENT)] pub serial: u64, pub id: u64, pub tenant: u64,
    #[column(generated(STORED, "zz_base * 2"))] pub a_double: i32,
    pub zz_base: i32,
    #[column(generated(VIRTUAL, "zz_base * 3"))] pub b_triple: i32,
}"#,
            // ... and dropped before them.
            r#"#[MySQLTable(NAME = "mysql_regress_keys")]
pub struct Keys { #[column(PRIMARY, AUTO_INCREMENT)] pub serial: u64, pub id: u64, pub tenant: u64 }"#,
        ];
        drop_all(&mut connection, &tables);
        let mut previous = Snapshot::empty(Dialect::MySQL);
        for source in versions {
            let next = snapshot(source);
            let plan = drizzle::migrations::diff(&previous, &next).expect("plan");
            apply(&mut connection, &plan.statements);
            previous = next;
        }
        drop_all(&mut connection, &tables);
    }

    #[test]
    fn table_character_set_changes_reach_inheriting_columns() {
        let _guard = mysql_sync_setup::acquire_lock();
        let mut connection = connect();
        let tables = ["mysql_regress_charset"];
        drop_all(&mut connection, &tables);
        migrate(
            &mut connection,
            Snapshot::empty(Dialect::MySQL),
            &[
                (
                    r#"#[MySQLTable(NAME = "mysql_regress_charset", DEFAULT_CHARSET = "latin1")]
pub struct Charset { #[column(PRIMARY)] pub id: u64, #[column(VARCHAR(20))] pub name: String, #[column(VARCHAR(20), CHARSET = "ascii")] pub code: String }"#,
                    DiffOptions::new(),
                ),
                (
                    r#"#[MySQLTable(NAME = "mysql_regress_charset", DEFAULT_CHARSET = "utf8mb4")]
pub struct Charset { #[column(PRIMARY)] pub id: u64, #[column(VARCHAR(20))] pub name: String, #[column(VARCHAR(20), CHARSET = "ascii")] pub code: String }"#,
                    DiffOptions::new(),
                ),
            ],
        );
        let charsets: Vec<(String, String)> = connection
            .query(
                "SELECT COLUMN_NAME, CHARACTER_SET_NAME FROM information_schema.COLUMNS \
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'mysql_regress_charset' \
                 AND CHARACTER_SET_NAME IS NOT NULL ORDER BY COLUMN_NAME",
            )
            .expect("read column charsets");
        assert_eq!(
            charsets,
            [
                ("code".to_string(), "ascii".to_string()),
                ("name".to_string(), "utf8mb4".to_string()),
            ]
        );

        // Push can return the table to the database default.
        let desired = snapshot(
            r#"#[MySQLTable(NAME = "mysql_regress_charset")]
pub struct Charset { #[column(PRIMARY)] pub id: u64, #[column(VARCHAR(20))] pub name: String, #[column(VARCHAR(20), CHARSET = "ascii")] pub code: String }"#,
        );
        let defaults = catalog_defaults(&mut connection);
        let (mut db, ()) = Drizzle::<mysql::Conn, ()>::new(connect());
        let live = db.introspect().expect("introspect");
        let plan = plan_against(live, &desired, defaults.clone());
        assert!(
            plan.iter().all(|statement| !statement.contains("=DEFAULT")),
            "{plan:#?}"
        );
        apply(&mut connection, &plan);
        let live = db.introspect().expect("introspect");
        let remaining = plan_against(live, &desired, defaults);
        assert!(remaining.is_empty(), "{remaining:#?}");
        drop_all(&mut connection, &tables);
    }

    fn plan_against(live: Snapshot, desired: &Snapshot, defaults: [String; 4]) -> Vec<String> {
        let [database, engine, charset, collation] = defaults;
        let (Snapshot::MySQL(live), Snapshot::MySQL(wanted)) = (&live, desired) else {
            panic!("MySQL snapshots expected");
        };
        let live = live
            .prepare_for_push(wanted, &database)
            .expect("prepare the live snapshot for push");
        diff_with(
            &Snapshot::MySQL(live),
            desired,
            &DiffOptions::new().mysql_catalog_defaults(
                MySQLCatalogDefaults::new()
                    .engine(engine)
                    .charset(charset)
                    .collation(collation),
            ),
        )
        .expect("plan push")
        .statements
    }
}
