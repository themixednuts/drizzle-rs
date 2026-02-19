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
fn sqlite_strict_affinity_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/sqlite_strict_affinity/pass/*.rs");
    t.compile_fail("tests/ui/sqlite_strict_affinity/fail/*.rs");
}
