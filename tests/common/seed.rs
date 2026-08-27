use drizzle_seed::{Generator, RngCore, SeedError, SeedValue};

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub sql: String,
    pub params: Vec<Param>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Integer(i128),
    Text(String),
    Other(String),
}

#[derive(Clone, Copy, Debug)]
pub enum NameGenerator {
    Inferred,
    Email,
    Constant,
    Column,
}

#[derive(Clone, Copy, Debug)]
pub struct SimpleOptions {
    pub seed: u64,
    pub count: Option<usize>,
    pub default_count: Option<usize>,
    pub max_params: Option<usize>,
    pub name_generator: NameGenerator,
}

impl Default for SimpleOptions {
    fn default() -> Self {
        Self {
            seed: 42,
            count: Some(5),
            default_count: None,
            max_params: None,
            name_generator: NameGenerator::Inferred,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RelatedOptions {
    pub seed: u64,
    pub parent_count: Option<usize>,
    pub child_count: Option<usize>,
    pub children_per_parent: Option<usize>,
    pub skip_parent: bool,
    pub skip_child: bool,
}

impl Default for RelatedOptions {
    fn default() -> Self {
        Self {
            seed: 42,
            parent_count: Some(3),
            child_count: Some(10),
            children_per_parent: None,
            skip_parent: false,
            skip_child: false,
        }
    }
}

pub trait SeedContract {
    fn simple(options: SimpleOptions) -> Vec<Statement>;
    fn related(options: RelatedOptions) -> Vec<Statement>;
    fn reset_related() -> Vec<String>;
    fn reset_self_referential() -> Vec<String>;
    fn parameter_limit_error() -> SeedError;
    fn unsafe_reset_error() -> SeedError;
    fn all_tables(seed: u64, count: usize) -> Vec<Statement>;
    fn profiles(seed: u64, count: usize) -> Vec<Statement>;
}

pub struct ConstantName;

impl Generator for ConstantName {
    fn generate(&self, _rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Text("FIXED".to_string())
    }

    fn name(&self) -> &'static str {
        "ConstantName"
    }
}

pub fn assert_simple_generation<C: SeedContract>() {
    let statements = C::simple(SimpleOptions::default());

    assert_eq!(statements.len(), 1);
    let statement = &statements[0];
    assert_insert_for(&statement.sql, "seed_simple");
    assert_eq!(row_count(&statement.sql), 5);
    assert_eq!(statement.params.len(), 10);

    let ids = statement
        .params
        .iter()
        .step_by(2)
        .map(integer)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![1, 2, 3, 4, 5]);

    assert!(
        statement
            .params
            .iter()
            .skip(1)
            .step_by(2)
            .all(|param| { matches!(param, Param::Text(value) if !value.is_empty()) })
    );
}

pub fn assert_configuration_overrides<C: SeedContract>() {
    for name_generator in [
        NameGenerator::Email,
        NameGenerator::Constant,
        NameGenerator::Column,
    ] {
        let statements = C::simple(SimpleOptions {
            count: Some(4),
            name_generator,
            ..SimpleOptions::default()
        });
        let names = statements[0]
            .params
            .iter()
            .skip(1)
            .step_by(2)
            .map(text)
            .collect::<Vec<_>>();

        match name_generator {
            NameGenerator::Email => assert!(
                names
                    .iter()
                    .all(|name| name.contains('@') && name.contains('.'))
            ),
            NameGenerator::Constant => assert_eq!(names, vec!["FIXED"; 4]),
            NameGenerator::Column => assert!(names.iter().all(|name| !name.is_empty())),
            NameGenerator::Inferred => unreachable!(),
        }
    }
}

pub fn assert_determinism<C: SeedContract>() {
    let options = SimpleOptions {
        seed: 123,
        count: Some(20),
        ..SimpleOptions::default()
    };
    assert_eq!(C::simple(options), C::simple(options));

    let first = C::simple(SimpleOptions {
        seed: 1,
        count: Some(10),
        ..SimpleOptions::default()
    });
    let second = C::simple(SimpleOptions {
        seed: 2,
        count: Some(10),
        ..SimpleOptions::default()
    });
    assert_ne!(first[0].params, second[0].params);
}

pub fn assert_count_defaults_and_batching<C: SeedContract>() {
    assert!(
        C::simple(SimpleOptions {
            count: Some(0),
            ..SimpleOptions::default()
        })
        .is_empty()
    );

    let default_count = C::simple(SimpleOptions {
        count: None,
        default_count: Some(7),
        ..SimpleOptions::default()
    });
    assert_eq!(row_count(&default_count[0].sql), 7);

    let batches = C::simple(SimpleOptions {
        count: Some(10),
        max_params: Some(4),
        ..SimpleOptions::default()
    });
    assert_eq!(batches.len(), 5);
    assert!(batches.iter().all(|statement| statement.params.len() <= 4));
}

pub fn assert_relations<C: SeedContract>() {
    let statements = C::related(RelatedOptions::default());
    let parent_index = table_index(&statements, "seed_parent");
    let child_index = table_index(&statements, "seed_child");
    assert!(parent_index < child_index);

    let child = &statements[child_index];
    let parent_ids = child
        .params
        .iter()
        .skip(1)
        .step_by(3)
        .map(integer)
        .collect::<Vec<_>>();
    assert!(parent_ids.iter().all(|id| (1..=3).contains(id)));

    let grouped = C::related(RelatedOptions {
        parent_count: Some(3),
        child_count: None,
        children_per_parent: Some(2),
        ..RelatedOptions::default()
    });
    let child = &grouped[table_index(&grouped, "seed_child")];
    assert_eq!(row_count(&child.sql), 6);
    assert_eq!(
        child
            .params
            .iter()
            .skip(1)
            .step_by(3)
            .map(integer)
            .collect::<Vec<_>>(),
        vec![1, 1, 2, 2, 3, 3]
    );

    let derived = C::related(RelatedOptions {
        parent_count: Some(4),
        child_count: None,
        ..RelatedOptions::default()
    });
    let child = &derived[table_index(&derived, "seed_child")];
    assert_eq!(row_count(&child.sql), 4);
}

pub fn assert_reset_order<C: SeedContract>() {
    let statements = C::reset_related();
    assert_eq!(statements.len(), 2);
    assert!(statements[0].starts_with("DELETE FROM"));
    assert!(statements[0].contains("seed_child"));
    assert!(statements[1].starts_with("DELETE FROM"));
    assert!(statements[1].contains("seed_parent"));
}

pub fn assert_nullable_self_reference_reset<C: SeedContract>() {
    let statements = C::reset_self_referential();
    assert_eq!(statements.len(), 2);
    assert!(statements[0].starts_with("UPDATE"));
    assert!(statements[0].contains("parent_id"));
    assert!(statements[0].contains("NULL"));
    assert!(statements[1].starts_with("DELETE FROM"));
}

pub fn assert_public_error_paths<C: SeedContract>() {
    assert!(matches!(
        C::parameter_limit_error(),
        SeedError::ParameterLimitTooLow {
            required: 2,
            limit: 1,
            ..
        }
    ));
    assert!(matches!(
        C::unsafe_reset_error(),
        SeedError::UnsafeResetSelection { .. }
    ));
}

pub fn assert_table_selection<C: SeedContract>() {
    let without_parent = C::related(RelatedOptions {
        parent_count: None,
        child_count: Some(2),
        skip_parent: true,
        ..RelatedOptions::default()
    });
    assert!(!has_table(&without_parent, "seed_parent"));
    assert!(has_table(&without_parent, "seed_child"));

    let without_child = C::related(RelatedOptions {
        parent_count: Some(3),
        child_count: None,
        skip_child: true,
        ..RelatedOptions::default()
    });
    assert!(has_table(&without_child, "seed_parent"));
    assert!(!has_table(&without_child, "seed_child"));

    let all = C::all_tables(7, 2);
    for table in ["seed_simple", "seed_parent", "seed_child", "seed_profile"] {
        assert!(has_table(&all, table), "missing INSERT for {table}");
    }
    assert!(table_index(&all, "seed_parent") < table_index(&all, "seed_child"));
}

pub fn assert_inferred_generators<C: SeedContract>() {
    let statements = C::profiles(42, 10);
    let statement = &statements[0];
    for column in ["id", "email", "name", "description"] {
        assert!(statement.sql.contains(column), "missing column {column}");
    }

    let emails = statement
        .params
        .iter()
        .filter_map(|param| match param {
            Param::Text(value) if value.contains('@') && value.contains('.') => Some(value),
            _ => None,
        })
        .count();
    assert!(emails > 0);
}

fn assert_insert_for(sql: &str, table: &str) {
    assert!(sql.starts_with("INSERT INTO"));
    assert!(sql.contains(table));
    assert!(sql.contains("VALUES"));
}

fn row_count(sql: &str) -> usize {
    let values = sql
        .split_once("VALUES ")
        .unwrap_or_else(|| panic!("missing VALUES clause in {sql}"))
        .1;
    values.matches('(').count()
}

fn table_index(statements: &[Statement], table: &str) -> usize {
    statements
        .iter()
        .position(|statement| statement.sql.contains(table))
        .unwrap_or_else(|| panic!("missing INSERT for {table}"))
}

fn has_table(statements: &[Statement], table: &str) -> bool {
    statements
        .iter()
        .any(|statement| statement.sql.contains(table))
}

fn integer(param: &Param) -> i128 {
    let Param::Integer(value) = param else {
        panic!("expected integer parameter, got {param:?}");
    };
    *value
}

fn text(param: &Param) -> &str {
    let Param::Text(value) = param else {
        panic!("expected text parameter, got {param:?}");
    };
    value
}

macro_rules! seed_contract_tests {
    ($contract:ty) => {
        #[test]
        fn shared_seed_simple_generation() {
            $crate::common::seed::assert_simple_generation::<$contract>();
        }

        #[test]
        fn shared_seed_configuration_overrides() {
            $crate::common::seed::assert_configuration_overrides::<$contract>();
        }

        #[test]
        fn shared_seed_is_deterministic() {
            $crate::common::seed::assert_determinism::<$contract>();
        }

        #[test]
        fn shared_seed_count_defaults_and_batching() {
            $crate::common::seed::assert_count_defaults_and_batching::<$contract>();
        }

        #[test]
        fn shared_seed_relations() {
            $crate::common::seed::assert_relations::<$contract>();
        }

        #[test]
        fn shared_seed_reset_deletes_children_before_parents() {
            $crate::common::seed::assert_reset_order::<$contract>();
        }

        #[test]
        fn shared_seed_reset_clears_nullable_self_references() {
            $crate::common::seed::assert_nullable_self_reference_reset::<$contract>();
        }

        #[test]
        fn shared_seed_public_error_paths_are_typed() {
            $crate::common::seed::assert_public_error_paths::<$contract>();
        }

        #[test]
        fn shared_seed_table_selection() {
            $crate::common::seed::assert_table_selection::<$contract>();
        }

        #[test]
        fn shared_seed_inferred_generators() {
            $crate::common::seed::assert_inferred_generators::<$contract>();
        }
    };
}

pub(crate) use seed_contract_tests;
