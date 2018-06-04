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

//! An implementation of the locking API using DynamoDB as a storage provider
//!
//! This implementation fully implements the `Locking` trait for the `DistLock<DynamoDbDriver>`
//! structure. Lock items on the DynamoDB table are composed of the following attributes:
//!
//! - Partition key field
//! - Fence token field
//! - Lease duration field
//! - TTL field
//!
//! The partition key of the table is used as an identifier of the shared resource,
//! while the fence token is used to prevent the ABA problem. The duration attribute
//! is used to specify the lock's duration. The TTL field is used to tell DynamoDB
//! when to garbage-collect or remove items that has expired, that if TTL is
//! configured on the table.
//!
//! Currently the fence token is implemented by generating a UUID v4 token for
//! every `acquire_lock` and `release_lock` operation. UUID v4 security and strength depends on
//! the recent implementation of a reseeded version of the HC-128 CSPRNG in `std::rand`,
//! as long as this invariant holds, fence token collisions are as rare as the CSPRNG period
//! allows it to be (i.e., incredibly long period).

use std::default::Default;
use std::result::Result;
use std::time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH};
use uuid::Uuid;

use rusoto_core::reactor::{CredentialsProvider, RequestDispatcher};
use rusoto_core::{DispatchSignedRequest, ProvideAwsCredentials};
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemError, GetItemInput,
                      UpdateItemError, UpdateItemInput};

use {DistLock, DynaError, DynaErrorKind, Locking};

#[cfg(test)]
mod tests;

/// A structure to contain details of the DynamoDB lock implementation.
///
/// # Examples
///
/// Initialize a new DynamoDbDriver struct.
///
/// ```rust,no_run
/// extern crate dynalock;
///
/// use dynalock::rusoto_core::Region;
/// use dynalock::rusoto_dynamodb::DynamoDbClient;
///
/// use dynalock::dynamodb::{DynamoDbDriver, DynamoDbDriverInput};
///
/// # fn main() {
///     let input = DynamoDbDriverInput {
///          table_name: "locks_table".to_string(),
///          partition_key_field_name: String::from("lock_id"),
///          ..Default::default()
///     };
///
///     let driver = DynamoDbDriver::new(
///         DynamoDbClient::simple(Region::UsEast1), &input);
/// # }
/// ```
pub struct DynamoDbDriver<P = CredentialsProvider, D = RequestDispatcher>
where
    P: ProvideAwsCredentials,
    D: DispatchSignedRequest,
{
    client: DynamoDbClient<P, D>,
    table_name: String,
    partition_key_field_name: String,
    token_field_name: String,
    duration_field_name: String,
    ttl_field_name: String,
    ttl_value: u64,
    partition_key_value: String,
    current_token: String,
}

impl<P, D> DynamoDbDriver<P, D>
where
    P: ProvideAwsCredentials,
    D: DispatchSignedRequest,
{
    /// Initialize a new DynamoDbDriver structure and fill it with the `client`
    /// and `input` variables' contents.
    pub fn new(client: DynamoDbClient<P, D>, input: &DynamoDbDriverInput) -> Self {
        DynamoDbDriver {
            client: client,
            table_name: input.table_name.clone(),
            partition_key_field_name: input.partition_key_field_name.clone(),
            partition_key_value: input.partition_key_value.clone(),
            token_field_name: input.token_field_name.clone(),
            duration_field_name: input.duration_field_name.clone(),
            ttl_field_name: input.ttl_field_name.clone(),
            ttl_value: input.ttl_value,
            current_token: String::new(),
        }
    }
}

/// The number of seconds in 24 hours.
pub const DAY_SECONDS: u64 = 86400;

/// A structure that describes the inputs to `DynamoDbDriver::new`.
///
/// This structure's `Default` trait implementation provides sane default
/// values. Only the `table_name` and the `partition_key_field_name` fields are
/// required.
#[derive(Debug)]
pub struct DynamoDbDriverInput {
    /// The DynamoDB lock table name to be used.
    pub table_name: String,
    /// The partition key field name.
    pub partition_key_field_name: String,
    /// The partition key value (default: "singleton"). This field should be provided
    /// to use the lock driver on multiple shared resources, each represented by a
    /// partition key value.
    pub partition_key_value: String,
    /// The fence token field name (default: "rvn").
    pub token_field_name: String,
    /// The lease duration field name (default: "duration").
    pub duration_field_name: String,
    /// The TTL field name (default: "ttl").
    pub ttl_field_name: String,
    /// The TTL value to be added to the wall clock for expiration (default: 7 days in seconds).
    pub ttl_value: u64,
}

impl Default for DynamoDbDriverInput {
    fn default() -> Self {
        DynamoDbDriverInput {
            table_name: String::new(),
            partition_key_field_name: String::new(),
            partition_key_value: String::from("singleton"),
            token_field_name: String::from("rvn"),
            duration_field_name: String::from("duration"),
            ttl_field_name: String::from("ttl"),
            ttl_value: DAY_SECONDS * 7,
        }
    }
}

/// A struct to hold input variables for the `Locking` trait methods inputs.
///
/// The optional field `consistent_read` is not required to be set for the `refresh_lock`
/// method, this field only exists for convenience.
#[derive(Debug, Clone)]
pub struct DynamoDbLockInput {
    /// After how much time we timeout from a lock acquisition or refresh request to DynamoDB.
    pub timeout: Duration,
    /// Whether to carry out a strongly consistent read on the table within a refresh request.
    pub consistent_read: Option<bool>,
}

impl Default for DynamoDbLockInput {
    fn default() -> Self {
        DynamoDbLockInput {
            timeout: Duration::from_secs(10),
            consistent_read: Some(false),
        }
    }
}

mod expressions {
    pub const ACQUIRE_UPDATE: &'static str =
        "SET #token_field = :new_token, #duration_field = :lease, #ttl_field = :ttl";
    pub const ACQUIRE_CONDITION: &'static str =
        "attribute_not_exists(#token_field) OR #token_field = :cond_current_token";
    pub const RELEASE_UPDATE: &'static str = "REMOVE #token_field";
    pub const RELEASE_CONDITION: &'static str =
        "attribute_exists(#token_field) AND #token_field = :cond_current_token";
}

impl<P, D> Locking for DistLock<DynamoDbDriver<P, D>>
where
    P: ProvideAwsCredentials + 'static,
    D: DispatchSignedRequest + 'static,
{
    type AcquireLockInputType = DynamoDbLockInput;
    type RefreshLockInputType = DynamoDbLockInput;
    type ReleaseLockInputType = DynamoDbLockInput;

    fn acquire_lock(&mut self, input: &Self::AcquireLockInputType) -> Result<Instant, DynaError> {
        let new_token = Uuid::new_v4().hyphenated().to_string();

        // Use new token as current token if this is our first run
        if self.driver.current_token.is_empty() {
            self.driver.current_token = new_token.clone();
        }

        // Get time since EPOCH in seconds and add to it the TTL value
        let ttl_secs =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + self.driver.ttl_value;

        // Prepare update method input
        let update_input = UpdateItemInput {
            table_name: self.driver.table_name.clone(),
            update_expression: Some(String::from(expressions::ACQUIRE_UPDATE)),
            condition_expression: Some(String::from(expressions::ACQUIRE_CONDITION)),
            expression_attribute_names: Some(hashmap! {
                String::from("#token_field") => self.driver.token_field_name.clone(),
                String::from("#duration_field") => self.driver.duration_field_name.clone(),
                String::from("#ttl_field") => self.driver.ttl_field_name.clone(),
            }),
            expression_attribute_values: Some(hashmap! {
                String::from(":new_token") => AttributeValue { s: Some(new_token.clone()), ..Default::default() },
                String::from(":lease") => AttributeValue { n: Some(self.duration.as_secs().to_string()), ..Default::default() },
                String::from(":ttl") => AttributeValue { n: Some(ttl_secs.to_string()), ..Default::default() },
                String::from(":cond_current_token") => AttributeValue { s: Some(self.driver.current_token.clone()), ..Default::default() }
            }),
            key: hashmap! {
                self.driver.partition_key_field_name.clone() => AttributeValue {
                    s: Some(self.driver.partition_key_value.clone()),
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        // Make a sync call with timeout
        self.driver
            .client
            .update_item(&update_input)
            .with_timeout(input.timeout)
            .sync()?;

        ////////// After this point the lock clock starts //////////
        let start = Instant::now();

        // Lock acquired successfully, record the new fence token
        info!(
            "lock '{}' acquired successfully, current token ({}) new token ({}) lease ({}s)",
            self.driver.partition_key_value,
            self.driver.current_token,
            new_token,
            self.duration.as_secs()
        );
        self.driver.current_token = new_token.clone();

        Ok(start)
    }

    fn refresh_lock(&mut self, input: &Self::RefreshLockInputType) -> Result<(), DynaError> {
        // Prepare get method input
        let get_input = GetItemInput {
            consistent_read: input.consistent_read,
            table_name: self.driver.table_name.clone(),
            key: hashmap! {
                self.driver.partition_key_field_name.clone() => AttributeValue {
                    s: Some(self.driver.partition_key_value.clone()),
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        // Make a sync call with timeout
        let output = self.driver
            .client
            .get_item(&get_input)
            .with_timeout(input.timeout)
            .sync()?;

        // A lock item was found
        if output.item.is_some() {
            let attr = output
                .item
                .as_ref()
                .unwrap()
                .get(&self.driver.token_field_name);

            if attr.is_some() {
                self.driver.current_token = attr.unwrap().s.as_ref().unwrap().clone();
                info!(
                    "lock '{}' refreshed successful, found new token ({})",
                    self.driver.partition_key_value, self.driver.current_token
                );
            }
        }

        Ok(())
    }

    fn release_lock(&mut self, input: &Self::ReleaseLockInputType) -> Result<(), DynaError> {
        // Prepare update method input
        let update_input = UpdateItemInput {
            table_name: self.driver.table_name.clone(),
            update_expression: Some(String::from(expressions::RELEASE_UPDATE)),
            condition_expression: Some(String::from(expressions::RELEASE_CONDITION)),
            expression_attribute_names: Some(hashmap! {
                String::from("#token_field") => self.driver.token_field_name.clone(),
            }),
            expression_attribute_values: Some(hashmap! {
                String::from(":cond_current_token") => AttributeValue { s: Some(self.driver.current_token.clone()), ..Default::default() }
            }),
            key: hashmap! {
                self.driver.partition_key_field_name.clone() => AttributeValue {
                    s: Some(self.driver.partition_key_value.clone()),
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        // Make a sync call with timeout
        self.driver
            .client
            .update_item(&update_input)
            .with_timeout(input.timeout)
            .sync()?;

        // Lock released successfully, clear the fence token
        info!(
            "lock '{}' successfully released for token ({})",
            self.driver.partition_key_value, self.driver.current_token
        );
        self.driver.current_token.clear();

        Ok(())
    }

    fn remaining(&self, instant: Instant) -> Option<Duration> {
        self.duration.checked_sub(instant.elapsed())
    }
}

impl From<SystemTimeError> for DynaError {
    fn from(err: SystemTimeError) -> DynaError {
        error!("{}", err);
        DynaError::new(DynaErrorKind::UnhandledError, Some(&err.to_string()))
    }
}

impl From<GetItemError> for DynaError {
    fn from(err: GetItemError) -> DynaError {
        error!("{}", err);
        DynaError::new(DynaErrorKind::ProviderError, Some(&err.to_string()))
    }
}

impl From<UpdateItemError> for DynaError {
    fn from(err: UpdateItemError) -> DynaError {
        match err {
            UpdateItemError::ConditionalCheckFailed(_) => {
                warn!("{}", err);
                DynaError::new(DynaErrorKind::LockAlreadyAcquired, None)
            }
            _ => {
                error!("{}", err);
                DynaError::new(DynaErrorKind::ProviderError, Some(&err.to_string()))
            }
        }
    }
}
