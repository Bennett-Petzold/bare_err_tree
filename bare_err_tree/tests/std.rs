#![cfg(all(
    not(feature = "tracing"),
    not(feature = "source_line"),
    feature = "derive_alloc"
))]

include!("../../testing/src/bin/example.rs");

#[test]
fn example() {}
