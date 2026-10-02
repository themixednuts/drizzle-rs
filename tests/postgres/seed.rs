use crate::common::seed::{
    ConstantName, NameGenerator, Param, RelatedOptions, SeedContract, SimpleOptions, Statement,
};
use drizzle::postgres::prelude::*;
use drizzle_seed::{GeneratorKind, SeedConfig, SeedError};

#[PostgresTable(NAME = "seed_simple")]
struct ContractSimple {
    #[column(PRIMARY)]
    id: i32,
    name: String,
}

#[PostgresTable(NAME = "seed_parent")]
struct ContractParent {
    #[column(PRIMARY)]
    id: i32,
    name: String,
}

#[PostgresTable(NAME = "seed_child")]
struct ContractChild {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = ContractParent::id)]
    parent_id: i32,
    value: String,
}

#[PostgresTable(NAME = "seed_profile")]
struct ContractProfile {
    #[column(PRIMARY)]
    id: i32,
    email: String,
    name: String,
    description: String,
}

#[PostgresTable(NAME = "seed_self_reference")]
struct ContractSelfReference {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = ContractSelfReference::id)]
    parent_id: Option<i32>,
}

#[PostgresTable(SCHEMA = "seed_a", NAME = "duplicate")]
struct QualifiedA {
    #[column(PRIMARY)]
    id: i32,
}

#[PostgresTable(SCHEMA = "seed_b", NAME = "duplicate")]
struct QualifiedB {
    #[column(PRIMARY)]
    id: i32,
}

#[derive(PostgresSchema)]
struct ContractSimpleSchema {
    simple: ContractSimple,
}

#[derive(PostgresSchema)]
struct ContractRelatedSchema {
    parent: ContractParent,
    child: ContractChild,
}

#[derive(PostgresSchema)]
struct ContractProfileSchema {
    profile: ContractProfile,
}

#[derive(PostgresSchema)]
struct ContractAllSchema {
    simple: ContractSimple,
    parent: ContractParent,
    child: ContractChild,
    profile: ContractProfile,
}

#[derive(PostgresSchema)]
struct ContractSelfReferenceSchema {
    nodes: ContractSelfReference,
}

#[derive(PostgresSchema)]
struct QualifiedSchema {
    first: QualifiedA,
    second: QualifiedB,
}

struct PostgresSeedContract;

impl SeedContract for PostgresSeedContract {
    fn simple(options: SimpleOptions) -> Vec<Statement> {
        let schema = ContractSimpleSchema::new();
        let mut config = SeedConfig::postgres(&schema).seed(options.seed);
        if let Some(count) = options.count {
            config = config.count(&schema.simple, count);
        }
        if let Some(count) = options.default_count {
            config = config.default_count(count);
        }
        if let Some(max_params) = options.max_params {
            config = config.max_params(max_params);
        }
        config = match options.name_generator {
            NameGenerator::Inferred => config,
            NameGenerator::Email => config.kind(&ContractSimple::name, GeneratorKind::Email),
            NameGenerator::Constant => config.generator(&ContractSimple::name, ConstantName),
            NameGenerator::Column => config.generator(&ContractSimple::name, &ContractSimple::name),
        };
        config.generate().into_iter().map(normalize).collect()
    }

    fn related(options: RelatedOptions) -> Vec<Statement> {
        let schema = ContractRelatedSchema::new();
        let mut config = SeedConfig::postgres(&schema).seed(options.seed);
        if let Some(count) = options.parent_count {
            config = config.count(&schema.parent, count);
        }
        if let Some(count) = options.child_count {
            config = config.count(&schema.child, count);
        }
        if let Some(count) = options.children_per_parent {
            config = config.relation(&schema.parent, &schema.child, count);
        }
        if options.skip_parent {
            config = config.skip(&schema.parent);
        }
        if options.skip_child {
            config = config.skip(&schema.child);
        }
        config.generate().into_iter().map(normalize).collect()
    }

    fn reset_related() -> Vec<String> {
        let schema = ContractRelatedSchema::new();
        SeedConfig::postgres(&schema)
            .reset_plan()
            .unwrap()
            .into_iter()
            .map(|statement| statement.sql())
            .collect()
    }

    fn reset_self_referential() -> Vec<String> {
        let schema = ContractSelfReferenceSchema::new();
        SeedConfig::postgres(&schema)
            .reset_plan()
            .unwrap()
            .into_iter()
            .map(|statement| statement.sql())
            .collect()
    }

    fn parameter_limit_error() -> SeedError {
        let schema = ContractSimpleSchema::new();
        SeedConfig::postgres(&schema)
            .count(&schema.simple, 1)
            .max_params(1)
            .try_generate()
            .unwrap_err()
    }

    fn unsafe_reset_error() -> SeedError {
        let schema = ContractRelatedSchema::new();
        SeedConfig::postgres(&schema)
            .skip(&schema.child)
            .reset_plan()
            .unwrap_err()
    }

    fn all_tables(seed: u64, count: usize) -> Vec<Statement> {
        let schema = ContractAllSchema::new();
        SeedConfig::postgres(&schema)
            .seed(seed)
            .default_count(count)
            .generate()
            .into_iter()
            .map(normalize)
            .collect()
    }

    fn profiles(seed: u64, count: usize) -> Vec<Statement> {
        let schema = ContractProfileSchema::new();
        SeedConfig::postgres(&schema)
            .seed(seed)
            .count(&schema.profile, count)
            .generate()
            .into_iter()
            .map(normalize)
            .collect()
    }
}

fn normalize(statement: drizzle_seed::PostgresSeedStatement) -> Statement {
    let (sql, params) = statement.build();
    Statement {
        sql,
        params: params
            .into_iter()
            .map(|param| match param {
                drizzle::postgres::values::OwnedPostgresValue::Smallint(value) => {
                    Param::Integer(i128::from(value))
                }
                drizzle::postgres::values::OwnedPostgresValue::Integer(value) => {
                    Param::Integer(i128::from(value))
                }
                drizzle::postgres::values::OwnedPostgresValue::Bigint(value) => {
                    Param::Integer(i128::from(value))
                }
                drizzle::postgres::values::OwnedPostgresValue::Text(value) => Param::Text(value),
                other => Param::Other(format!("{other:?}")),
            })
            .collect(),
    }
}

crate::common::seed::seed_contract_tests!(PostgresSeedContract);

#[test]
fn schema_qualified_tables_with_the_same_name_keep_distinct_counts() {
    let schema = QualifiedSchema::new();
    let statements = SeedConfig::postgres(&schema)
        .count(&schema.first, 1)
        .count(&schema.second, 2)
        .generate();

    assert_eq!(statements.len(), 2);
    assert!(
        statements
            .iter()
            .any(|statement| statement.sql().contains("seed_a"))
    );
    assert!(
        statements
            .iter()
            .any(|statement| statement.sql().contains("seed_b"))
    );

    let mut param_counts = statements
        .iter()
        .map(|statement| statement.build().1.len())
        .collect::<Vec<_>>();
    param_counts.sort_unstable();
    assert_eq!(param_counts, vec![1, 2]);
}

/// Seeds the column shapes that used to fail on a real server and reads the
/// rows back through the typed models.
#[cfg(all(feature = "uuid", feature = "serde", feature = "chrono"))]
mod executed {
    use drizzle::postgres::prelude::*;
    use drizzle_seed::SeedConfig;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
    pub enum SeedMood {
        #[default]
        Calm,
        Busy,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
    #[repr(i64)]
    pub enum SeedLevel {
        #[default]
        Low = 1,
        High = 5,
    }

    #[PostgresTable(NAME = "seed_typed")]
    pub struct SeedTyped {
        #[column(PRIMARY, identity(always))]
        pub id: i32,
        #[column(UNIQUE, VARCHAR(12))]
        pub username: String,
        pub external_id: uuid::Uuid,
        #[column(JSONB)]
        pub settings: serde_json::Value,
        #[column(ENUM)]
        pub mood: SeedMood,
        #[column(ENUM)]
        pub level: SeedLevel,
        pub updated_at: chrono::NaiveDateTime,
        pub position: i32,
        pub email_count: i32,
        pub tags: Vec<String>,
        pub score: f32,
    }

    #[PostgresTable(NAME = "seed_typed_children")]
    pub struct SeedTypedChild {
        #[column(PRIMARY, SERIAL)]
        pub id: i32,
        #[column(REFERENCES = SeedTyped::id)]
        pub parent_id: i32,
        pub label: String,
    }

    #[derive(PostgresSchema)]
    pub struct SeedTypedSchema {
        pub mood: SeedMood,
        pub typed: SeedTyped,
        pub child: SeedTypedChild,
    }

    #[drizzle::test]
    fn seeded_rows_insert_and_decode(db: &mut TestDb<SeedTypedSchema>) {
        let SeedTypedSchema { typed, child, .. } = schema;
        for statement in SeedConfig::postgres(&schema)
            .seed(3)
            .count(&typed, 40)
            .relation(&typed, &child, 2)
            .generate()
        {
            db.execute(statement);
        }

        let rows: Vec<SelectSeedTyped> = db.select(()).from(typed).all();
        assert_eq!(rows.len(), 40);
        let mut usernames: Vec<&str> = rows.iter().map(|row| row.username.as_str()).collect();
        assert!(usernames.iter().all(|name| name.chars().count() <= 12));
        usernames.sort_unstable();
        usernames.dedup();
        assert_eq!(usernames.len(), 40, "UNIQUE username values repeat");

        let children: Vec<SelectSeedTypedChild> = db.select(()).from(child).all();
        assert_eq!(children.len(), 80);

        // The SERIAL sequence was moved past the seeded ids.
        db.insert(child)
            .values([InsertSeedTypedChild::new(rows[0].id, "after seed")])
            .execute();
        let children: Vec<SelectSeedTypedChild> = db.select(()).from(child).all();
        assert_eq!(children.len(), 81);
    }

    /// `SeedTypedSchema`, described at runtime instead of with the macros.
    fn runtime_schema() -> drizzle_seed::schema::Schema {
        use drizzle_seed::schema::{Column, Schema, Table};
        Schema::postgres()
            .table(
                Table::new("seed_typed")
                    .column(Column::new("id", "INTEGER").primary_key().identity_always())
                    .column(Column::new("username", "VARCHAR(12)").not_null().unique())
                    .column(Column::new("external_id", "UUID").not_null())
                    .column(Column::new("settings", "JSONB").not_null())
                    .column(
                        // Quoted, as the macro creates the type.
                        Column::new("mood", "\"SeedMood\"")
                            .not_null()
                            .enum_values(["Calm", "Busy"]),
                    )
                    .column(Column::new("level", "integer").not_null())
                    .column(Column::new("updated_at", "TIMESTAMP").not_null())
                    .column(Column::new("position", "INTEGER").not_null())
                    .column(Column::new("email_count", "INTEGER").not_null())
                    .column(Column::new("tags", "TEXT[]").not_null())
                    .column(Column::new("score", "REAL").not_null()),
            )
            .table(
                Table::new("seed_typed_children")
                    .column(Column::new("id", "SERIAL").primary_key())
                    .column(
                        Column::new("parent_id", "INTEGER")
                            .not_null()
                            .references("seed_typed", "id"),
                    )
                    .column(Column::new("label", "TEXT").not_null()),
            )
    }

    #[test]
    fn runtime_schema_and_names_match_the_macro_schema() {
        use drizzle_seed::generators;
        type Built = Vec<(String, Vec<drizzle::postgres::values::OwnedPostgresValue>)>;

        let schema = SeedTypedSchema::new();
        let typed: Built = SeedConfig::postgres(&schema)
            .seed(3)
            .count(&schema.typed, 20)
            .relation(&schema.typed, &schema.child, 2)
            .generator(&schema.typed.level, generators::one_of([1, 5]))
            .generate()
            .iter()
            .map(|statement| statement.build())
            .collect();

        let runtime = runtime_schema();
        let named: Built = SeedConfig::postgres(&runtime)
            .seed(3)
            .count_by_name("seed_typed", 20)
            .relation_by_name("public.seed_typed", "seed_typed_children", 2)
            .generator_by_name("seed_typed", "level", generators::one_of([1, 5]))
            .generate()
            .iter()
            .map(|statement| statement.build())
            .collect();

        assert!(!typed.is_empty());
        assert_eq!(typed, named);
    }

    #[drizzle::test]
    fn runtime_schema_rows_insert_and_decode(db: &mut TestDb<SeedTypedSchema>) {
        let SeedTypedSchema { typed, child, .. } = schema;
        let runtime = runtime_schema();
        for statement in SeedConfig::postgres(&runtime)
            .seed(8)
            .count_by_name("seed_typed", 25)
            .relation_by_name("seed_typed", "seed_typed_children", 3)
            .generator_by_name(
                "seed_typed",
                "level",
                drizzle_seed::generators::one_of([1, 5]),
            )
            .generate()
        {
            db.execute(statement);
        }

        let rows: Vec<SelectSeedTyped> = db.select(()).from(typed).all();
        assert_eq!(rows.len(), 25);
        let children: Vec<SelectSeedTypedChild> = db.select(()).from(child).all();
        assert_eq!(children.len(), 75);

        // The SERIAL sequence was moved past the seeded ids.
        db.insert(child)
            .values([InsertSeedTypedChild::new(rows[0].id, "after seed")])
            .execute();
    }

    const LIVE_TABLES: [&str; 4] = [
        r#"CREATE TYPE "LiveMood" AS ENUM ('calm', 'busy')"#,
        r#"CREATE TABLE live_accounts (
            id serial PRIMARY KEY,
            external_id uuid NOT NULL UNIQUE,
            handle varchar(16) NOT NULL UNIQUE,
            plan varchar(8) NOT NULL CHECK (plan IN ('free', 'pro')),
            mood "LiveMood" NOT NULL,
            settings jsonb NOT NULL,
            tags text[] NOT NULL,
            balance numeric(10, 2) NOT NULL,
            ratio real,
            avatar bytea,
            is_active boolean NOT NULL,
            born_on date,
            wakes_at time,
            created_at timestamptz NOT NULL DEFAULT now(),
            updated_at timestamp NOT NULL,
            handle_length integer GENERATED ALWAYS AS (length(handle)) STORED
        )"#,
        r"CREATE TABLE live_events (
            id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            account_id integer NOT NULL REFERENCES live_accounts (id),
            kind text NOT NULL,
            happened_at timestamp NOT NULL
        )",
        r"CREATE UNIQUE INDEX live_events_account_kind ON live_events (account_id, kind)",
    ];

    #[derive(Debug, PostgresFromRow)]
    struct Text(String);

    /// Every row of `table`, as one comparable string, without
    /// `created_at`, whose `DEFAULT now()` differs between two runs.
    fn fingerprint_sql(table: &str) -> String {
        format!(
            "SELECT string_agg((to_jsonb(t) - 'created_at')::text, '|' ORDER BY (to_jsonb(t) - 'created_at')::text) FROM {table} t"
        )
    }

    /// A database not described in Rust at all: introspect it, seed it, and
    /// check the inline script inserts exactly what the bound statements do.
    #[drizzle::test]
    fn introspected_database_seeds_with_bound_and_inline_values(db: &mut TestDb<SeedTypedSchema>) {
        use drizzle_seed::schema::Schema;

        for statement in LIVE_TABLES {
            db.execute(SQL::raw(statement));
        }
        let Text(namespace) = result!(db.get(SQL::raw("SELECT current_schema()::text"))).unwrap();
        let snapshot = result!(db.introspect_schemas(&[namespace.as_str()])).expect("introspect");
        let schema = Schema::from_snapshot(&snapshot)
            .unwrap()
            .retain(|table| table.name().starts_with("live_"));
        assert_eq!(schema.tables().len(), 2);

        let config = SeedConfig::postgres(&schema)
            .seed(11)
            .count_by_name("live_accounts", 15)
            .relation_by_name("live_accounts", "live_events", 2);
        let tables = ["live_accounts", "live_events"];

        for statement in config.generate() {
            db.execute(statement);
        }
        let mut bound = Vec::new();
        for table in tables {
            let Text(rows) = result!(db.get(SQL::raw(fingerprint_sql(table)))).unwrap();
            bound.push(rows);
        }
        let Text(plans) = result!(db.get(SQL::raw(
            "SELECT string_agg(DISTINCT plan, ',') FROM live_accounts",
        )))
        .unwrap();
        assert!(
            plans.split(',').all(|plan| plan == "free" || plan == "pro"),
            "{plans}"
        );

        for statement in config.reset_plan().unwrap() {
            db.execute(statement);
        }
        for statement in config.generate() {
            db.execute(SQL::raw(statement.inline_sql().unwrap()));
        }
        let mut inline = Vec::new();
        for table in tables {
            let Text(rows) = result!(db.get(SQL::raw(fingerprint_sql(table)))).unwrap();
            inline.push(rows);
        }
        assert_eq!(inline, bound);

        // The serial and identity sequences were moved past the seeded ids.
        db.execute(SQL::raw(
            "INSERT INTO live_accounts (external_id, handle, plan, mood, settings, tags, balance, is_active, updated_at) \
             VALUES (gen_random_uuid(), 'after', 'free', 'calm', '{}', '{}', 1, true, now())",
        ));
        db.execute(SQL::raw(
            "INSERT INTO live_events (account_id, kind, happened_at) VALUES (1, 'after seed', now())",
        ));
    }
}
