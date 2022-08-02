use std::collections::BTreeMap;

use crate::helpers::{anonymous_message, has_attribute, send};
use crate::tests::{TestCaseResult, TestConfig};
use ciborium::value::Value;
use many_types::attributes::AttributeId;
use minicbor::{Decode, Encode};
use reftests_macros::test_case;

#[derive(Decode, Encode)]
#[cbor(map)]
pub struct InfoReturn {
    #[n(0)]
    pub total: u64,
    // #[n(1)]
    // pub event_types: Vec<EventKind>,
}

#[derive(Encode, Decode)]
#[cbor(map)]
pub struct ListReturns {
    #[n(0)]
    pub nb_events: u64,
    // #[n(1)]
    // pub events: Vec<events::EventLog>, -> EventInfo -> EventMacro
}

pub struct EventClient {
    pub test_config: TestConfig,
    // pub ledger_config: LedgerConfig,
}

impl EventClient {
    pub async fn new(
        test_config: TestConfig,
        attributes: Option<Vec<AttributeId>>,
    ) -> Result<Self, String> {
        let envelope = anonymous_message("status", "null");
        let _response = send(&test_config, envelope).await;

        if attributes.is_some() {
            for a in attributes.unwrap().into_iter() {
                if !has_attribute(a, &test_config).await {
                    return Err("Server does not support ledger attribute.".to_string());
                }
            }
        }

        // let ledger_config = test_config.read_config("ledger".to_string());

        Ok(Self {
            test_config,
            // ledger_config,
        })
    }

    pub async fn events_info(&self) -> Result<u128, String> {
        let payload = "{}".to_string();
        let envelope = anonymous_message("events.info", payload);
        let response = send(&self.test_config, envelope).await;

        let payload = response.payload.expect("No payload from status");
        let response: BTreeMap<u8, Value> =
            ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

        let response_payload = response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
            .ok_or_else(|| {
                format!(
                    "[Response: {:?} -> was expected to be Bytes]",
                    response.get(&4).unwrap()
                )
            })?;

        let events_info_returns: InfoReturn =
            minicbor::decode(response_payload.as_slice()).unwrap();

        let total = events_info_returns.total;
        Ok(total.into())
    }

    pub async fn events_list(&self) -> Result<u128, String> {
        let payload = "{}".to_string();
        let envelope = anonymous_message("events.list", payload);
        let response = send(&self.test_config, envelope).await;

        let payload = response.payload.expect("No payload from status");
        let response: BTreeMap<u8, Value> =
            ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

        let response_payload = response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
            .ok_or_else(|| {
                format!(
                    "[Response: {:?} -> was expected to be Bytes]",
                    response.get(&4).unwrap()
                )
            })?;

        let events_list_returns: ListReturns =
            minicbor::decode(response_payload.as_slice()).unwrap();

        let total = events_list_returns.nb_events;
        Ok(total.into())
    }
}

#[test_case]
async fn can_get_events_list(test_config: TestConfig) -> TestCaseResult {
    let client = EventClient::new(test_config, Some(vec![4])).await.unwrap();

    async fn events_list_test(client: EventClient) -> Result<(), String> {
        // Test balance query
        // Balance should verify that the server has the attribute `2`.
        if !has_attribute(4, &client.test_config).await {
            return Err("Server does not support events attribute.".to_string());
        }

        // Getting events list from endpoint
        match client.events_list().await {
            Ok(_) => Ok(()),
            Err(s) => Err(s),
        }
    }

    match events_list_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

#[test_case]
async fn can_get_events_info(test_config: TestConfig) -> TestCaseResult {
    // // If the config file is not present, it should skip the test.
    // // This should return something that skips
    // let ledger_config = test_config.read_config("ledger".to_string());

    // // This should probably make a call to `status` endpoint of the server,
    // // but should definitely cache the result (avoid calling it everytime).
    // let client = EventClient::new(test_config, ledger_config).await;
    let client = EventClient::new(test_config, Some(vec![4])).await.unwrap();

    async fn events_info_test(client: EventClient) -> Result<(), String> {
        // Test balance query
        // Balance should verify that the server has the attribute `2`.
        if !has_attribute(4, &client.test_config).await {
            return Err("Server does not support events attribute.".to_string());
        }

        // Checking available events from endpoint
        match client.events_info().await {
            Ok(_) => Ok(()),
            Err(s) => Err(s),
        }
    }

    match events_info_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}
