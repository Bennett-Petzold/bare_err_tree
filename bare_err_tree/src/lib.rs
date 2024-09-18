/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*!
CRATE DOCS TODO
*/

#![no_std]

#[cfg(feature = "heap_buffer")]
extern crate alloc;

use core::{
    borrow::Borrow,
    cell::RefCell,
    error::Error,
    fmt::{self, Debug},
    panic::Location,
};

mod pkg;
pub use pkg::*;
mod fmt_logic;
use fmt_logic::*;

#[cfg(feature = "derive")]
pub use bare_err_tree_proc::*;

/// Alternative to [`Result::unwrap`] that formats the error as a tree.
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
            tree.borrow().as_err_tree(&mut |tree| {
                #[cfg(not(feature = "heap_buffer"))]
                let front_lines = RefCell::new([0; FRONT_MAX]);

                #[cfg(feature = "heap_buffer")]
                let front_lines = RefCell::new(alloc::string::String::new());

                panic!(
                    "{}",
                    ErrTreeFmt::<FRONT_MAX> {
                        tree,
                        node: FmtDepthNode::new(false, None),
                        front_lines: &front_lines,
                    }
                )
            });
            unreachable!()
        }
    }
}

/// Produces [`ErrTree`] formatted output for an error.
///
/// TODO: document `FRONT_MAX`.
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
        #[cfg(not(feature = "heap_buffer"))]
        let front_lines = RefCell::new([0; FRONT_MAX]);

        #[cfg(feature = "heap_buffer")]
        let front_lines = RefCell::new(alloc::string::String::new());

        res = write!(
            formatter,
            "{}",
            ErrTreeFmt::<FRONT_MAX> {
                tree,
                node: FmtDepthNode::new(false, None),
                front_lines: &front_lines,
            }
        );
    });
    res
}

/// Intermediate struct for printing created by [`AsErrTree`].
///
/// # Fields
/// * `inner`: The current error.
/// * `sources`: The source error(s).
/// * `location`: The callsite (use `#[track_caller]` to track properly).
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
///         (func)(ErrTree::with_pkg(self, &[&[source]], self._pkg));
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
#[derive(Debug, Clone)]
pub struct ErrTree<'a> {
    inner: &'a dyn Error,
    sources: &'a [&'a [&'a dyn AsErrTree]],
    location: Option<&'a Location<'a>>,
}

impl<'a> ErrTree<'a> {
    pub fn with_pkg(
        inner: &'a dyn Error,
        sources: &'a [&[&dyn AsErrTree]],
        pkg: ErrTreePkg,
    ) -> Self {
        Self {
            inner,
            sources,
            location: pkg.location,
        }
    }

    pub fn no_pkg(inner: &'a dyn Error, sources: &'a [&[&dyn AsErrTree]]) -> Self {
        Self {
            inner,
            sources,
            location: None,
        }
    }
}

/// Defines an [`Error`]'s temporary view as an [`ErrTree`] for printing.
///
/// This can be defined with [`err_tree`], manually (see [`ErrTree`]), or with
/// the default `dyn` implementation. The `dyn` implementation does not track
/// any more information than standard library errors or track multiple sources.
pub trait AsErrTree: Error {
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

impl<T: ?Sized + AsErrTree> AsErrTree for &T {
    fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
        T::as_err_tree(self, func)
    }
}
