/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{cmp::Ordering, fmt::Debug, hash::Hash};

#[cfg(feature = "source_line")]
use core::panic::Location;

#[cfg(feature = "tracing")]
use tracing_error::SpanTrace;

#[cfg(feature = "boxed")]
use alloc::boxed::Box;

/// Captures extra information for [`ErrTree`][`crate::ErrTree`]
/// automatically.
///
/// [`Self::new()`] must be called by a function annotated with
/// `#[track_caller]` to capture the correct callsite.
///
/// The inner fields are obscured to allow arbitrary metadata tracking
/// combinations via feature flags without changing the API. The `boxed`
/// feature can be enabled to store this in heap.
///
/// All instances of this are considered equal, to avoid infecting sort order
/// or comparisons between the parent error types. Hashing is a no-op.
#[derive(Clone)]
pub struct ErrTreePkg {
    #[cfg(not(feature = "boxed"))]
    #[allow(dead_code)]
    inner: InnerErrTreePkg,
    #[cfg(feature = "boxed")]
    #[allow(dead_code)]
    inner: Box<InnerErrTreePkg>,
}

#[derive(Clone)]
pub struct InnerErrTreePkg {
    #[cfg(feature = "source_line")]
    location: &'static Location<'static>,
    #[cfg(feature = "tracing")]
    trace: SpanTrace,
}

impl ErrTreePkg {
    #[track_caller]
    pub fn new() -> Self {
        let inner = InnerErrTreePkg {
            #[cfg(feature = "source_line")]
            location: Location::caller(),
            #[cfg(feature = "tracing")]
            trace: SpanTrace::capture(),
        };

        #[cfg(feature = "boxed")]
        let inner = Box::new(inner);

        Self { inner }
    }

    #[cfg(feature = "source_line")]
    pub(crate) fn location(&self) -> &'static Location<'static> {
        self.inner.location
    }

    #[cfg(feature = "tracing")]
    pub(crate) fn trace(&self) -> &SpanTrace {
        &self.inner.trace
    }
}

impl Default for ErrTreePkg {
    #[cfg_attr(coverage, coverage(off))]
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ErrTreePkg {
    #[cfg_attr(coverage, coverage(off))]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "...")
    }
}

impl PartialEq for ErrTreePkg {
    #[cfg_attr(coverage, coverage(off))]
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Ord for ErrTreePkg {
    #[cfg_attr(coverage, coverage(off))]
    fn cmp(&self, _other: &Self) -> core::cmp::Ordering {
        Ordering::Equal
    }
}

impl Eq for ErrTreePkg {}

impl PartialOrd for ErrTreePkg {
    #[cfg_attr(coverage, coverage(off))]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for ErrTreePkg {
    #[cfg_attr(coverage, coverage(off))]
    fn hash<H: core::hash::Hasher>(&self, _state: &mut H) {}
}
