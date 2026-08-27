#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
use crate::common::schema::mysql::TestSchema;
use crate::common::seed::{
    ConstantName, NameGenerator, Param, RelatedOptions, SeedContract, SimpleOptions, Statement,
};
use drizzle::mysql::prelude::*;
use drizzle_seed::{Generator, GeneratorKind, RngCore, SeedConfig, SeedError, SeedValue};

#[MySQLTable(NAME = "seed_simple")]
struct ContractSimple {
    #[column(PRIMARY)]
    id: i32,
    #[column(VARCHAR(255))]
    name: String,
}

#[MySQLTable(NAME = "seed_parent")]
struct ContractParent {
    #[column(PRIMARY)]
    id: i32,
    #[column(VARCHAR(255))]
    name: String,
}

#[MySQLTable(NAME = "seed_child")]
struct ContractChild {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = ContractParent::id)]
    parent_id: i32,
    #[column(VARCHAR(255))]
    value: String,
}

#[MySQLTable(NAME = "seed_profile")]
struct ContractProfile {
    #[column(PRIMARY)]
    id: i32,
    #[column(VARCHAR(255))]
    email: String,
    #[column(VARCHAR(255))]
    name: String,
    #[column(TEXT)]
    description: String,
}

#[MySQLTable(NAME = "seed_self_reference")]
struct ContractSelfReference {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = ContractSelfReference::id)]
    parent_id: Option<i32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, MySQLEnum)]
enum SeedRole {
    #[default]
    Member,
    Admin,
}

#[MySQLTable(NAME = "seed_specific")]
struct MySQLSpecific {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
    unsigned_count: u32,
    #[column(ENUM)]
    status: SeedRole,
    #[column(SET("reader", "writer", "admin"))]
    permissions: String,
    #[column(YEAR)]
    founded_year: u16,
    #[column(generated(STORED, "CHAR_LENGTH(name)"))]
    name_length: u32,
}

#[MySQLTable(NAME = "seed_temporal")]
struct MySQLTemporal {
    #[column(PRIMARY)]
    id: i32,
    #[column(DATE)]
    event_date: String,
    #[column(TIME(2))]
    event_time: String,
    #[column(DATETIME(3))]
    event_at: String,
    #[column(TIMESTAMP(6))]
    updated_at: String,
}

#[MySQLTable(DATABASE = "seed_a", NAME = "duplicate")]
struct QualifiedA {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: i32,
}

#[MySQLTable(DATABASE = "seed_b", NAME = "duplicate")]
struct QualifiedB {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: i32,
}

#[derive(MySQLSchema)]
struct ContractSimpleSchema {
    simple: ContractSimple,
}

#[derive(MySQLSchema)]
struct ContractRelatedSchema {
    parent: ContractParent,
    child: ContractChild,
}

#[derive(MySQLSchema)]
struct ContractProfileSchema {
    profile: ContractProfile,
}

#[derive(MySQLSchema)]
struct ContractAllSchema {
    simple: ContractSimple,
    parent: ContractParent,
    child: ContractChild,
    profile: ContractProfile,
}

#[derive(MySQLSchema)]
struct ContractSelfReferenceSchema {
    nodes: ContractSelfReference,
}

#[derive(MySQLSchema)]
struct MySQLSpecificSchema {
    specific: MySQLSpecific,
}

#[derive(MySQLSchema)]
struct MySQLTemporalSchema {
    temporal: MySQLTemporal,
}

#[derive(MySQLSchema)]
struct QualifiedSchema {
    first: QualifiedA,
    second: QualifiedB,
}

struct MySQLSeedContract;

struct FixedYear;

impl Generator for FixedYear {
    fn generate(&self, _rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Integer(2024)
    }

    fn name(&self) -> &'static str {
        "FixedYear"
    }
}

impl SeedContract for MySQLSeedContract {
    fn simple(options: SimpleOptions) -> Vec<Statement> {
        let schema = ContractSimpleSchema::new();
        let mut config = SeedConfig::mysql(&schema).seed(options.seed);
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
        let mut config = SeedConfig::mysql(&schema).seed(options.seed);
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
        SeedConfig::mysql(&schema)
            .reset_plan()
            .unwrap()
            .into_iter()
            .map(|statement| statement.sql())
            .collect()
    }

    fn reset_self_referential() -> Vec<String> {
        let schema = ContractSelfReferenceSchema::new();
        SeedConfig::mysql(&schema)
            .reset_plan()
            .unwrap()
            .into_iter()
            .map(|statement| statement.sql())
            .collect()
    }

    fn parameter_limit_error() -> SeedError {
        let schema = ContractSimpleSchema::new();
        SeedConfig::mysql(&schema)
            .count(&schema.simple, 1)
            .max_params(1)
            .try_generate()
            .unwrap_err()
    }

    fn unsafe_reset_error() -> SeedError {
        let schema = ContractRelatedSchema::new();
        SeedConfig::mysql(&schema)
            .skip(&schema.child)
            .reset_plan()
            .unwrap_err()
    }

    fn all_tables(seed: u64, count: usize) -> Vec<Statement> {
        let schema = ContractAllSchema::new();
        SeedConfig::mysql(&schema)
            .seed(seed)
            .default_count(count)
            .generate()
            .into_iter()
            .map(normalize)
            .collect()
    }

    fn profiles(seed: u64, count: usize) -> Vec<Statement> {
        let schema = ContractProfileSchema::new();
        SeedConfig::mysql(&schema)
            .seed(seed)
            .count(&schema.profile, count)
            .generate()
            .into_iter()
            .map(normalize)
            .collect()
    }
}

fn normalize(statement: drizzle_seed::MySQLSeedStatement) -> Statement {
    let (sql, params) = statement.build();
    Statement {
        sql,
        params: params
            .into_iter()
            .map(|param| match param {
                drizzle::mysql::values::OwnedMySQLValue::Int(value) => {
                    Param::Integer(i128::from(value))
                }
                drizzle::mysql::values::OwnedMySQLValue::UInt(value) => {
                    Param::Integer(i128::from(value))
                }
                drizzle::mysql::values::OwnedMySQLValue::Bytes(value) => String::from_utf8(value)
                    .map_or_else(
                        |error| Param::Other(format!("Bytes({:?})", error.into_bytes())),
                        Param::Text,
                    ),
                other => Param::Other(format!("{other:?}")),
            })
            .collect(),
    }
}

crate::common::seed::seed_contract_tests!(MySQLSeedContract);

#[test]
fn mysql_seed_uses_native_rendering_and_values() {
    use drizzle::mysql::values::OwnedMySQLValue;

    let schema = MySQLSpecificSchema::new();
    let statements = SeedConfig::mysql(&schema)
        .seed(42)
        .count(&schema.specific, 3)
        .generator(&MySQLSpecific::founded_year, FixedYear)
        .generate();
    let (sql, params) = statements[0].build();

    assert!(sql.starts_with("INSERT INTO `seed_specific`"));
    assert!(sql.contains("`name`"));
    assert!(sql.contains('?'));
    assert!(sql.contains("`name_length`"));
    assert_eq!(sql.matches("DEFAULT").count(), 3);
    assert!(
        params
            .iter()
            .any(|param| matches!(param, OwnedMySQLValue::UInt(_)))
    );

    let enum_values = params
        .iter()
        .filter_map(|param| match param {
            OwnedMySQLValue::Bytes(value) if value == b"Member" || value == b"Admin" => Some(value),
            _ => None,
        })
        .count();
    assert_eq!(enum_values, 3);

    let set_values = params
        .iter()
        .filter_map(|param| match param {
            OwnedMySQLValue::Bytes(value) => std::str::from_utf8(value).ok(),
            _ => None,
        })
        .filter(|value| {
            let members = value.split(',').collect::<Vec<_>>();
            !members.is_empty()
                && members
                    .iter()
                    .all(|member| matches!(*member, "reader" | "writer" | "admin"))
        })
        .count();
    assert_eq!(set_values, 3);

    let years = params
        .iter()
        .filter_map(|param| match param {
            OwnedMySQLValue::UInt(2024) => Some(2024),
            _ => None,
        })
        .count();
    assert_eq!(years, 3);
}

#[test]
fn mysql_seed_binds_temporal_fsp_columns_as_native_values() {
    use drizzle::mysql::values::OwnedMySQLValue;

    let schema = MySQLTemporalSchema::new();
    let params = SeedConfig::mysql(&schema)
        .seed(42)
        .count(&schema.temporal, 2)
        .generate()[0]
        .build()
        .1;

    assert_eq!(
        params
            .iter()
            .filter(|param| matches!(param, OwnedMySQLValue::Date { .. }))
            .count(),
        6
    );
    assert_eq!(
        params
            .iter()
            .filter(|param| matches!(param, OwnedMySQLValue::Time { .. }))
            .count(),
        2
    );
}

#[test]
fn mysql_schema_qualified_tables_with_the_same_name_keep_distinct_counts() {
    let schema = QualifiedSchema::new();
    let statements = SeedConfig::mysql(&schema)
        .count(&schema.first, 1)
        .count(&schema.second, 2)
        .generate();

    assert_eq!(statements.len(), 2);
    assert!(
        statements
            .iter()
            .any(|statement| statement.sql().contains("`seed_a`.`duplicate`"))
    );
    assert!(
        statements
            .iter()
            .any(|statement| statement.sql().contains("`seed_b`.`duplicate`"))
    );

    let mut param_counts = statements
        .iter()
        .map(|statement| statement.build().1.len())
        .collect::<Vec<_>>();
    param_counts.sort_unstable();
    assert_eq!(param_counts, vec![1, 2]);
}

#[test]
fn mysql_reset_keeps_constraints_enabled_and_resets_qualified_auto_increment_tables_last() {
    let schema = QualifiedSchema::new();
    let statements = SeedConfig::mysql(&schema)
        .reset_plan()
        .unwrap()
        .into_iter()
        .map(|statement| statement.sql())
        .collect::<Vec<_>>();

    assert_eq!(statements.len(), 4);
    assert!(
        statements[..2]
            .iter()
            .all(|sql| sql.starts_with("DELETE FROM"))
    );
    assert!(
        statements[2..]
            .iter()
            .all(|sql| sql.starts_with("ALTER TABLE") && sql.contains("AUTO_INCREMENT = 1"))
    );
    for table in ["`seed_a`.`duplicate`", "`seed_b`.`duplicate`"] {
        assert!(statements.iter().any(|sql| sql.contains(table)));
    }
    assert!(
        statements
            .iter()
            .all(|sql| !sql.contains("FOREIGN_KEY_CHECKS"))
    );
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
#[drizzle::test]
fn mysql_reset_plan_executes_with_foreign_keys_enabled(db: &mut TestDb<TestSchema>) {
    use crate::common::schema::mysql::{InsertPost, InsertUser, Role};
    use drizzle::core::expr::count;

    let TestSchema { users, posts, .. } = schema;
    for statement in SeedConfig::mysql(&schema).default_count(1).generate() {
        db.execute(statement);
    }

    let generated_user_count: i64 = db.select(count(users.id)).from(users).get();
    let generated_post_count: i64 = db.select(count(posts.id)).from(posts).get();
    assert_eq!((generated_user_count, generated_post_count), (1, 1));

    for statement in SeedConfig::mysql(&schema).reset_plan().unwrap() {
        db.execute(statement);
    }

    let inserted = db
        .insert(users)
        .value(
            InsertUser::new("before reset", true, Role::Member, vec![], 0, 0.0)
                .with_note(None::<String>),
        )
        .execute();
    let user_id = inserted.last_insert_id().unwrap();
    db.insert(posts)
        .value(InsertPost::new(user_id, "before reset"))
        .execute();

    for statement in SeedConfig::mysql(&schema).reset_plan().unwrap() {
        db.execute(statement);
    }

    let user_count: i64 = db.select(count(users.id)).from(users).get();
    let post_count: i64 = db.select(count(posts.id)).from(posts).get();
    assert_eq!((user_count, post_count), (0, 0));

    let inserted = db
        .insert(users)
        .value(
            InsertUser::new("after reset", true, Role::Member, vec![], 0, 0.0)
                .with_note(None::<String>),
        )
        .execute();
    assert_eq!(inserted.last_insert_id(), Some(1));
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
#[drizzle::test]
fn mysql_reset_plan_clears_live_nullable_self_references(
    db: &mut TestDb<ContractSelfReferenceSchema>,
) {
    use drizzle::core::expr::count;

    let ContractSelfReferenceSchema { nodes } = schema;
    db.insert(nodes)
        .value(InsertContractSelfReference::new(1).with_parent_id(None::<i32>))
        .execute();
    db.insert(nodes)
        .value(InsertContractSelfReference::new(2).with_parent_id(1))
        .execute();

    for statement in SeedConfig::mysql(&schema).reset_plan().unwrap() {
        db.execute(statement);
    }

    let remaining: i64 = db.select(count(nodes.id)).from(nodes).get();
    assert_eq!(remaining, 0);
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
#[drizzle::test]
fn mysql_specific_seed_values_execute(db: &mut TestDb<MySQLSpecificSchema>) {
    use drizzle::core::expr::count;

    let MySQLSpecificSchema { specific } = schema;
    for statement in SeedConfig::mysql(&schema)
        .count(&specific, 3)
        .generator(&MySQLSpecific::founded_year, FixedYear)
        .generate()
    {
        db.execute(statement);
    }

    let inserted: i64 = db.select(count(specific.id)).from(specific).get();
    assert_eq!(inserted, 3);
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
#[drizzle::test]
fn mysql_temporal_seed_values_execute(db: &mut TestDb<MySQLTemporalSchema>) {
    use drizzle::core::expr::count;

    let MySQLTemporalSchema { temporal } = schema;
    for statement in SeedConfig::mysql(&schema).count(&temporal, 2).generate() {
        db.execute(statement);
    }

    let inserted: i64 = db.select(count(temporal.id)).from(temporal).get();
    assert_eq!(inserted, 2);
}
