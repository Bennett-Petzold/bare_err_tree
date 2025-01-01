#![cfg(all(
    not(feature = "tracing"),
    feature = "derive_alloc",
    feature = "source_line"
))]

use std::println;

mod example {
    include!("../test_cases/std/src/bin/example.rs");

    #[test]
    fn readme_example() {
        let expected_lines = "missed class
├─ at bare_err_tree/tests/../test_cases/std/src/bin/example.rs:26:6
│
╰─▶ stayed in bed too long
    ├─ at bare_err_tree/tests/../test_cases/std/src/bin/example.rs:18:57
    │
    ├─▶ bed is comfortable
    │
    ╰─▶ went to sleep at 2 A.M.
        ├─ at bare_err_tree/tests/../test_cases/std/src/bin/example.rs:18:72
        │
        ├─▶ finishing a project
        │   │
        │   ╰─▶ proving 1 == 2
        │
        ├─▶ stressed about exams
        │
        ╰─▶ playing video games";

        assert_eq!(gen_print(), expected_lines);
    }
}

mod near_empty {
    include!("../test_cases/std/src/bin/near-empty.rs");

    #[test]
    fn near_empty() {
        let expected_lines = "EMPTY
╰─ at bare_err_tree/tests/../test_cases/std/src/bin/near-empty.rs:16:17";

        assert_eq!(gen_print(), expected_lines);
    }
}