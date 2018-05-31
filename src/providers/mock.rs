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
