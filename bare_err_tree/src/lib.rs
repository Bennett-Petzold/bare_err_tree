/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*!
`bare_err_tree` is a `no_std` library to print an [`Error`] with a tree of sources.

The functionality introduced by this library does not change the type or public
API beyond a hidden field (when implemented on structs) or deref (when
implemented via a wrapper).
It is added via macro or manual implementation of [`AsErrTree`].
End users can then use [`tree_unwrap`] or [`print_tree`] to get better error output.


If none of the [tracking feature flags](#tracking-feature-flags) are enabled,
the metadata is set to the [`unit`] type to take zero space.
If the print methods are never called, and none of the tracking features are
enabled, this library incurs zero runtime cost.
Usage of the [`err_tree`] macro incurs a compliation time cost.

# Feature Flags
* `derive`: Enabled by default, provides [`err_tree`] via proc macro.
* `derive_alloc`: Allows derive to generate allocating code (e.g. for `Vec`
    sources).
* `heap_buffer`: Uses heap to store leading arrows so that `FRONT_MAX` bytes of
    the stack aren't statically allocated for this purpose.
* `unix_color`: Outputs UNIX console codes for emphasis.
* `anyhow`: Adds implementation for [`anyhow::Error`].
* `eyre`: Adds implementation for [`eyre::Report`].
* `json`: Allows for storage to/reconstruction from JSON. Uses allocation due
    to [`serde_json`] internals.
#### Tracking Feature Flags
* `source_line`: Tracks the source line of tree errors.
* `tracing`: Produces a `tracing` backtrace with [`tracing_error`]. Uses
    allocation to format prints.
* `boxed_tracing`: Same as `tracing`, but boxes the trace.

# Adding [`ErrTree`] Support (Library or Bin)
Both libraries and binaries can add type support for [`ErrTree`] prints.
The [`err_tree`] macro is recommended, but [`ErrTree`] allows for a manual
implementation.

#### Feature Flags in Libraries
Libraries should NOT enable any of the
[tracking feature flags](#tracking-feature-flags) by default. Those are tunable
for a particular binary's environment and needs. [`tree_unwrap`]/[`print_tree`]
should be used sparingly within the library, ideally with a small `FRONT_MAX`
to minimize out of stack memory errors.

# Using [`AsErrTree`] Implementors (Bin)
Specify desired tracking features by importing `bare_err_tree` in `Cargo.toml`.
(e.g. `bare_err_tree = { version = "*", features = ["source_line"] }`)

Call [`tree_unwrap`] on the [`Result`] or [`print_tree`] on the [`Error`] with
`FRONT_MAX` set to `6 * (maximum tree depth)`. Note that unless `heap_buffer`
is enabled, `FRONT_MAX` bytes will always be occupied on stack for the duration
of a print call. Make sure this falls within platform stack size, and single
stack frame size, limits.

# Credit

The formatting is borrowed from from [error-stack](https://crates.io/crates/error-stack).
Please see the [contributors page](https://github.com/hashintel/hash/graphs/contributors) for appropriate credit.

# Licensing and Contributing

All code is licensed under MPL 2.0. See the [FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)
for license questions. The license is non-viral copyleft and does not block this library from
being used in closed-source codebases. If you are using this library for a commercial purpose,
consider reaching out to `dansecob.dev@gmail.com` to make a financial contribution.

Contributions are welcome at
<https://github.com/Bennett-Petzold/bare_err_tree>.
*/

#![no_std]

#[cfg(any(feature = "heap_buffer", feature = "tracing", feature = "json"))]
extern crate alloc;

#[cfg(feature = "source_line")]
use core::panic::Location;

use core::{
    borrow::Borrow,
    error::Error,
    fmt::{self},
};

mod pkg;
pub use pkg::*;
mod fmt_logic;
use fmt_logic::*;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::*;

#[cfg(feature = "derive")]
pub use bare_err_tree_proc::*;

/// Alternative to [`Result::unwrap`] that formats the error as a tree.
///
/// `FRONT_MAX` limits the number of leading bytes. Each deeper error requires 6
/// bytes to fit "│   ". So for a max depth of 3 errors, `FRONT_MAX` == 18.
/// By default, `FRONT_MAX` bytes are allocated on stack. When `heap_buffer` is
/// enabled, the bytes are allocated on heap and `FRONT_MAX` only acts as a
/// depth limit.
///
/// Errors must define [`Error::source`] correctly for the tree to display.
/// The derive macros for [`ErrTree`] track extra information and handle
/// multiple sources ([`Error::source`] is designed around a single error
/// source).
#[track_caller]
pub fn tree_unwrap<const FRONT_MAX: usize, T, E, S>(res: Result<T, S>) -> T
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
{
    match res {
        Ok(x) => x,
        Err(tree) => {
            let loc = core::panic::Location::caller();
            tree.borrow().as_err_tree(&mut |tree| {
                panic!(
                    "Panic origin at: {:#?}\n{}",
                    loc,
                    ErrTreeFmtWrap::<FRONT_MAX, _> { tree }
                )
            });
            unreachable!()
        }
    }
}

/// Produces [`ErrTree`] formatted output for an error.
///
/// `FRONT_MAX` limits the number of leading bytes. Each deeper error requires 6
/// bytes to fit "│   ". So for a max depth of 3 errors, `FRONT_MAX` == 18.
/// By default, `FRONT_MAX` bytes are allocated on stack. When `heap_buffer` is
/// enabled, the bytes are allocated on stack and `FRONT_MAX` only acts as a
/// depth limit.
///
/// Errors must define [`Error::source`] correctly for the tree to display.
/// The derive macros for [`ErrTree`] track extra information and handle
/// multiple sources ([`Error::source`] is designed around a single error
/// source).
#[track_caller]
pub fn print_tree<const FRONT_MAX: usize, E, S, F>(
    tree: S,
    formatter: &mut F,
) -> Result<(), fmt::Error>
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
    F: fmt::Write,
{
    let mut res = Ok(());
    tree.borrow().as_err_tree(&mut |tree| {
        res = write!(formatter, "{}", ErrTreeFmtWrap::<FRONT_MAX, _> { tree });
    });
    res
}

/// Intermediate struct for printing created by [`AsErrTree`].
///
/// Only allowing construction through [`Self::with_pkg`] and [`Self::no_pkg`]
/// allows arbitrary combinations of metadata tracking without changing
/// construction syntax. Sources are stored under three layers of indirection
/// to allow for maximum type and size flexibility without generics or heap
/// allocation.
///
/// See [`tree`] to reduce [`Self::with_pkg`] boilerplate.
///
/// # Manual Implementation Example
/// ```
/// # use std::{
/// #   panic::Location,
/// #   error::Error,
/// #   fmt::{Display, Formatter},
/// # };
/// use bare_err_tree::{ErrTree, ErrTreePkg, AsErrTree};
///
/// #[derive(Debug)]
/// pub struct HighLevelIo {
///     source: std::io::Error,
///     _pkg: ErrTreePkg,
/// }
///
/// impl HighLevelIo {
///     #[track_caller]
///     pub fn new(source: std::io::Error) -> Self {
///         Self {
///             source,
///             _pkg: ErrTreePkg::new(),
///         }
///     }
/// }
///
/// impl AsErrTree for HighLevelIo {
///     fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
///         // Cast to AsErrTree via Error
///         let source = &(&self.source as &dyn Error) as &dyn AsErrTree;
///
///         // Call the formatting function
///         (func)(ErrTree::with_pkg(self, &[&[source]], &self._pkg));
///     }
/// }
///
/// impl Error for HighLevelIo {
///     fn source(&self) -> Option<&(dyn Error + 'static)> {
///         Some(&self.source)
///     }
/// }
/// impl Display for HighLevelIo {
///     # /*
///     ...
///     # */
///     # fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
///         # write!(f, "High level IO error!")
///     # }
/// }
/// ```
#[derive(Clone)]
pub struct ErrTree<'a> {
    inner: &'a dyn Error,
    sources: &'a [&'a [&'a dyn AsErrTree]],
    #[cfg(feature = "source_line")]
    location: Option<&'a Location<'a>>,
    #[cfg(feature = "tracing")]
    trace: Option<&'a tracing_error::SpanTrace>,
}

impl<'a> ErrTree<'a> {
    /// Common constructor, with metadata.
    pub fn with_pkg(
        inner: &'a dyn Error,
        sources: &'a [&[&dyn AsErrTree]],
        #[cfg_attr(
            not(feature = "source_line"),
            expect(unused, reason = "should be null when no tracking is enabled")
        )]
        pkg: &'a ErrTreePkg,
    ) -> Self {
        Self {
            inner,
            sources,
            #[cfg(feature = "source_line")]
            location: Some(pkg.location),
            #[cfg(all(feature = "tracing", not(feature = "boxed_tracing")))]
            trace: Some(&pkg.trace),
            #[cfg(feature = "boxed_tracing")]
            trace: Some(&*pkg.trace),
        }
    }

    /// Constructor for when metadata needs to be hidden.
    pub fn no_pkg(inner: &'a dyn Error, sources: &'a [&[&dyn AsErrTree]]) -> Self {
        Self {
            inner,
            sources,
            #[cfg(feature = "source_line")]
            location: None,
            #[cfg(feature = "tracing")]
            trace: None,
        }
    }

    pub fn sources(&self) -> &[&[&dyn AsErrTree]] {
        self.sources
    }
}

/// Defines an [`Error`]'s temporary view as an [`ErrTree`] for printing.
///
/// This can be defined with [`err_tree`], manually (see [`ErrTree`]), or with
/// the default `dyn` implementation. The `dyn` implementation does not track
/// any more information than standard library errors or track multiple sources.
///
/// Implementors must call `func` with a properly constructed [`ErrTree`].
pub trait AsErrTree {
    /// Constructs the [`ErrTree`] internally and calls `func` on it.
    fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>));
}

/// Displays with [`Error::source`] as the child.
///
/// Does not provide any of the extra tracking information or handle multiple
/// sources.
impl AsErrTree for dyn Error {
    fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
        match self.source() {
            Some(e) => (func)(ErrTree::no_pkg(self, &[&[&e as &dyn AsErrTree]])),
            None => (func)(ErrTree::no_pkg(self, &[])),
        }
    }
}

#[cfg(feature = "anyhow")]
impl AsErrTree for anyhow::Error {
    fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
        let this: &dyn Error = self.as_ref();
        this.as_err_tree(func)
    }
}

#[cfg(feature = "eyre")]
impl AsErrTree for eyre::Report {
    fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
        let this: &dyn Error = self.as_ref();
        this.as_err_tree(func)
    }
}

impl<T: ?Sized + AsErrTree> AsErrTree for &T {
    fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
        T::as_err_tree(self, func)
    }
}

/// Boilerplate reducer for manual [`ErrTree`].
///
/// Expands out to [`ErrTree::with_pkg`] with `$x` as source(s).
/// Preface with `dyn` to use the generic `dyn` [`Error`] rendering.
///
/// ```
/// # use std::{
/// #   panic::Location,
/// #   error::Error,
/// #   fmt::{Display, Formatter},
/// # };
/// use bare_err_tree::{tree, ErrTree, ErrTreePkg, AsErrTree};
///
/// #[derive(Debug)]
/// struct Foo(std::io::Error, ErrTreePkg);
///
/// impl AsErrTree for Foo {
///     fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
///         // Equivalent to:
///         // (func)(bare_err_tree::ErrTree::with_pkg(
///         //     &self,
///         //     &[&[&(&self.0 as &dyn Error) as &dyn AsErrTree,]]
///         //     self.1.clone()
///         // )
///         tree!(dyn, func, self, self.1, &self.0)
///     }
/// }
///
/// impl Error for Foo {
///     fn source(&self) -> Option<&(dyn Error + 'static)> {
///         Some(&self.0)
///     }
/// }
/// impl Display for Foo {
///     # /*
///     ...
///     # */
///     # fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
///         # write!(f, "")
///     # }
/// }
/// ```
#[macro_export]
macro_rules! tree {
    (dyn, $func:expr, $inner:expr, $pkg:expr, $( $x:expr ),* ) => {
        ($func)(bare_err_tree::ErrTree::with_pkg(
            &$inner,
            &[&[ $( &( $x as &dyn core::error::Error ) as &dyn bare_err_tree::AsErrTree , )* ]],
            &$pkg,
        ))
    };
    ($func:expr, $inner:expr, $pkg:expr, $( $x:expr ),* ) => {
        ($func)(bare_err_tree::ErrTree::with_pkg(
            &$inner,
            &[&[ $( $x as &dyn bare_err_tree::AsErrTree , )* ]],
            &$pkg,
        ))
    };
}
