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

//! Unit tests for the DynamoDB provider.

extern crate rusoto_mock;

use std::default::Default;

use super::*;

use self::rusoto_mock::*;
use rusoto_core::Region;

#[test]
fn driver_input_default_is_sane() {
    let input = DynamoDbDriverInput::default();

    assert!(input.table_name.is_empty());
    assert!(input.partition_key_field_name.is_empty());
    assert_eq!(input.partition_key_value, String::from("singleton"));
    assert_eq!(input.token_field_name, String::from("rvn"));
    assert_eq!(input.duration_field_name, String::from("duration"));
    assert_eq!(input.ttl_field_name, String::from("ttl"));
    assert_eq!(input.ttl_value, DAY_SECONDS * 7);
}

#[test]
fn lock_input_default_is_sane() {
    let input = DynamoDbLockInput::default();

    assert_eq!(input.timeout, Duration::from_secs(10));
    assert_eq!(input.consistent_read, Some(false));
}

#[test]
fn first_to_acquire_the_lock_success() {
    let body = MockResponseReader::read_response(
        "test_resources/dynamodb",
        "update_lock_item_success.json",
    );
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
fn second_to_acquire_the_lock_fail() {
    let body = MockResponseReader::read_response(
        "test_resources/dynamodb",
        "update_lock_condition_fail.json",
    );
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
    assert_eq!(
        result.err().unwrap().kind(),
        DynaErrorKind::LockAlreadyAcquired
    );
}

#[test]
fn refresh_lock_updates_current_token_success() {
    let body =
        MockResponseReader::read_response("test_resources/dynamodb", "get_lock_item_success.json");
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
    assert!(lock.driver.current_token.is_empty());

    let result = lock.refresh_lock(&DynamoDbLockInput::default());
    assert!(result.is_ok());
    assert_eq!(lock.driver.current_token, String::from("test RVN token"));
}

#[test]
fn refresh_lock_no_update_current_token_when_empty_success() {
    let body = MockResponseReader::read_response(
        "test_resources/dynamodb",
        "get_empty_lock_item_success.json",
    );
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
    assert!(lock.driver.current_token.is_empty());
    lock.driver.current_token = String::from("test-manually-set RVN token");

    let result = lock.refresh_lock(&DynamoDbLockInput::default());
    assert!(result.is_ok());
    assert_eq!(
        lock.driver.current_token,
        String::from("test-manually-set RVN token")
    );
}

#[test]
fn release_lock_clears_current_token_success() {
    let body = MockResponseReader::read_response(
        "test_resources/dynamodb",
        "update_lock_item_success.json",
    );
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
    lock.driver.current_token = String::from("test RVN token");

    let result = lock.release_lock(&DynamoDbLockInput::default());
    assert!(result.is_ok());
    println!("{}", lock.driver.current_token);
    assert!(lock.driver.current_token.is_empty())
}

#[test]
fn remaining_time_is_calculated_correctly_success() {
    let body = MockResponseReader::read_response(
        "test_resources/dynamodb",
        "update_lock_item_success.json",
    );
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
    let remaining = lock.remaining(instant).unwrap();

    assert_eq!(remaining.as_secs(), 9);
    assert!(remaining.subsec_nanos() > 999900000);
}
