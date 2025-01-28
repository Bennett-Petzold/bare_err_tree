[![Crate][CrateStatus]][Crate]
[![Tests][TestsStatus]][Tests]
[![Docs][PagesStatus]][Docs]
[![Coverage][Coverage]][CoveragePages]

# bare\_err\_tree
`bare_err_tree` is a `no_std` + no `alloc` library to print a standard `Error` with a tree of sources.

Support for the extra information prints does not change the type or public API (besides a hidden field or deref).
It is added via macro or manual implementation of the `AsErrTree` trait (see
the [docs][Docs] for details).
End users can then use `ErrTreeDisplay` or `tree_unwrap` to get better error output,
or store as [JSON][JSON] for later reconstruction.

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

The `json` feature is currently the best way to store/transmit error logs.
It does not use heap allocation to allow usage on all embedded systems.
The JSON output heavily repeats keys and does not deduplicate traces: compression
(e.g. [zstd][zstd-rs]) is strongly recommended for storage.

The formatting is borrowed from from [error-stack][ErrorStack].
Please see the [contributors page](https://github.com/hashintel/hash/graphs/contributors) for appropriate credit.

# Policies
See [CONTRIBUTING.md][Contributing] for development specific guidance/rules.

## No std/alloc Core
The library, built with default features, never uses `extern crate std` or `extern crate alloc`.
Any features that enable either note this in the [crate root][clib].

## Version Compatibility
[Semantic versioning][semver] is accurately followed.
Libraries are expected to use depend on the major release (e.g. `version = "1"`)
with at most the default features.
This allows binaries to unify and control the `bare_err_tree` feature flags for
all their dependencies.
New features are a minor release.

## MSRV
The `rust-version` is specified in the workspace [Cargo.toml][ctoml] and must be accurate. It can be bumped up to the latest standard rust version whenever a feature is useful. Utilize [Cargo dependecy resolution][cdeps] if you use this library

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

[Contributing]: CONTRIBUTING.md
[ctoml]: Cargo.toml
[cdeps]: https://doc.rust-lang.org/cargo/reference/resolver.html
[clib]: bare_err_tree/src/lib.rs
[semver]: https://semver.org/

[ErrorStack]: https://crates.io/crates/error-stack
[Eyre]: https://crates.io/crates/eyre
[Anyhow]: https://crates.io/crates/anyhow

[zstd-rs]: https://crates.io/crates/zstd
[JSON]: https://www.json.org/json-en.html
