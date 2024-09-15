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

/// Captures extra information for [`ErrTree`][`crate::ErrTree`]
/// automatically.
///
/// [`Self::new()`] must be called by a function annotated with
/// `#[track_caller]` to capture the correct callsite.
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
