#![cfg(all(
    feature = "tracing",
    feature = "derive",
    feature = "source_line",
    feature = "json",
    not(feature = "unix_color")
))]

mod example {
    include!("../test_cases/json/src/bin/reconstruct.rs");

    #[test]
    fn readme_example() {
        let expected_json = r#"{"msg":"missed class","location":"bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:51:6","trace":[{"target":"json::example","name":"gen_print_inner","fields":"","source_loc":{"file":"bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs","line":38}}],"sources":[{"msg":"stayed in bed too long","location":"bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:40:57","trace":[{"target":"json::example","name":"new","fields":"bed_time=BedTime { hour: 2, reasons: [FinishingProject(ClassProject { desc: \"proving 1 == 2\" }), ExamStressed, PlayingGames] } _garbage=5","source_loc":{"file":"bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs","line":130}},{"target":"json::example","name":"gen_print_inner","fields":"","source_loc":{"file":"bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs","line":38}}],"sources":[{"msg":"bed is comfortable"},{"msg":"went to sleep at 2 A.M.","location":"bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:41:9","trace":[{"target":"json::example","name":"gen_print_inner","fields":"","source_loc":{"file":"bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs","line":38}}],"sources":[{"msg":"finishing a project","sources":[{"msg":"proving 1 == 2"}]},{"msg":"stressed about exams"},{"msg":"playing video games"}]}]}]}"#;

        let expected_lines = r#"missed class
├─ at bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:51:6
│
├─ tracing frame 0 => json::example::gen_print_inner
│        at bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:38
│
╰─▶ stayed in bed too long
    ├─ at bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:40:57
    │
    ├─ tracing frame 1 => json::example::new with
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
    │        at bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:130
    ├─ 1 duplicate tracing frame(s): [0]
    │
    ├─▶ bed is comfortable
    │
    ╰─▶ went to sleep at 2 A.M.
        ├─ at bare_err_tree/tests/../test_cases/json/src/bin/reconstruct.rs:41:9
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

        assert_eq!(gen_print(), expected_json);

        assert_eq!(reconstruct(&gen_print()), expected_lines);
    }
}

mod json_escapes {
    use core::{error::Error, fmt::Write};

    use bare_err_tree::{reconstruct_output, ErrTreeJson};
    use thiserror::Error;

    #[derive(Debug, Error)]
    #[error(
        "foo\n \\ \\n \t/
bar"
    )]
    struct WeirdError;

    #[test]
    fn handles_escapes() {
        let mut out = String::new();
        write!(out, "{}", ErrTreeJson((&WeirdError) as &dyn Error)).unwrap();

        let expected_json = r#"{"msg":"foo\n \\ \\n \t/\nbar"}"#;
        let expected_reconstruct = "foo\n│  \\ \\n \t/\n│ bar";

        assert_eq!(out, expected_json);

        let mut reconstructed = String::new();
        reconstruct_output::<60, _, _>(out, &mut reconstructed).unwrap();
        assert_eq!(reconstructed, expected_reconstruct);
    }
}
