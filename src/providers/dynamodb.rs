//! An implementation of the locking API using DynamoDB as a storage provider.

pub use super::{
    Locking,
    DistLock
};

use rusoto_core::Region;
use rusoto_dynamodb::{
    DynamoDbClient,
    GetItemInput,
    GetItemOutput,
    PutItemInput,
    PutItemOutput
};

use std::time::Duration;

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
/// ```rust
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
pub struct DynamoDbDriver {
    pub client: DynamoDbClient,
    pub table_name: String,
    pub token_field_name: String
}

impl Locking for DistLock<DynamoDbDriver> {
    fn acquire_lock(&mut self) -> &Self {
        self
    }

    fn release_lock(&mut self) -> &Self {
        self
    }

    fn expired(&self) -> bool {
        true
    }

    fn remaining(&self) -> Duration {
        Duration::from_secs(10)
    }
}
