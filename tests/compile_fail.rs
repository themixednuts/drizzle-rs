#[cfg(feature = "rusqlite")]
#[test]
fn strict_decode_ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/strict_decode/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn cast_targets_sqlite_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/cast_targets/sqlite/pass/*.rs");
    t.compile_fail("tests/ui/cast_targets/sqlite/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn cast_targets_postgres_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/cast_targets/postgres/pass/*.rs");
    t.compile_fail("tests/ui/cast_targets/postgres/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn raw_sql_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/raw_sql/pass/*.rs");
    t.compile_fail("tests/ui/raw_sql/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn aggregate_types_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/aggregate_types/pass/*.rs");
    t.compile_fail("tests/ui/aggregate_types/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn aggregate_types_postgres_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/aggregate_types_postgres/pass/*.rs");
    t.compile_fail("tests/ui/aggregate_types_postgres/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn scalar_types_sqlite_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/scalar_types_sqlite/pass/*.rs");
    t.compile_fail("tests/ui/scalar_types_sqlite/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn scalar_types_postgres_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/scalar_types_postgres/pass/*.rs");
    t.compile_fail("tests/ui/scalar_types_postgres/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn join_nullability_postgres_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/join_nullability_postgres/pass/*.rs");
    t.compile_fail("tests/ui/join_nullability_postgres/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn set_ops_sqlite_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/set_ops_sqlite/pass/*.rs");
    t.compile_fail("tests/ui/set_ops_sqlite/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn subquery_types_sqlite_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/subquery_types_sqlite/pass/*.rs");
    t.compile_fail("tests/ui/subquery_types_sqlite/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn set_ops_postgres_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/set_ops_postgres/pass/*.rs");
    t.compile_fail("tests/ui/set_ops_postgres/fail/*.rs");
}

#[cfg(feature = "postgres")]
#[test]
fn no_widening_postgres_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/no_widening_postgres/pass/*.rs");
    t.compile_fail("tests/ui/no_widening_postgres/fail/*.rs");
}

#[cfg(feature = "rusqlite")]
#[test]
fn sqlite_strict_affinity_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/sqlite_strict_affinity/pass/*.rs");
    t.compile_fail("tests/ui/sqlite_strict_affinity/fail/*.rs");
}
