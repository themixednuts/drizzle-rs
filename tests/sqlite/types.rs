use drizzle::core::expr::CastTarget;
use drizzle::core::types as core_types;
use drizzle::sqlite::types as sqlite_types;
use drizzle_core::dialect::SQLiteDialect;

#[test]
fn sqlite_dialect_types_are_distinct_markers_with_cast_mappings() {
    fn assert_target<T: core_types::DataType, C: CastTarget<'static, T, SQLiteDialect>>(_: C) {}

    let _ = sqlite_types::Integer;
    let _ = sqlite_types::Real;
    let _ = sqlite_types::Blob;

    assert_target::<sqlite_types::Integer, _>(sqlite_types::Integer);
    assert_target::<sqlite_types::Real, _>(sqlite_types::Real);
    assert_target::<sqlite_types::Blob, _>(sqlite_types::Blob);

    fn assert_compatible<S: core_types::DataType, T: core_types::DataType>()
    where
        S: core_types::Compatible<T>,
    {
    }

    assert_compatible::<sqlite_types::Integer, sqlite_types::Real>();
    assert_compatible::<sqlite_types::Real, sqlite_types::Integer>();
    assert_compatible::<sqlite_types::Blob, sqlite_types::Text>();
}
