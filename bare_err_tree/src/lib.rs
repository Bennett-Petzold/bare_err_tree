/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*!
`bare_err_tree` is a `no_std` and no `alloc` library to print an [`Error`] with a tree of sources.

Support for the extra information prints does not change the type or public
API (besides a hidden field or deref). It is added via macro or manual
implementation of the [`AsErrTree`] trait. End users can then use
[`tree_unwrap`] or [`print_tree`] to get better error output, or store as JSON
for later reconstruction.

If none of the [tracking feature flags](#tracking-feature-flags) are enabled,
the metadata is set to the [`unit`] type to take zero space.
If the print methods are never called, and none of the tracking features are
enabled, this library incurs zero runtime cost.
Usage of the [`err_tree`] macro incurs a compliation time cost.

# Feature Flags
* `derive`: Enabled by default, provides [`err_tree`] via proc macro.
* `json`: Allows for storage to/reconstruction from JSON.
* `heap_buffer`: Uses heap to store so state that `FRONT_MAX` (x3 if tracing
    is enabled) bytes of the stack aren't statically allocated for this purpose.
* `boxed`: Boxes the error package. Addresses ballooning from large tracking
    features. Boxing the error itself is likely more efficient, when available.
* `unix_color`: Outputs UNIX console codes for emphasis.
* `anyhow`: Adds implementation for [`anyhow::Error`].
* `eyre`: Adds implementation for [`eyre::Report`].
* `adapt`: Provides a [`std::io::Write`] adapter.
#### Tracking Feature Flags
* `source_line`: Tracks the source line of tree errors.
* `tracing`: Produces a `tracing` backtrace with [`tracing_error`].

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
is enabled, `FRONT_MAX` (x3 if `tracing` is enabled) bytes will be
occupied on stack for the duration of a print call. Make sure this falls
within platform stack size, and single stack frame size, limits.

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
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(feature = "adapt")]
extern crate std;

#[cfg(any(feature = "heap_buffer", feature = "boxed"))]
extern crate alloc;

#[cfg(feature = "source_line")]
use core::panic::Location;

use core::{
    error::Error,
    fmt::{self},
};

mod pkg;
pub use pkg::*;
pub mod flex;
pub use flex::*;
mod fmt_logic;
use fmt_logic::*;
mod buffer;
use buffer::*;

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
/// depth limit. When `tracing` is enabled, at most `FRONT_MAX` stack traces
/// will be tracked for duplicates.
///
/// Errors must define [`Error::source`] correctly for the tree to display.
/// The derive macros for [`ErrTree`] track extra information and handle
/// multiple sources ([`Error::source`] is designed around a single error
/// source).
#[track_caller]
pub fn tree_unwrap<const FRONT_MAX: usize, T, E>(res: Result<T, E>) -> T
where
    E: AsErrTree,
{
    match res {
        Ok(x) => x,
        Err(tree) => {
            let loc = core::panic::Location::caller();
            tree.as_err_tree(&mut |tree| {
                panic!(
                    "Panic origin at: {:#?}\n{}",
                    loc,
                    ErrTreeFmtWrap::<FRONT_MAX, _>::new(tree)
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
/// depth limit. When `tracing` is enabled, at most `FRONT_MAX` stack traces
/// will be tracked for duplicates.
///
/// Errors must define [`Error::source`] correctly for the tree to display.
/// The derive macros for [`ErrTree`] track extra information and handle
/// multiple sources ([`Error::source`] is designed around a single error
/// source).
///
/// ```rust
/// # use std::{
/// #   panic::Location,
/// #   error::Error,
/// #   fmt::{self, Write, Display, Formatter},
/// #   string::String,
/// #   io::self,
/// # };
/// use bare_err_tree::{AsErrTree, print_tree};
///
/// const PRINT_SIZE: usize = 60;
///
/// fn sized_print<E, F>(tree: E, formatter: F) -> fmt::Result
/// where
///     E: AsErrTree,
///     F: fmt::Write,
/// {
///     print_tree::<PRINT_SIZE, _, _>(tree, formatter)
/// }
///
/// fn io_as_tree() {
///     let mut out = String::new();
///     sized_print(&io::Error::last_os_error() as &dyn Error, &mut out).unwrap();
///     println!("{out}");
/// }
/// ```
#[track_caller]
pub fn print_tree<const FRONT_MAX: usize, E, F>(tree: E, mut formatter: F) -> fmt::Result
where
    E: AsErrTree,
    F: fmt::Write,
{
    let mut res = Ok(());
    tree.as_err_tree(&mut |tree| {
        res = fmt_tree::<FRONT_MAX, _, _>(tree, &mut formatter);
    });
    res
}

#[cfg(feature = "adapt")]
/// Converts [`std::io::Write`] to [`core::fmt::Write`].
///
/// Provided for using [`print_tree`] without a [`std::string::String`] buffer.
/// This adapter does not call [`flush`][`std::io::Write::flush`], only
/// [`write_all`][`std::io::Write::write_all`].
///
/// ```rust
/// # use std::{
/// #   panic::Location,
/// #   error::Error,
/// #   fmt::{self, Write, Display, Formatter},
/// #   io::{self, stdout},
/// # };
/// use bare_err_tree::{AdaptWrite, AsErrTree, print_tree};
///
/// const PRINT_SIZE: usize = 60;
///
/// fn sized_print<E, F>(tree: E, formatter: F) -> fmt::Result
/// where
///     E: AsErrTree,
///     F: fmt::Write,
/// {
///     print_tree::<PRINT_SIZE, _, _>(tree, formatter)
/// }
///
/// fn io_as_tree() {
///     let mut out = AdaptWrite(stdout());
///     sized_print(&io::Error::last_os_error() as &dyn Error, &mut out).unwrap();
///     out.flush().unwrap();
/// }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdaptWrite<W>(pub W);

#[cfg(feature = "adapt")]
impl<W> From<W> for AdaptWrite<W> {
    fn from(value: W) -> Self {
        Self(value)
    }
}

#[cfg(feature = "adapt")]
impl<W> core::fmt::Write for AdaptWrite<W>
where
    W: std::io::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[cfg(feature = "adapt")]
impl<W> AdaptWrite<W>
where
    W: std::io::Write,
{
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
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
/// use bare_err_tree::{ErrTree, ErrTreePkg, AsErrTree, WrapErr};
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
///         // Cast to ErrTree adapter via Error
///         let source = WrapErr::tree(&self.source);
///         // Convert to a single item iterator
///         let source_iter = &mut core::iter::once(source);
///
///         // Call the formatting function
///         (func)(ErrTree::with_pkg(self, source_iter, &self._pkg));
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
pub struct ErrTree<'a> {
    inner: &'a dyn Error,
    sources: IterBuffer<&'a mut dyn Iterator<Item = &'a dyn AsErrTree>>,
    #[cfg(feature = "source_line")]
    location: Option<&'a Location<'a>>,
    #[cfg(feature = "tracing")]
    trace: Option<&'a tracing_error::SpanTrace>,
}

impl<'a> ErrTree<'a> {
    /// Common constructor, with metadata.
    pub fn with_pkg(
        inner: &'a dyn Error,
        sources: &'a mut dyn Iterator<Item = &'a dyn AsErrTree>,
        #[allow(unused)] pkg: &'a ErrTreePkg,
    ) -> Self {
        Self {
            inner,
            sources: sources.into(),
            #[cfg(feature = "source_line")]
            location: Some(pkg.location()),
            #[cfg(feature = "tracing")]
            trace: Some(pkg.trace()),
        }
    }

    /// Constructor for when metadata needs to be hidden.
    pub fn no_pkg(
        inner: &'a dyn Error,
        sources: &'a mut dyn Iterator<Item = &'a dyn AsErrTree>,
    ) -> Self {
        Self {
            inner,
            sources: sources.into(),
            #[cfg(feature = "source_line")]
            location: None,
            #[cfg(feature = "tracing")]
            trace: None,
        }
    }

    /// Consumes this tree to return its sources
    pub fn sources(self) -> impl Iterator<Item = &'a dyn AsErrTree> {
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
            Some(e) => (func)(ErrTree::no_pkg(
                self,
                &mut core::iter::once(&e as &dyn AsErrTree),
            )),
            None => (func)(ErrTree::no_pkg(self, &mut core::iter::empty())),
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
///         //     core::iter::once(bare_err_tree::ErrTreeConv::from(&self.0 as &dyn Error))
///         //     self.1
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
            &mut core::iter::empty()$( .chain(
                core::iter::once(
                    bare_err_tree::WrapErr::tree(&$x)
                )
            ) )*,
            &$pkg,
        ))
    };
    ($func:expr, $inner:expr, $pkg:expr, $( $x:expr ),* ) => {
        ($func)(bare_err_tree::ErrTree::with_pkg(
            &$inner,
            &mut core::iter::empty()$( .chain(
                core::iter::once($x)
            ) )*,
            &$pkg,
        ))
    };
}
