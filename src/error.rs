//! DynaLock error type and kinds.

use core::fmt;
use std::error::Error as StdError;
use std::string::{String, ToString};

/// Kinds of errors
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    /// Unhandled error from another crate.
    UnhandledError,
}

impl ErrorKind {
    /// Return a string description of the error.
    pub fn as_str(&self) -> &str {
        match *self {
            ErrorKind::UnhandledError => "unhandled internal error",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    description: Option<String>
}

impl Error {
    /// Create a new error object with an optional error message.
    pub fn new(kind: ErrorKind, description: Option<&str>) -> Self {
        Error {
            kind: kind,
            description: description.map(|desc| desc.to_string())
        }
    }

    /// Get ErrorKind for this error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.description {
            Some(ref desc) => write!(f, "{}: {}", self.description(), desc),
            None => write!(f, "{}", self.description())
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.kind.as_str()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind: kind,
            description: None
        }
    }
}
