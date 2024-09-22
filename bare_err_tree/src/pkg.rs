/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{cmp::Ordering, fmt::Debug};

#[cfg(feature = "source_line")]
use core::panic::Location;

/// Captures extra information for [`ErrTree`][`crate::ErrTree`]
/// automatically.
///
/// [`Self::new()`] must be called by a function annotated with
/// `#[track_caller]` to capture the correct callsite.
///
/// The inner fields are obscured to allow arbitrary metadata tracking
/// combinations via feature flags without changing the API.
///
/// All instances of this are considered equal, to avoid infecting sort order
/// or comparisons between the parent error types.
#[derive(Debug, Clone)]
pub struct ErrTreePkg {
    #[cfg(feature = "source_line")]
    pub(crate) location: &'static Location<'static>,
    #[cfg(feature = "tracing")]
    pub(crate) trace: tracing_error::SpanTrace,
}

impl ErrTreePkg {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "source_line")]
            location: Location::caller(),
            #[cfg(feature = "tracing")]
            trace: tracing_error::SpanTrace::capture(),
        }
    }
}

impl Default for ErrTreePkg {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ErrTreePkg {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Ord for ErrTreePkg {
    fn cmp(&self, _other: &Self) -> core::cmp::Ordering {
        Ordering::Equal
    }
}

impl Eq for ErrTreePkg {}

impl PartialOrd for ErrTreePkg {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
