use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

#[allow(dead_code)]
fn must_pass(glob: &str) {
    let t = trybuild::TestCases::new();
    t.pass(glob);
}

#[allow(dead_code)]
fn collect_files(glob_pattern: &str) -> Vec<PathBuf> {
    let mut files = glob::glob(glob_pattern)
        .unwrap_or_else(|err| panic!("invalid glob pattern `{glob_pattern}`: {err}"))
        .map(|entry| {
            entry.unwrap_or_else(|err| {
                panic!("failed to read glob entry for `{glob_pattern}`: {err}")
            })
        })
        .collect::<Vec<_>>();
    files.sort();
    assert!(
        !files.is_empty(),
        "no files matched compile-fail pattern `{glob_pattern}`"
    );
    files
}

#[allow(dead_code)]
fn must_fail(glob_pattern: &str) {
    for file in collect_files(glob_pattern) {
        let file = file.to_string_lossy().replace('\\', "/");
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let t = trybuild::TestCases::new();
            t.pass(&file);
        }));

        assert!(
            outcome.is_err(),
            "expected `{file}` to fail compilation, but it compiled successfully"
        );
    }
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

#[cfg(feature = "rusqlite")]
#[test]
fn sqlite_strict_affinity_ui() {
    must_pass("tests/ui/sqlite_strict_affinity/pass/*.rs");
    must_fail("tests/ui/sqlite_strict_affinity/fail/*.rs");
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
