/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{cmp::Ordering, fmt::Debug};

#[cfg(feature = "source_line")]
use core::panic::Location;

#[cfg(feature = "boxed_tracing")]
use alloc::boxed::Box;

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
#[derive(Clone)]
pub struct ErrTreePkg {
    #[cfg(feature = "source_line")]
    pub(crate) location: &'static Location<'static>,
    #[cfg(all(feature = "tracing", not(feature = "boxed_tracing")))]
    pub(crate) trace: tracing_error::SpanTrace,
    #[cfg(feature = "boxed_tracing")]
    pub(crate) trace: Box<tracing_error::SpanTrace>,
}

impl ErrTreePkg {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "source_line")]
            location: Location::caller(),
            #[cfg(all(feature = "tracing", not(feature = "boxed_tracing")))]
            trace: tracing_error::SpanTrace::capture(),
            #[cfg(feature = "boxed_tracing")]
            trace: Box::new(tracing_error::SpanTrace::capture()),
        }
    }
}

impl Default for ErrTreePkg {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ErrTreePkg {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "...")
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
