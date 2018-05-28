//! Unit tests for the DynamoDB provider.

extern crate rusoto_mock;

#[allow(unused_import)]
use super::*;

use std::default::Default;

use rusoto_core::Region;
use self::rusoto_mock::*;

#[test]
fn driver_input_default_is_sane() {
    let input = DynamoDbDriverInput::default();

    assert!(input.table_name.is_empty());
    assert!(input.partition_key_field_name.is_empty());
    assert_eq!(input.partition_key_value, String::from("singleton"));
    assert_eq!(input.token_field_name, String::from("rvn"));
    assert_eq!(input.duration_field_name, String::from("duration"));
}

#[test]
fn first_to_acquire_the_lock() {
    let body = MockResponseReader::read_response("test_resources/dynamodb", "update_lock_item_success.json");
    let mock = MockRequestDispatcher::with_status(200).with_body(&body);

    // Prepare input for DynamoDbDriver
    let input = DynamoDbDriverInput {
        table_name: String::from("test_lock_table"),
        partition_key_field_name: String::from("lock_id"),
        ..Default::default()
    };

    let client = DynamoDbClient::new(mock, MockCredentialsProvider, Region::UsEast1);
    let driver = DynamoDbDriver::new(client, &input);
    let mut lock = DistLock::new(driver, Duration::from_secs(10));

    let instant = lock.acquire_lock(&DynamoDbLockInput::default()).unwrap();
    assert_eq!(instant.elapsed().as_secs(), 0);
}

#[test]
fn second_to_acquire_the_lock() {
    let body = MockResponseReader::read_response("test_resources/dynamodb", "update_lock_condition_fail.json");
    let mock = MockRequestDispatcher::with_status(400).with_body(&body);

    // Prepare input for DynamoDbDriver
    let input = DynamoDbDriverInput {
        table_name: String::from("test_lock_table"),
        partition_key_field_name: String::from("lock_id"),
        ..Default::default()
    };

    let client = DynamoDbClient::new(mock, MockCredentialsProvider, Region::UsEast1);
    let driver = DynamoDbDriver::new(client, &input);
    let mut lock = DistLock::new(driver, Duration::from_secs(10));

    let result = lock.acquire_lock(&DynamoDbLockInput::default());
    assert!(result.is_err());
    assert_eq!(result.err().unwrap().kind(), DynaErrorKind::LockAlreadyAcquired);
}
