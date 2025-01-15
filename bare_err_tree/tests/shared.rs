#![cfg(not(feature = "unix_color"))]

#[cfg(feature = "derive")]
mod empty {
    include!("../test_cases/std/src/bin/empty.rs");

    #[test]
    fn empty() {
        let expected_lines = "EMPTY";
        assert_eq!(gen_print(), expected_lines);
    }
}

#[cfg(all(feature = "derive", feature = "source_line"))]
mod near_empty {
    include!("../test_cases/std/src/bin/near-empty.rs");

    #[test]
    fn near_empty() {
        let expected_lines = "EMPTY
╰─ at bare_err_tree/tests/../test_cases/std/src/bin/near-empty.rs:17:17";

        assert_eq!(gen_print(), expected_lines);
    }
}

mod multiline {
    use core::error::Error;

    use bare_err_tree::print_tree;
    use thiserror::Error;

    #[derive(Debug, Error)]
    #[error("And is\nnested")]
    struct InnerMultiline;

    #[derive(Debug, Error)]
    #[error("This error spans\nmultiple\nlines")]
    struct MultilineErr(#[source] InnerMultiline);

    #[test]
    fn multiline() {
        let expected_lines = "This error spans
│ multiple
│ lines
│
╰─▶ And is
    │ nested";

        let err = MultilineErr(InnerMultiline);
        let mut out = String::new();
        print_tree::<60, _, _>(&err as &dyn Error, &mut out).unwrap();

        assert_eq!(out, expected_lines);
    }
}
