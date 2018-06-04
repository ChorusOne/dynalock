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

//! Dynalock: A lease based distributed lock.
//!
//! Dynalock algorithm supports lease based distributed locking implementations
//! through providers that support a strongly consistent Compare-And-Swap (CAS)
//! operation, or at least the compare-and-set variant and an eventually consistent
//! read operation. For algorithmic details please refer to the README.md file.
//!
//! You can use this library to cooperatively synchronize access on a conceptual
//! resource (e.g., pushing an item to a queue). The main data structure we use
//! to describe the lock is `DistLock`.
//!
//! The generic structure `DistLock` accepts a single type parameter `Driver` to
//! delegate the implementation of the `Locking` trait to back-end or storage
//! driver implementations. As an example, the DynamoDB driver implements the
//! `Locking` trait for `DistLock<DynamoDbDriver>`. The `Locking` trait is an API
//! contract where driver implementations will implement the Dynalock algorithm for
//! using the provider's primitives.

extern crate core;

#[macro_use]
extern crate log;

#[macro_use]
extern crate maplit;

#[cfg(feature = "dynamodb")]
pub extern crate rusoto_core;
#[cfg(feature = "dynamodb")]
pub extern crate rusoto_dynamodb;
#[cfg(feature = "dynamodb")]
extern crate uuid;

pub mod error;
pub mod providers;

pub use error::{DynaError, DynaErrorKind};
pub use providers::*;

use std::time::{Duration, Instant};

/// The Locking trait provides a contractual API that providers implement the Dynalock
/// algorithm using the particular provider's primitives.
///
/// Each method has an associated type on the trait for its `input`. Providers
/// must implement all of the associated types to provide input to this trait's
/// methods.
///
/// All trait methods return `Result<T, E>` where `E` is always `DynaError` except
/// for `remaining` where it returns `Option<Duration>`.
pub trait Locking {
    /// Associated type for the `acquire_lock` method input type.
    type AcquireLockInputType;
    /// Associated type for the `refresh_lock` method input type.
    type RefreshLockInputType;
    /// Associated type for the `release_lock` method input type.
    type ReleaseLockInputType;

    /// Try to acquire a lock on a shared resource.
    ///
    /// If successful this method must return an `std::time::Instant` that marks
    /// the point in time when the lease on a lock was obtained. Providers should
    /// only generate an `Instant` after the last I/O call is made.
    fn acquire_lock(&mut self, input: &Self::AcquireLockInputType) -> Result<Instant, DynaError>;

    /// Try to refresh the current lock data structure.
    ///
    /// This is useful when `acquire_lock` fails with `DynaErrorKind::LockAlreadyAcquired`
    /// error and the provider doesn't support a Compare-And-Swap primitive and only supports
    /// the compare-and-set variant.
    fn refresh_lock(&mut self, input: &Self::RefreshLockInputType) -> Result<(), DynaError>;

    /// When `acquire_lock` is successful it returns an `std::time::Instant` which is used
    /// to track the time from when the lease was issued. This method is used to safely
    /// calculate the time or duration left since `acquire_lock` was called. If the return
    /// value is `None` this means that the lock lease has expired and you must stop
    /// mutating the shared resource immediately.
    fn remaining(&self, instant: Instant) -> Option<Duration>;

    /// This optional method is only useful in rare situations and highly depends on
    /// the provider's implementation and primitives supported. Providers should
    /// implement this method to release the lock by clearing the fence token.
    fn release_lock(&mut self, _input: &Self::ReleaseLockInputType) -> Result<(), DynaError> {
        Ok(())
    }
}

/// The distributed lock structure that holds all the internal lock state and information.
///
/// This is the entry point to this library and should be used to hold a lock on a shared resource.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use dynalock::DistLock;
///
/// let mut dlock = DistLock::new(
///     "some driver",
///     Duration::from_secs(10)
///     );
///
/// # assert_eq!(*dlock.driver(), "some driver");
/// # assert_eq!(dlock.duration(), Duration::from_secs(10));
/// ```
#[derive(Debug)]
pub struct DistLock<Driver> {
    driver: Driver,
    duration: Duration,
}

impl<Driver> DistLock<Driver> {
    /// Initialize a new DistLock structure and return it.
    ///
    /// This static method accepts a `Driver` and `std::time::Duration` as parameters.
    /// The `duration` parameter is used to describe the time for which the lock should be held.
    pub fn new(driver: Driver, duration: Duration) -> Self {
        DistLock {
            driver: driver,
            duration: duration,
        }
    }

    /// Return a mutable reference to the underlying `driver` field.
    pub fn driver(&mut self) -> &mut Driver {
        &mut self.driver
    }

    /// Return the configured lease duration for the lock.
    pub fn duration(&self) -> Duration {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distlock_new_success() {
        let mut lock = DistLock::new("test driver", Duration::from_secs(10));
        assert_eq!(lock.driver, "test driver");
        assert_eq!(lock.duration, Duration::from_secs(10));
        assert_eq!(*lock.driver(), "test driver");
        assert_eq!(lock.duration(), Duration::from_secs(10));
    }
}
