//! DynaLock error type and kinds.

use core::fmt;
use std::error::Error;
use std::string::{String, ToString};

/// Kinds of errors
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DynaErrorKind {
    /// Unhandled error from another crate.
    UnhandledError,
    /// Provider error from another crate.
    ProviderError,
    /// Lock has been acquired by another processor.
    LockAlreadyAcquired,
}

impl DynaErrorKind {
    /// Return a string description of the error.
    pub fn as_str(&self) -> &str {
        match *self {
            DynaErrorKind::UnhandledError => "unhandled internal error",
            DynaErrorKind::ProviderError => "provider error",
            DynaErrorKind::LockAlreadyAcquired => "lock has been acquired by another processor",
        }
    }
}

impl fmt::Display for DynaErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type
#[derive(Debug)]
pub struct DynaError {
    kind: DynaErrorKind,
    description: Option<String>,
}

impl DynaError {
    /// Create a new error object with an optional error message.
    pub fn new(kind: DynaErrorKind, description: Option<&str>) -> Self {
        DynaError {
            kind: kind,
            description: description.map(|desc| desc.to_string()),
        }
    }

    /// Get DynaErrorKind for this error.
    pub fn kind(&self) -> DynaErrorKind {
        self.kind
    }
}

impl fmt::Display for DynaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.description {
            Some(ref desc) => write!(f, "{}: {}", self.description(), desc),
            None => write!(f, "{}", self.description()),
        }
    }
}

impl Error for DynaError {
    fn description(&self) -> &str {
        self.kind.as_str()
    }
}

impl From<DynaErrorKind> for DynaError {
    fn from(kind: DynaErrorKind) -> DynaError {
        DynaError {
            kind: kind,
            description: None,
        }
    }
}
