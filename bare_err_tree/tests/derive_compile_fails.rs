#![cfg(feature = "derive")]

use trybuild::TestCases;

#[test]
fn derive_example() {
    TestCases::new().pass("test_cases/std/src/bin/derive_testing.rs");
}

#[cfg(not(any(feature = "anyhow", feature = "eyre")))]
#[test]
fn false_tree_defs() {
    TestCases::new().compile_fail("test_cases/std/fail_src/false_tree*.rs");
}

#[test]
fn container_as_err() {
    TestCases::new().compile_fail("test_cases/std/fail_src/container.rs");
}

#[test]
fn single_as_err() {
    TestCases::new().compile_fail("test_cases/std/fail_src/single.rs");
}

#[test]
fn early_clone_derive() {
    TestCases::new().compile_fail("test_cases/std/fail_src/early_clone_derive.rs");
}

#[test]
fn direct_enum() {
    TestCases::new().compile_fail("test_cases/std/fail_src/direct_enum.rs");
}

#[test]
fn direct_unit() {
    TestCases::new().compile_fail("test_cases/std/fail_src/direct_union.rs");
}
