use drizzle::core::expr::CastTarget;
use drizzle::core::types as core_types;
use drizzle::postgres::prelude::*;
use drizzle::postgres::types as postgres_types;
use drizzle_core::dialect::PostgresDialect;

#[PostgresTable]
struct BoundedCharacters {
    #[column(VARCHAR(255))]
    varchar_value: String,
    #[column(CHAR(8))]
    char_value: String,
    #[column(VARCHAR(64))]
    aliases: Vec<String>,
}

#[PostgresView(EXISTING)]
struct BoundedCharacterView {
    #[column(VARCHAR(32))]
    label: String,
}

#[test]
fn explicit_bounded_characters_preserve_physical_types() {
    let sql = BoundedCharacters::create_table_sql();
    assert!(sql.contains("\"varchar_value\" VARCHAR(255) NOT NULL"));
    assert!(sql.contains("\"char_value\" CHAR(8) NOT NULL"));
    assert!(sql.contains("\"aliases\" VARCHAR(64)[] NOT NULL"));
    assert_eq!(
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[0].sql_type,
        "VARCHAR(255)"
    );
    assert_eq!(
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[1].sql_type,
        "CHAR(8)"
    );
    assert_eq!(
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[2].sql_type,
        "VARCHAR(64)"
    );
    let drizzle::core::ColumnDialect::PostgreSQL { dimensions, .. } =
        <BoundedCharacters as drizzle::core::DrizzleTable>::TABLE_REF.columns[2].dialect
    else {
        panic!("expected PostgreSQL column metadata");
    };
    assert_eq!(dimensions, Some(1));
    assert_eq!(
        <BoundedCharacterView as drizzle::core::DrizzleTable>::TABLE_REF.columns[0].sql_type,
        "VARCHAR(32)"
    );
}

#[test]
fn postgres_dialect_types_are_distinct_markers_with_cast_mappings() {
    fn assert_target<T: core_types::DataType, C: CastTarget<'static, T, PostgresDialect>>(_: C) {}

    let _ = postgres_types::Int2;
    let _ = postgres_types::Int4;
    let _ = postgres_types::Int8;
    let _ = postgres_types::Float4;
    let _ = postgres_types::Float8;
    let _ = postgres_types::Varchar;
    let _ = postgres_types::Bytea;
    let _ = postgres_types::Boolean;
    let _ = postgres_types::Timestamptz;

    assert_target::<postgres_types::Int2, _>(postgres_types::Int2);
    assert_target::<postgres_types::Int4, _>(postgres_types::Int4);
    assert_target::<postgres_types::Int8, _>(postgres_types::Int8);
    assert_target::<postgres_types::Float4, _>(postgres_types::Float4);
    assert_target::<postgres_types::Float8, _>(postgres_types::Float8);
    assert_target::<postgres_types::Varchar, _>(postgres_types::Varchar);
    assert_target::<postgres_types::Bytea, _>(postgres_types::Bytea);
    assert_target::<postgres_types::Boolean, _>(postgres_types::Boolean);
    assert_target::<postgres_types::Timestamptz, _>(postgres_types::Timestamptz);

    fn assert_compatible<S, T>()
    where
        S: core_types::DataType + core_types::Compatible<T>,
        T: core_types::DataType,
    {
    }

    // Postgres numeric widening
    assert_compatible::<postgres_types::Int2, postgres_types::Int4>();
    assert_compatible::<postgres_types::Int4, postgres_types::Int8>();
    assert_compatible::<postgres_types::Float4, postgres_types::Float8>();
    // Cross int/float
    assert_compatible::<postgres_types::Int4, postgres_types::Float8>();
    // Numeric compat
    assert_compatible::<postgres_types::Numeric, postgres_types::Int4>();
    // Text variants
    assert_compatible::<postgres_types::Text, postgres_types::Varchar>();
    // Temporal
    assert_compatible::<postgres_types::Timestamp, postgres_types::Timestamptz>();
}
