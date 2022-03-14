use crate::helpers::{anonymous_message, envelope, message, send, Payload};
use crate::tests::{TestCaseResult, TestConfig};
use ciborium::value::Value;
use coset::CborSerializable;
use reftests_macros::test_case;
use reqwest::StatusCode;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[test_case]
async fn signature_works(config: TestConfig) -> TestCaseResult {
    let envelope = message(0, "heartbeat", Value::Null);
    let response = send(&config, envelope).await;

    let payload = response.payload.expect("No payload");
    let value: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Not a CBOR encoded payload.");

    assert!(value.get(&4).unwrap().as_bytes().is_some());
    TestCaseResult::Success()
}

#[test_case]
async fn anonymous(config: TestConfig) -> TestCaseResult {
    let envelope = anonymous_message("heartbeat", Value::Null);
    let response = send(&config, envelope).await;

    let payload = response.payload.expect("No payload");
    let value: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Not a CBOR encoded payload.");

    assert!(value.get(&4).unwrap().as_bytes().is_some());
    TestCaseResult::Success()
}

#[test_case]
async fn requires_tag(config: TestConfig) -> TestCaseResult {
    let envelope = anonymous_message("heartbeat", Value::Null);
    let client = reqwest::Client::new();
    // Uses the `to_vec()` serializer instead of `to_tagged_vec()`.
    let r = client
        .get(config.url.clone())
        .body(envelope.to_vec().unwrap())
        .build()
        .unwrap();

    let result = client.execute(r).await.unwrap();
    let status = result.status();
    assert_eq!(status, StatusCode::from_u16(500).unwrap());

    // Just verify the body is valid text (or empty).
    let _ = result.text().await.unwrap();
    TestCaseResult::Success()
}

#[test_case]
async fn invalid_signature(config: TestConfig) -> TestCaseResult {
    let mut envelope = message(0, "heartbeat", Value::Null);
    // Override the first 4 bytes with zeros to invalidate the signature.
    envelope.signature[0..4].copy_from_slice(&[0u8; 4]);
    let response = send(&config, envelope).await;

    let payload = response.payload.expect("No payload");
    let value: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Not a CBOR encoded payload.");

    assert_eq!(
        value.get(&4).unwrap().as_map().unwrap().get(0).unwrap().1,
        Value::from(-1002i16)
    );
    TestCaseResult::Success()
}

#[test_case]
async fn accept_no_version(config: TestConfig) -> TestCaseResult {
    let message = Payload {
        endpoint: Some(Value::Text("status".to_string())),
        arguments: Some(Value::Bytes(vec![])),
        time: Some(Value::Tag(
            1,
            Box::new(Value::from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )),
        )),
        ..Default::default()
    }
    .to_tagged_value();

    let response = send(&config, envelope(None, message)).await;
    let payload = response.payload.expect("No payload");
    let value: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Not a CBOR encoded payload.");
    assert!(value.get(&4).unwrap().as_bytes().is_some());

    TestCaseResult::Success()
}

#[test_case]
async fn refuse_non_numerical_version(config: TestConfig) -> TestCaseResult {
    let message = Payload {
        version: Some(Value::Null),
        endpoint: Some(Value::Text("status".to_string())),
        arguments: Some(Value::Bytes(vec![])),
        time: Some(Value::Tag(
            1,
            Box::new(Value::from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )),
        )),
        ..Default::default()
    }
    .to_tagged_value();

    let response = send(&config, envelope(None, message)).await;
    let payload = response.payload.expect("No payload");
    let value: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Not a CBOR encoded payload.");
    assert!(value.get(&4).unwrap().as_map().is_some());

    TestCaseResult::Success()
}

#[test_case]
async fn refuse_future_version(config: TestConfig) -> TestCaseResult {
    let message = Payload {
        version: Some(Value::from(255u8)),
        endpoint: Some(Value::Text("status".to_string())),
        arguments: Some(Value::Bytes(vec![])),
        time: Some(Value::Tag(
            1,
            Box::new(Value::from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )),
        )),
        ..Default::default()
    }
    .to_tagged_value();

    let response = send(&config, envelope(None, message)).await;
    let payload = response.payload.expect("No payload");
    let value: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Not a CBOR encoded payload.");
    assert!(value.get(&4).unwrap().as_map().is_some());

    TestCaseResult::Success()
}

#[test_case]
async fn refuse_version_zero(config: TestConfig) -> TestCaseResult {
    let message = Payload {
        version: Some(Value::from(0)),
        endpoint: Some(Value::Text("status".to_string())),
        arguments: Some(Value::Bytes(vec![])),
        time: Some(Value::Tag(
            1,
            Box::new(Value::from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )),
        )),
        ..Default::default()
    }
    .to_tagged_value();

    let response = send(&config, envelope(None, message)).await;
    let payload = response.payload.expect("No payload");
    let value: BTreeMap<u8, Value> =
        ciborium::de::from_reader(payload.as_slice()).expect("Not a CBOR encoded payload.");
    assert!(value.get(&4).unwrap().as_map().is_some());

    TestCaseResult::Success()
}
