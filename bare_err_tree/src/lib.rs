/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! CRATE DOCS TODO

#![no_std]
extern crate alloc;

use core::{
    borrow::Borrow,
    error::Error,
    fmt,
    fmt::{Debug, Display, Formatter},
    panic::Location,
};

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
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
pub fn tree_unwrap<T, E, S>(res: Result<T, S>) -> T
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
{
    match res {
        Ok(x) => x,
        Err(tree) => {
            panic!(
                "{}",
                ErrTreeFmt {
                    tree: &tree.borrow().as_err_tree(),
                    node: FmtDepthNode::new(false, None),
                }
            );
        }
    }
}

/// Produces [`ErrTree`] formatted output for an error.
#[track_caller]
pub fn print_tree<E, S>(tree: S) -> String
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
{
    ErrTreeFmt {
        tree: &tree.borrow().as_err_tree(),
        node: FmtDepthNode::new(false, None),
    }
    .to_string()
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
/// use bare_err_tree::{ErrTree, AsErrTree};
///
/// #[derive(Debug)]
/// pub struct HighLevelIo {
///     source: std::io::Error,
///     _location: &'static Location<'static>,
/// }
///
/// impl HighLevelIo {
///     #[track_caller]
///     pub fn new(source: std::io::Error) -> Self {
///         Self {
///             source,
///             _location: Location::caller(),
///         }
///     }
/// }
///
/// impl AsErrTree for HighLevelIo {
///     fn as_err_tree(&self) -> ErrTree<'_> {
///         let sources = Box::new([(&self.source as &dyn Error).as_err_tree()]);
///         ErrTree {
///             inner: self,
///             sources,
///             location: Some(self._location)
///         }
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
    pub inner: &'a dyn Error,
    pub sources: Box<[ErrTree<'a>]>,
    pub location: Option<&'a Location<'a>>,
}

/// Defines an [`Error`]'s temporary view as an [`ErrTree`] for printing.
///
/// This can be defined with [`err_tree`], manually (see [`ErrTree`]), or with
/// the default `dyn` implementation. The `dyn` implementation does not track
/// any more information than standard library errors or track multiple sources.
pub trait AsErrTree: Error {
    fn as_err_tree(&self) -> ErrTree<'_>;
}

impl<'a> AsErrTree for ErrTree<'a> {
    fn as_err_tree(&self) -> ErrTree<'_> {
        Self {
            inner: self.inner,
            sources: self.sources.clone(),
            location: self.location,
        }
    }
}

/// Displays with [`Error::source`] as the child.
///
/// Does not provide any of the extra tracking information or handle multiple
/// sources.
impl AsErrTree for dyn Error {
    fn as_err_tree(&self) -> ErrTree<'_> {
        let sources = match self.source() {
            Some(e) => vec![e.as_err_tree()],
            None => vec![],
        }
        .into_boxed_slice();
        ErrTree {
            inner: self,
            sources,
            location: None,
        }
    }
}

impl ErrTree<'_> {
    fn sources(&self) -> &[ErrTree<'_>] {
        &self.sources
    }
}

impl Error for ErrTree<'_> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

impl Display for ErrTree<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
