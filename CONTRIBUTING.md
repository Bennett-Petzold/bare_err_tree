# bare\_err\_tree Contributing
Contributions to this library are welcome!
See the [policies][Policies] for consumer facing rules.

## How
Communicate over GitHub issues and pull requests.
Draft pull requests are suggested to claim and avoid duplication of work.
Usually, you will want to open an issue first for discussion.

### CI
There are automated tests on GitHub CI that must be passed before merge.
They are intended to be comprehensive (feel free to contribute more tests).
Code coverage is for reviewer information, not a guideline.

## Style
Standard clippy lints are used.
Code should be clear to read and documented as appropriate both internally and
externally.
Specific standards are loose -- don't bother documenting getters.

## Dependencies
Default dependencies are deliberately minimal.
Any dependencies that are introduced should be specified as a major version.
Non-optional dependencies must provide necessary functionality for the library
that cannot be easily written into this library itself (e.g. requires unsafe
code blocks).
Optional dependencies for a feature such as `tracing` should still be
minimized.

## Public Documentation
All features should be documented in the [library root][clib].
Downstream users should be able to use this library by looking only at the
rustdocs, without any knowledge of the source code.

## Licensing
This library is licensed under MPL 2.0.
The license header must be on all source files.

## Forking
Ideally all debate can come to a reasonable consensus, but forking a new
project from this is encouraged if there is an intractable disagreement.

[Policies]: README.md#Policies
[clib]: bare_err_tree/src/lib.rs
