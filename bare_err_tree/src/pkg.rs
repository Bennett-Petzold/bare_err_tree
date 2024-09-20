/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{fmt::Debug, panic::Location};

/// Captures extra information for [`ErrTree`][`crate::ErrTree`]
/// automatically.
///
/// [`Self::new()`] must be called by a function annotated with
/// `#[track_caller]` to capture the correct callsite.
///
/// The inner fields are obscured to allow arbitrary metadata tracking
/// combinations via feature flags without changing the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErrTreePkg {
    pub(crate) location: Option<&'static Location<'static>>,
}

impl ErrTreePkg {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            location: Some(Location::caller()),
        }
    }
}

impl Default for ErrTreePkg {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}
