//! Compile-fail tests for type safety.
//!
//! These tests verify that type mismatches are caught at compile time.

#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
