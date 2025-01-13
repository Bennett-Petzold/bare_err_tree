/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::error::Error;

use crate::{AsErrTree, ErrTree};

/// Provides a default [`AsErrTree`] for arbitrary [`Error`]s.
///
/// The primary purpose of this type is to enable `&E` to have a
/// `&dyn AsErrTree` cast without an intermediate `&dyn Error` and without
/// an [`AsErrTree`] blanket impl. This requires an unsafe cast via the
/// transparent repr guarantees.
///
/// ```rust
/// use bare_err_tree::{WrapErr, AsErrTree};
///
/// let err = std::io::Error::last_os_error();
/// let err_ref = &err;
///
/// let wrapped = WrapErr::wrap(err_ref);
/// let as_dyn = wrapped as &dyn AsErrTree;
///
/// let alt_dyn = WrapErr::tree(err_ref);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct WrapErr<E: Error + ?Sized>(pub E);

impl<E: Error + ?Sized> From<&E> for &WrapErr<E> {
    fn from(value: &E) -> Self {
        unsafe { &*(value as *const E as *const WrapErr<E>) }
    }
}

impl<E: Error + ?Sized> WrapErr<E> {
    pub fn wrap(err: &E) -> &Self {
        err.into()
    }
}

impl<E: Error> WrapErr<E> {
    pub fn tree(err: &E) -> &dyn AsErrTree {
        Self::wrap(err) as &dyn AsErrTree
    }
}

impl<E: Error> AsErrTree for WrapErr<E> {
    fn as_err_tree(&self, func: &mut dyn FnMut(ErrTree<'_>)) {
        match self.0.source() {
            Some(e) => (func)(ErrTree::no_pkg(
                &self.0,
                &mut core::iter::once(&e as &dyn AsErrTree),
            )),
            None => (func)(ErrTree::no_pkg(&self.0, &mut core::iter::empty())),
        }
    }
}
