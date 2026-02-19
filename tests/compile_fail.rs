#![cfg(feature = "rusqlite")]

#[test]
fn strict_decode_ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/strict_decode/*.rs");
}
