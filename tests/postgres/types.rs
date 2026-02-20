use drizzle::core::expr::CastTarget;
use drizzle::core::types as core_types;
use drizzle::postgres::types as postgres_types;
use drizzle_core::dialect::PostgresDialect;

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

    fn assert_compatible<S: core_types::DataType, T: core_types::DataType>()
    where
        S: core_types::Compatible<T>,
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
