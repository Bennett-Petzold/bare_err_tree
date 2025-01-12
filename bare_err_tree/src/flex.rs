/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::error::Error;

use crate::{AsErrTree, ErrTree};

/// Unifies both `AsErrTree` and `dyn Error` implementors into one type.
///
/// Solves lifetime problems that blocked a zero-allocation macro.
#[derive(Clone, Copy)]
pub enum ErrTreeConv<'a> {
    Tree(&'a dyn AsErrTree),
    Err(&'a dyn Error),
}

impl<'a> From<&'a dyn AsErrTree> for ErrTreeConv<'a> {
    fn from(value: &'a dyn AsErrTree) -> Self {
        Self::Tree(value)
    }
}

impl<'a> From<&'a dyn Error> for ErrTreeConv<'a> {
    fn from(value: &'a dyn Error) -> Self {
        Self::Err(value)
    }
}

impl<'a, T: AsErrTree> From<&'a T> for ErrTreeConv<'a> {
    fn from(value: &'a T) -> Self {
        Self::Tree(value)
    }
}

impl AsErrTree for ErrTreeConv<'_> {
    fn as_err_tree(&self, func: &mut dyn FnMut(crate::ErrTree<'_>)) {
        match self {
            Self::Tree(x) => x.as_err_tree(func),
            Self::Err(x) => match x.source() {
                Some(e) => (func)(ErrTree::no_pkg(x, &mut core::iter::once(e.into()))),
                None => (func)(ErrTree::no_pkg(x, &mut core::iter::empty())),
            },
        };
    }
}
