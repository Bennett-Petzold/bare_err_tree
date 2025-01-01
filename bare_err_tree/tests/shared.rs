#![cfg(all(feature = "derive_alloc", not(feature = "unix_color")))]

mod empty {
    include!("../test_cases/std/src/bin/empty.rs");

    #[test]
    fn empty() {
        let expected_lines = "EMPTY";
        assert_eq!(gen_print(), expected_lines);
    }
}

#[cfg(feature = "source_line")]
mod near_empty {
    include!("../test_cases/std/src/bin/near-empty.rs");

    #[test]
    fn near_empty() {
        let expected_lines = "EMPTY
╰─ at bare_err_tree/tests/../test_cases/std/src/bin/near-empty.rs:17:17";

        assert_eq!(gen_print(), expected_lines);
    }
}
