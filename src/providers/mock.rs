//! A mock implementation of the locking API with in-memory storage.

use super::{
    Locking,
    DistLock
};

use std::time::Duration;
use std::result::Result;
use error::DynaError;

pub struct MockDetail {
    field_name: String
}

impl Locking for DistLock<String> {

    fn acquire_lock(&mut self) -> Result<(), DynaError> {
        Ok(())
    }

    fn release_lock(&mut self) -> Result<(), DynaError> {
        Ok(())
    }

    fn expired(&self) -> bool {
        true
    }

    fn remaining(&self) -> Duration {
        Duration::from_secs(10)
    }
}
