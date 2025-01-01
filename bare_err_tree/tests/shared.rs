#![cfg(feature = "derive_alloc")]

use std::println;

mod empty {
    include!("../../testing/src/bin/empty.rs");

    #[test]
    fn empty() {
        let expected_lines = "EMPTY";
        assert_eq!(gen_print(), expected_lines);
    }
}

mod near_empty {
    include!("../../testing/src/bin/near-empty.rs");

    #[test]
    fn near_empty() {
        let expected_lines = "EMPTY
╰─ at bare_err_tree/tests/../../testing/src/bin/near-empty.rs:16:17";

        assert_eq!(gen_print(), expected_lines);
    }
}
