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
