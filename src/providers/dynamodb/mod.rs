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

//! An implementation of the locking API using DynamoDB as a storage provider.

use std::default::Default;
use std::result::Result;
use std::time::{Duration, Instant};
use uuid::Uuid;

use rusoto_core::reactor::{CredentialsProvider, RequestDispatcher};
use rusoto_core::{DispatchSignedRequest, ProvideAwsCredentials};
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemError, GetItemInput,
                      UpdateItemError, UpdateItemInput};

use {DistLock, Locking, DynaError, DynaErrorKind};

#[cfg(test)]
mod tests;

/// A struct to contain details of the DynamoDB lock implementation.
///
/// * `region`: An Amazon Web Services region.
/// * `table_name`: The DynamoDB table name.
/// * `token_field_name`: The token field to be used for RVN.
///
/// # Examples
///
/// Initialize a new DynamoDbDriver struct.
///
/// ```rust,ignore
/// extern crate dynalock;
/// extern crate rusoto_core;
/// extern crate rusoto_dynamodb;
///
/// use rusoto_core::Region;
/// use rusoto_dynamodb::DynamoDbClient;
///
/// use dynalock::providers::DynamoDbDriver;
///
/// # fn main() {
///     let detail = DynamoDbDriver {
///          client: DynamoDbClient::simple(Region::UsEast1),
///          table_name: "locks_table".to_string(),
///          token_field_name: "rvn".to_string()
///     };
///
/// #     assert_eq!(detail.table_name, "locks_table".to_string());
/// #     assert_eq!(detail.token_field_name, "rvn".to_string());
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
    partition_key_value: String,
    current_token: String,
}

impl<P, D> DynamoDbDriver<P, D>
where
    P: ProvideAwsCredentials,
    D: DispatchSignedRequest,
{
    fn new(client: DynamoDbClient<P, D>, input: &DynamoDbDriverInput) -> Self {
        DynamoDbDriver {
            client: client,
            table_name: input.table_name.clone(),
            partition_key_field_name: input.partition_key_field_name.clone(),
            partition_key_value: input.partition_key_value.clone(),
            token_field_name: input.token_field_name.clone(),
            duration_field_name: input.duration_field_name.clone(),
            current_token: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct DynamoDbDriverInput {
    pub table_name: String,
    pub partition_key_field_name: String,
    pub partition_key_value: String,
    pub token_field_name: String,
    pub duration_field_name: String,
}

impl Default for DynamoDbDriverInput {
    fn default() -> Self {
        DynamoDbDriverInput {
            table_name: String::new(),
            partition_key_field_name: String::new(),
            partition_key_value: String::from("singleton"),
            token_field_name: String::from("rvn"),
            duration_field_name: String::from("duration"),
        }
    }
}

/// A struct to hold input variables for locking methods method.
#[derive(Debug, Clone)]
pub struct DynamoDbLockInput {
    /// After how much time we timeout from a lock acquisition or refresh request to DynamoDB.
    pub timeout: Duration,
}

impl Default for DynamoDbLockInput {
    fn default() -> Self {
        DynamoDbLockInput {
            timeout: Duration::from_secs(10),
        }
    }
}

mod expressions {
    pub const UPDATE: &'static str = "SET #token_field = :token, #duration_field = :lease";
    pub const CONDITION: &'static str =
        "attribute_not_exists(#token_field) OR #token_field = :cond_token";
}

impl<P, D> Locking for DistLock<DynamoDbDriver<P, D>>
where
    P: ProvideAwsCredentials + 'static,
    D: DispatchSignedRequest + 'static,
{
    type AcquireLockInputType = DynamoDbLockInput;
    type RefreshLockInputType = DynamoDbLockInput;

    fn acquire_lock(&mut self, input: &Self::AcquireLockInputType) -> Result<Instant, DynaError> {
        let new_token = Uuid::new_v4().hyphenated().to_string();

        // Use new token as current token if this is our first run
        if self.driver.current_token.is_empty() {
            self.driver.current_token = new_token.clone();
        }

        // Prepare update method input
        let update_input = UpdateItemInput {
            table_name: self.driver.table_name.clone(),
            update_expression: Some(String::from(expressions::UPDATE)),
            condition_expression: Some(String::from(expressions::CONDITION)),
            expression_attribute_names: Some(hashmap! {
                String::from("#token_field") => self.driver.token_field_name.clone(),
                String::from("#duration_field") => self.driver.duration_field_name.clone()
            }),
            expression_attribute_values: Some(hashmap! {
                String::from(":token") => AttributeValue { s: Some(new_token.clone()), ..Default::default() },
                String::from(":lease") => AttributeValue { n: Some(self.duration.as_secs().to_string()), ..Default::default() },
                String::from(":cond_token") => AttributeValue { s: Some(self.driver.current_token.clone()), ..Default::default() }
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
        self.driver.current_token = new_token.clone();

        Ok(start)
    }

    fn refresh_lock(&mut self, input: &Self::RefreshLockInputType) -> Result<(), DynaError> {
        // Prepare get method input
        let get_input = GetItemInput {
            consistent_read: Some(true),
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
            }
        }

        Ok(())
    }

    fn remaining(&self, instant: Instant) -> Option<Duration> {
        self.duration.checked_sub(instant.elapsed())
    }
}

impl From<GetItemError> for DynaError {
    fn from(err: GetItemError) -> DynaError {
        DynaError::new(DynaErrorKind::ProviderError, Some(&err.to_string()))
    }
}

impl From<UpdateItemError> for DynaError {
    fn from(err: UpdateItemError) -> DynaError {
        match err {
            UpdateItemError::ConditionalCheckFailed(_) => {
                DynaError::new(DynaErrorKind::LockAlreadyAcquired, None)
            }
            _ => DynaError::new(DynaErrorKind::ProviderError, Some(&err.to_string())),
        }
    }
}
