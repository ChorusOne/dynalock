//   Copyright 2018 Chorus One, Inc.
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

//! DynaLock error type and kinds.

use core::fmt;
use std::error::Error;
use std::string::{String, ToString};

/// Kinds of errors
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DynaErrorKind {
    /// Unhandled error from another crate or the standard library.
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
#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynaerrorkind_as_str_success() {
        assert_eq!(
            DynaErrorKind::UnhandledError.as_str(),
            "unhandled internal error"
        );
        assert_eq!(DynaErrorKind::ProviderError.as_str(), "provider error");
        assert_eq!(
            DynaErrorKind::LockAlreadyAcquired.as_str(),
            "lock has been acquired by another processor"
        );
    }

    #[test]
    fn test_dynaerror_new_success() {
        let err = DynaError::new(DynaErrorKind::ProviderError, None);

        assert_eq!(err.kind(), DynaErrorKind::ProviderError);
        assert_eq!(err.description, None);
        assert_eq!(err.description(), "provider error");
    }

    #[test]
    fn test_from_dynaerrorkind_to_dynaerror_success() {
        let err = DynaError::new(DynaErrorKind::UnhandledError, None);

        assert_eq!(err, DynaError::from(DynaErrorKind::UnhandledError));
    }
}
