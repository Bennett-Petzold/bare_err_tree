#![cfg(all(
    feature = "tracing",
    feature = "derive_alloc",
    feature = "source_line",
    not(feature = "unix_color")
))]

mod example {
    use std::assert_eq;

    include!("../test_cases/trace/src/bin/trace_example.rs");

    #[test]
    fn readme_example() {
        let expected_lines = r#"missed class
├─ at bare_err_tree/tests/../test_cases/trace/src/bin/trace_example.rs:51:6
│
├─ tracing frame 0 => tracing::example::gen_print_inner
│        at bare_err_tree/tests/../test_cases/trace/src/bin/trace_example.rs:38
│
╰─▶ stayed in bed too long
    ├─ at bare_err_tree/tests/../test_cases/trace/src/bin/trace_example.rs:40:57
    │
    ├─ tracing frame 1 => tracing::example::new with
    │    bed_time=BedTime {
    │      hour: 2,
    │      reasons: [
    │        FinishingProject(
    │          ClassProject {
    │            desc: "proving 1 == 2"
    │          }
    │        ),
    │        ExamStressed,
    │        PlayingGames
    │      ]
    │    }
    │    _garbage=5
    │        at bare_err_tree/tests/../test_cases/trace/src/bin/trace_example.rs:124
    ├─ 1 duplicate tracing frame(s): [0]
    │
    ├─▶ bed is comfortable
    │
    ╰─▶ went to sleep at 2 A.M.
        ├─ at bare_err_tree/tests/../test_cases/trace/src/bin/trace_example.rs:41:9
        │
        ├─ 1 duplicate tracing frame(s): [0]
        │
        ├─▶ finishing a project
        │   │
        │   ╰─▶ proving 1 == 2
        │
        ├─▶ stressed about exams
        │
        ╰─▶ playing video games"#;

        assert_eq!(gen_print(), expected_lines);
    }
}

mod near_empty {
    include!("../test_cases/trace/src/bin/near-empty.rs");

    #[test]
    fn near_empty() {
        let expected_lines = "EMPTY
╰─ at bare_err_tree/tests/../test_cases/trace/src/bin/near-empty.rs:34:17";

        assert_eq!(gen_print(), expected_lines);
    }
}
