use drizzle::core::expr::CastTarget;
use drizzle::core::types as core_types;
use drizzle::sqlite::types as sqlite_types;

#[test]
fn sqlite_dialect_types_are_distinct_markers_with_cast_mappings() {
    fn assert_target<T: core_types::DataType, C: CastTarget<'static, T>>(_: C) {}

    let _ = sqlite_types::Integer;
    let _ = sqlite_types::Real;
    let _ = sqlite_types::Blob;

    assert_target::<core_types::BigInt, _>(sqlite_types::Integer);
    assert_target::<core_types::Double, _>(sqlite_types::Real);
    assert_target::<core_types::Bytes, _>(sqlite_types::Blob);
}
