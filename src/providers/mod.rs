//! Data store providers which provide a common API to storage of lock states.

#[cfg(feature = "dynamodb")]
pub mod dynamodb;

pub mod mock;

#[cfg(feature = "dynamodb")]
pub use self::dynamodb::DynamoDbDriver;

use error::DynaError;
use std::result::Result;
use std::time::{Duration, Instant};

pub trait Locking {
    type AcquireLockInputType;
    type RefreshLockInputType;

    fn acquire_lock(&mut self, input: Self::AcquireLockInputType) -> Result<Instant, DynaError>;
    fn refresh_lock(&mut self, input: Self::RefreshLockInputType) -> Result<(), DynaError>;
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
