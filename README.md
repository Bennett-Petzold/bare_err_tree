[![Crate][CrateStatus]][Crate]
[![Tests][TestsStatus]][Tests]
[![Docs][PagesStatus]][Docs]

# bare\_err\_tree
`bare_err_tree` is a `no_std` library to print a standard `Error` with a tree of sources.

Support for the extra information prints does not change the type or public API (besides a hidden field or deref).
It is added via macro or manual implementation of the `AsErrTree` trait (see
the [docs][Docs] for details).
End users can then use `tree_unwrap` or `print_tree` to get better error output,
or store as JSON for later reconstruction.

Unlike [anyhow][Anyhow], [eyre][Eyre], or [error-stack][ErrorStack], the extra
functionality does not require exposing a special type in a library's API.
A library can use `bare_err_tree` for its errors without changing any types\*,
and library users can ignore the the existence of this crate entirely\*. There is
also support for including any implementor of `Error` in the tree with less
information, so it works on top of std and other libraries.

\* Users will see this crate/types will be modified, minimally, when:
* Struct fields are public
    * The necessary pkg field does not block any functionality. When derived it will be hidden.
* The error is an enum
    * The macro only supports creating a struct that wraps the enum as transparently as possible.
    * Direct implementation on an enum is possible, but likely pretty clunky.

The formatting is borrowed from from [error-stack][ErrorStack].
Please see the [contributors page](https://github.com/hashintel/hash/graphs/contributors) for appropriate credit.

# Example Output (source\_line)
Generate with `cd bare_err_tree/test_cases/std; cargo run --bin example`.
```
missed class
├─ at src/bin/example.rs:26:6
│
╰─▶ stayed in bed too long
    ├─ at src/bin/example.rs:18:57
    │
    ├─▶ bed is comfortable
    │
    ╰─▶ went to sleep at 2 A.M.
        ├─ at src/bin/example.rs:18:72
        │
        ├─▶ finishing a project
        │   │
        │   ╰─▶ proving 1 == 2
        │
        ├─▶ stressed about exams
        │
        ╰─▶ playing video games
```

# Example Output (source\_line + tracing)
Generate with `cd bare_err_tree/test_cases/trace; cargo run --bin trace_example`.
```
missed class
├─ at src/bin/trace_example.rs:51:6
│
├─ tracing frame 0 => trace_example::gen_print_inner
│        at src/bin/trace_example.rs:38
│
╰─▶ stayed in bed too long
    ├─ at src/bin/trace_example.rs:40:57
    │
    ├─ tracing frame 1 => trace_example::new with
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
    │        at src/bin/trace_example.rs:124
    ├─ 1 duplicate tracing frame(s): [0]
    │
    ├─▶ bed is comfortable
    │
    ╰─▶ went to sleep at 2 A.M.
        ├─ at src/bin/trace_example.rs:41:9
        │
        ├─ 1 duplicate tracing frame(s): [0]
        │
        ├─▶ finishing a project
        │   │
        │   ╰─▶ proving 1 == 2
        │
        ├─▶ stressed about exams
        │
        ╰─▶ playing video games
```

# Example Output (source\_line + tracing + json)
Generate with `cd bare_err_tree/test_cases/json; cargo run --bin example`.
```json
{"msg":"missed class","location":"src/bin/example.rs:51:6","trace":[{"target":"example","name":"gen_print_inner","fields":"","source_loc":["file":"src/bin/example.rs","line":38]}],"sources":[{"msg":"stayed in bed too long","location":"src/bin/example.rs:40:57","trace":[{"target":"example","name":"new","fields":"bed_time=BedTime { hour: 2, reasons: [FinishingProject(ClassProject { desc: \"proving 1 == 2\" }), ExamStressed, PlayingGames] } _garbage=5","source_loc":["file":"src/bin/example.rs","line":124]},{"target":"example","name":"gen_print_inner","fields":"","source_loc":["file":"src/bin/example.rs","line":38]}],"sources":[{"msg":"bed is comfortable"},{"msg":"went to sleep at 2 A.M.","location":"src/bin/example.rs:41:9","trace":[{"target":"example","name":"gen_print_inner","fields":"","source_loc":["file":"src/bin/example.rs","line":38]}],"sources":[{"msg":"finishing a project","sources":[{"msg":"proving 1 == 2"}]},{"msg":"stressed about exams"},{"msg":"playing video games"}]}]}]}
```

[CrateStatus]: https://img.shields.io/crates/v/bare_err_tree.svg
[Crate]: https://crates.io/crates/bare_err_tree
[TestsStatus]: https://github.com/Bennett-Petzold/bare_err_tree/actions/workflows/all-tests.yml/badge.svg?branch=main
[Tests]: https://github.com/Bennett-Petzold/bare_err_tree/actions/workflows/all-tests.yml
[PagesStatus]: https://github.com/Bennett-Petzold/bare_err_tree/actions/workflows/pages.yml/badge.svg?branch=main
[Docs]: https://bennett-petzold.github.io/bare_err_tree/docs/bare_err_tree/
[Coverage]: https://bennett-petzold.github.io/bare_err_tree/coverage/badge.svg
[CoveragePages]: https://bennett-petzold.github.io/bare_err_tree/coverage/

[ErrorStack]: https://crates.io/crates/error-stack
[Eyre]: https://crates.io/crates/eyre
[Anyhow]: https://crates.io/crates/anyhow
