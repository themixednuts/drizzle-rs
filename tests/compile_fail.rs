#[allow(dead_code)]
fn must_pass(glob: &str) {
    let t = trybuild::TestCases::new();
    t.pass(glob);
}

#[allow(dead_code)]
fn must_fail(glob: &str) {
    let t = trybuild::TestCases::new();
    t.compile_fail(glob);
}

#[cfg(feature = "mysql")]
#[test]
fn mysql_macros_ui() {
    must_pass("tests/ui/mysql_macros/pass/*.rs");
    must_fail("tests/ui/mysql_macros/fail/*.rs");
}

// Enabling relational queries exposes a second public `QueryBuilder`, which
// changes only rustc's path formatting for the dialect-builder diagnostics.
// CI runs this suite once on the native MySQL builder feature graph and runs
// the shared relational suite separately with `query` enabled.
#[cfg(all(feature = "mysql", not(feature = "query")))]
#[test]
fn mysql_builder_ui() {
    must_pass("tests/ui/mysql_builder/pass/*.rs");
    must_fail("tests/ui/mysql_builder/fail/*.rs");
}

#[cfg(feature = "mysql")]
#[test]
fn update_assignments_mysql_ui() {
    must_pass("tests/ui/update_assignments/mysql/pass/*.rs");
    must_fail("tests/ui/update_assignments/mysql/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn update_assignments_postgres_ui() {
    must_pass("tests/ui/update_assignments/postgres/pass/*.rs");
    must_fail("tests/ui/update_assignments/postgres/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn update_assignments_sqlite_ui() {
    must_pass("tests/ui/update_assignments/sqlite/pass/*.rs");
    must_fail("tests/ui/update_assignments/sqlite/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn derived_tables_ui() {
    must_fail("tests/ui/derived_tables/fail/*.rs");
}

#[test]
fn derived_left_lateral_ui() {
    must_fail("tests/ui/derived_tables/shared/fail/*.rs");
}

#[cfg(all(feature = "rusqlite", feature = "uuid"))]
#[test]
fn strict_decode_ui() {
    must_fail("tests/ui/strict_decode/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn cast_targets_sqlite_ui() {
    must_pass("tests/ui/cast_targets/sqlite/pass/*.rs");
    must_fail("tests/ui/cast_targets/sqlite/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn cast_targets_postgres_ui() {
    must_pass("tests/ui/cast_targets/postgres/pass/*.rs");
    must_fail("tests/ui/cast_targets/postgres/fail/*.rs");
}

#[cfg(all(feature = "rusqlite", feature = "uuid"))]
#[test]
fn raw_sql_ui() {
    must_pass("tests/ui/raw_sql/pass/*.rs");
    must_fail("tests/ui/raw_sql/fail/*.rs");
}

#[cfg(all(feature = "rusqlite", feature = "uuid"))]
#[test]
fn aggregate_types_ui() {
    must_pass("tests/ui/aggregate_types/pass/*.rs");
    must_fail("tests/ui/aggregate_types/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn aggregate_types_postgres_ui() {
    must_pass("tests/ui/aggregate_types_postgres/pass/*.rs");
    must_fail("tests/ui/aggregate_types_postgres/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn scalar_types_sqlite_ui() {
    must_pass("tests/ui/scalar_types_sqlite/pass/*.rs");
    must_fail("tests/ui/scalar_types_sqlite/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn scalar_types_postgres_ui() {
    must_pass("tests/ui/scalar_types_postgres/pass/*.rs");
    must_fail("tests/ui/scalar_types_postgres/fail/*.rs");
}

#[cfg(all(feature = "postgres", feature = "uuid"))]
#[test]
fn join_nullability_postgres_ui() {
    must_pass("tests/ui/join_nullability_postgres/pass/*.rs");
    must_fail("tests/ui/join_nullability_postgres/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn join_nullability_sqlite_ui() {
    must_pass("tests/ui/join_nullability_sqlite/pass/*.rs");
    must_fail("tests/ui/join_nullability_sqlite/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn set_ops_sqlite_ui() {
    must_pass("tests/ui/set_ops_sqlite/pass/*.rs");
    must_fail("tests/ui/set_ops_sqlite/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn subquery_types_sqlite_ui() {
    must_pass("tests/ui/subquery_types_sqlite/pass/*.rs");
    must_fail("tests/ui/subquery_types_sqlite/fail/*.rs");
}

#[cfg(all(feature = "postgres", feature = "uuid"))]
#[test]
fn set_ops_postgres_ui() {
    must_pass("tests/ui/set_ops_postgres/pass/*.rs");
    must_fail("tests/ui/set_ops_postgres/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn no_widening_postgres_ui() {
    must_pass("tests/ui/no_widening_postgres/pass/*.rs");
    must_fail("tests/ui/no_widening_postgres/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_enum_storage_ui() {
    must_fail("tests/ui/postgres_enum_storage/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn sqlite_strict_affinity_ui() {
    must_pass("tests/ui/sqlite_strict_affinity/pass/*.rs");
    must_fail("tests/ui/sqlite_strict_affinity/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn sqlite_macro_errors_ui() {
    must_fail("tests/ui/sqlite_macro_errors/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn sqlite_enum_storage_ui() {
    must_fail("tests/ui/sqlite_enum_storage/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn boolean_enforcement_ui() {
    must_pass("tests/ui/boolean_enforcement/pass/*.rs");
    must_fail("tests/ui/boolean_enforcement/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn boolean_enforcement_sqlite_ui() {
    must_pass("tests/ui/boolean_enforcement_sqlite/pass/*.rs");
    must_fail("tests/ui/boolean_enforcement_sqlite/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn view_query_sqlite_ui() {
    must_pass("tests/ui/view_query_sqlite/pass/*.rs");
    must_fail("tests/ui/view_query_sqlite/fail/*.rs");
}

#[cfg(all(feature = "rusqlite", feature = "query"))]
#[test]
fn query_api_sqlite_ui() {
    must_fail("tests/ui/query_api_sqlite/fail/*.rs");
}

#[cfg(all(feature = "postgres", feature = "query", feature = "uuid"))]
#[test]
fn query_api_postgres_ui() {
    must_pass("tests/ui/query_api_postgres/pass/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn pagination_sqlite_ui() {
    must_pass("tests/ui/pagination_sqlite/pass/*.rs");
    must_fail("tests/ui/pagination_sqlite/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn aggregate_mixing_sqlite_ui() {
    must_pass("tests/ui/aggregate_mixing_sqlite/pass/*.rs");
    must_fail("tests/ui/aggregate_mixing_sqlite/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn aggregate_propagation_ui() {
    must_pass("tests/ui/aggregate_propagation/pass/*.rs");
    must_fail("tests/ui/aggregate_propagation/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn aggregate_propagation_postgres_ui() {
    must_pass("tests/ui/aggregate_propagation_postgres/pass/*.rs");
    must_fail("tests/ui/aggregate_propagation_postgres/fail/*.rs");
}
