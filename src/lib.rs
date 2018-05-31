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

//! Some docs go here

extern crate core;

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

pub use providers::*;
pub use error::{DynaError, DynaErrorKind};

use std::time::{Duration, Instant};

pub trait Locking {
    type AcquireLockInputType;
    type RefreshLockInputType;

    fn acquire_lock(&mut self, input: &Self::AcquireLockInputType) -> Result<Instant, DynaError>;
    fn refresh_lock(&mut self, input: &Self::RefreshLockInputType) -> Result<(), DynaError>;
    fn remaining(&self, instant: Instant) -> Option<Duration>;
}

/// A struct to represent a distributed lock.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use dynalock::DistLock;
///
/// let mut dl = DistLock::new(
///     "test",
///     Duration::from_secs(10)
///     );
///
/// # assert_eq!(*dl.driver(), "test");
/// # assert_eq!(dl.duration(), Duration::from_secs(10));
/// ```
#[derive(Debug)]
pub struct DistLock<Driver> {
    driver: Driver,
    duration: Duration,
}

impl<Driver> DistLock<Driver> {
    pub fn new(driver: Driver, duration: Duration) -> Self {
        DistLock {
            driver: driver,
            duration: duration,
        }
    }

    pub fn driver(&mut self) -> &mut Driver {
        &mut self.driver
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dist_lock_new() {
        assert!(true);
    }
}
