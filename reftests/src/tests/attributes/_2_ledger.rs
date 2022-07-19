use crate::helpers::{anonymous_message, generate_key, has_attribute, message, send};
use crate::support::token_amount::{self, Symbol, TokenAmount};
use crate::support::types::Identity;
use crate::tests::{ReadConfig, TestCaseResult, TestConfig};
use ciborium::value::Value;
use minicbor::{Decode, Encode};
use reftests_macros::test_case;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Encode, Decode)]
#[cbor(map)]
pub struct BalanceReturns {
    #[n(0)]
    pub balances: BTreeMap<token_amount::Symbol, token_amount::TokenAmount>,
}

#[derive(Serialize, Deserialize)]
pub struct LedgerConfig {
    // The identity of an account with tokens.
    pub faucet: Identity,

    // The private key of the faucet identity.
    pub faucet_pk: String,

    // The symbol for those tokens.
    pub symbol: Identity,
}

pub struct LedgerClient {
    test_config: TestConfig,
    ledger_config: LedgerConfig,
}

impl LedgerClient {
    pub async fn new(test_config: TestConfig, ledger_config: LedgerConfig) -> Self {
        let envelope = anonymous_message("status", "null");
        let _response = send(&test_config, envelope).await;

        Self {
            test_config,
            ledger_config,
        }
    }

    pub fn get_identity(&self, key_seed: u8) -> Identity {
        let (_, kid, _) = generate_key(Some(key_seed), None);
        Identity::from_bytes(&kid).unwrap()
    }

    pub async fn balance(&self, account: String, symbol: Identity) -> Result<u128, String> {
        let payload = format!("{{0:\"{}\", 1:\"{}\"}}", account, symbol);
        let envelope = anonymous_message("ledger.balance", payload);
        let response = send(&self.test_config, envelope).await;

        let payload = response.payload.expect("No payload from status");
        let response: BTreeMap<u8, Value> =
            ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

        match response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
        {
            Some(response_payload) => {
                let balance_returns: BalanceReturns =
                    minicbor::decode(response_payload.as_slice()).unwrap();

                let balance = balance_returns.balances.get(&symbol);

                if balance == None {
                    return Ok(0);
                }

                Ok(balance.unwrap().to_string().parse::<u128>().unwrap())
            }
            None => Err(format!(
                "[Response: {:?} -> was expected to be Bytes]",
                response.get(&4).unwrap()
            )),
        }
    }

    pub async fn send(
        &self,
        key_seed: Option<u8>,
        from: String,
        to: String,
        amount: u64,
        symbol: Symbol,
        pem: Option<String>,
    ) -> Result<(), String> {
        let payload = format!(
            "{{0:\"{}\", 1:\"{}\", 2:{}, 3:\"{}\"}}",
            from,
            to,
            TokenAmount::from(amount),
            symbol
        );

        let envelope = if let Some(pem) = pem {
            message(None, "ledger.send", payload, Some(pem))
        } else {
            message(key_seed, "ledger.send", payload, None)
        };

        let response = send(&self.test_config, envelope).await;

        let payload = response.payload.expect("No payload from status");
        let response: BTreeMap<u8, Value> =
            ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

        match response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
        {
            Some(_) => Ok(()),
            None => Err(format!(
                "[Response: {:?} -> was expected to be Bytes]",
                response.get(&4).unwrap()
            )),
        }
    }
}

#[test_case]
async fn send_to_bad_identity(test_config: TestConfig) -> TestCaseResult {
    // If the config file is not present, it should skip the test.
    // This should return something that skips
    let ledger_config = test_config.read_config("ledger".to_string());

    // This should probably make a call to `status` endpoint of the server,
    // but should definitely cache the result (avoid calling it everytime).
    let client = LedgerClient::new(test_config, ledger_config).await;

    async fn send_test(client: LedgerClient) -> Result<(), String> {
        let akward_identity = "some_akward_impossible_identity".to_string();

        // Test balance query
        // Balance should verify that the server has the attribute `2`.
        if !has_attribute(2, &client.test_config).await {
            return Err("Server does not support ledger attribute.".to_string());
        }

        // Checking faucet has enough funds for following tests.
        assert!(
            client
                .balance(
                    client.ledger_config.faucet.to_string(),
                    client.ledger_config.symbol
                )
                .await
                .unwrap()
                > 10000
        );

        // Test sending command
        // Balance should verify that the server has the attribute `6`.
        if !has_attribute(6, &client.test_config).await {
            return Err("Server does not support ledger commands attribute.".to_string());
        }

        // Sending 10000 from faucet to odd identity
        match client
            .send(
                None,
                client.ledger_config.faucet.to_string(),
                akward_identity.to_owned(),
                10000,
                client.ledger_config.symbol,
                Some(client.ledger_config.faucet_pk.to_owned()),
            )
            .await
        {
            Ok(_) => {
                assert!(
                    client
                        .balance(akward_identity, client.ledger_config.symbol)
                        .await
                        .unwrap()
                        == 10000
                );
                Err("Was able to send from faucet to odd identity".to_string())
            }
            Err(_) => Ok(()),
        }
    }

    match send_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

// #[test_case]
// async fn cant_send_with_bad_private_key(test_config: TestConfig) -> TestCaseResult {}
// Skipped because the helper will panic if it can decode the PEM string

#[test_case]
async fn cant_send_without_funds(test_config: TestConfig) -> TestCaseResult {
    // If the config file is not present, it should skip the test.
    // This should return something that skips
    let ledger_config = test_config.read_config("ledger".to_string());

    // This should probably make a call to `status` endpoint of the server,
    // but should definitely cache the result (avoid calling it everytime).
    let client = LedgerClient::new(test_config, ledger_config).await;

    async fn cant_send_test(client: LedgerClient) -> Result<(), String> {
        // Test balance query
        // Balance should verify that the server has the attribute `2`.
        if !has_attribute(2, &client.test_config).await {
            return Err("Server does not support ledger attribute.".to_string());
        }

        // Checking faucet has enough funds for following tests.
        assert!(
            client
                .balance(
                    client.ledger_config.faucet.to_string(),
                    client.ledger_config.symbol
                )
                .await
                .unwrap()
                > 10000
        );

        // Test sending command
        // Balance should verify that the server has the attribute `6`.
        if !has_attribute(6, &client.test_config).await {
            return Err("Server does not support ledger commands attribute.".to_string());
        }

        // Getting Identity for seed: 1
        let id1 = client.get_identity(1);

        // Checking balance is 0
        assert!(
            client
                .balance(id1.to_string(), client.ledger_config.symbol)
                .await
                .unwrap()
                == 0
        );

        // Getting Identity for seed: 2
        let id2 = client.get_identity(2);

        // Checking balance on Identity:2
        assert!(
            client
                .balance(id2.to_string(), client.ledger_config.symbol)
                .await
                .unwrap()
                == 0
        );

        // Sending 10000 from Identity:1 to Identity:2
        match client
            .send(
                Some(1),
                id1.to_string(),
                id2.to_string(),
                10000,
                client.ledger_config.symbol,
                None,
            )
            .await
        {
            Ok(_) => {
                // Checking Identity:2 balance is now 10000
                assert!(
                    client
                        .balance(id2.to_string(), client.ledger_config.symbol)
                        .await
                        .unwrap()
                        == 10000
                );
                Err("Was able to send without funds".to_string())
            }
            Err(_) => Ok(()),
        }
    }

    match cant_send_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

#[test_case]
async fn can_send_with_funds_and_private_key(test_config: TestConfig) -> TestCaseResult {
    // If the config file is not present, it should skip the test.
    // This should return something that skips
    let ledger_config = test_config.read_config("ledger".to_string());

    // This should probably make a call to `status` endpoint of the server,
    // but should definitely cache the result (avoid calling it everytime).
    let client = LedgerClient::new(test_config, ledger_config).await;

    async fn send_test(client: LedgerClient) -> Result<(), String> {
        // Test balance query
        // Balance should verify that the server has the attribute `2`.
        if !has_attribute(2, &client.test_config).await {
            return Err("Server does not support ledger attribute.".to_string());
        }

        // Checking faucet has enough funds for following tests.
        assert!(
            client
                .balance(
                    client.ledger_config.faucet.to_string(),
                    client.ledger_config.symbol
                )
                .await
                .unwrap()
                > 10000
        );

        // Test sending command
        // Balance should verify that the server has the attribute `6`.
        if !has_attribute(6, &client.test_config).await {
            return Err("Server does not support ledger commands attribute.".to_string());
        }

        // Getting Identity for seed: 1
        let id1 = client.get_identity(1);

        // Checking balance is 0
        assert!(
            client
                .balance(id1.to_string(), client.ledger_config.symbol)
                .await
                .unwrap()
                == 0
        );

        // Sending 10000 from faucet to Identity:1
        match client
            .send(
                None,
                client.ledger_config.faucet.to_string(),
                id1.to_string(),
                10000,
                client.ledger_config.symbol,
                Some(client.ledger_config.faucet_pk.to_owned()),
            )
            .await
        {
            Ok(_) => {
                // Checking balance on Identity:1
                assert!(
                    client
                        .balance(id1.to_string(), client.ledger_config.symbol)
                        .await
                        .unwrap()
                        == 10000
                );

                // Getting Identity for seed: 2
                let id2 = client.get_identity(2);

                // Checking balance on Identity:2
                assert!(
                    client
                        .balance(id2.to_string(), client.ledger_config.symbol)
                        .await
                        .unwrap()
                        == 0
                );

                // Sending 10000 from Identity:1 to Identity:2
                match client
                    .send(
                        Some(1),
                        id1.to_string(),
                        id2.to_string(),
                        10000,
                        client.ledger_config.symbol,
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        // Checking Identity:1 balance is now 0
                        assert!(
                            client
                                .balance(id1.to_string(), client.ledger_config.symbol)
                                .await
                                .unwrap()
                                == 0
                        );

                        // Checking Identity:2 balance is now 10000
                        assert!(
                            client
                                .balance(id2.to_string(), client.ledger_config.symbol)
                                .await
                                .unwrap()
                                == 10000
                        );

                        // Returning balance from Identity:2 to faucet
                        match client
                            .send(
                                Some(2),
                                id2.to_string(),
                                client.ledger_config.faucet.to_string(),
                                10000,
                                client.ledger_config.symbol,
                                None,
                            )
                            .await
                        {
                            Ok(_) => {
                                // Checking Identity:2 balance is now 0
                                assert!(
                                    client
                                        .balance(id2.to_string(), client.ledger_config.symbol)
                                        .await
                                        .unwrap()
                                        == 0
                                );
                                Ok(())
                            }
                            Err(e) => Err(format!("Was able to send from id2 to faucet: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("Was able to send from id1 to id2: {}", e)),
                }
            }
            Err(e) => Err(format!("Was able to send from faucet to id1: {}", e)),
        }
    }

    match send_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

#[test_case]
async fn cant_get_balance_from_bad_identity(test_config: TestConfig) -> TestCaseResult {
    // If the config file is not present, it should skip the test.
    // This should return something that skips
    let ledger_config = test_config.read_config("ledger".to_string());

    // This should probably make a call to `status` endpoint of the server,
    // but should definitely cache the result (avoid calling it everytime).
    let client = LedgerClient::new(test_config, ledger_config).await;

    async fn balance_test(client: LedgerClient) -> Result<(), String> {
        let akward_identity = "some_akward_impossible_identity".to_string();

        // Test balance query
        // Balance should verify that the server has the attribute `2`.
        if !has_attribute(2, &client.test_config).await {
            return Err("Server does not support ledger attribute.".to_string());
        }

        // Checking balance for odd identity
        match client
            .balance(akward_identity, client.ledger_config.symbol)
            .await
        {
            Ok(_) => Err("Was able to get balance from bad identity".to_string()),
            Err(_) => Ok(()),
        }
    }

    match balance_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}

#[test_case]
async fn can_get_balance(test_config: TestConfig) -> TestCaseResult {
    // If the config file is not present, it should skip the test.
    // This should return something that skips
    let ledger_config = test_config.read_config("ledger".to_string());

    // This should probably make a call to `status` endpoint of the server,
    // but should definitely cache the result (avoid calling it everytime).
    let client = LedgerClient::new(test_config, ledger_config).await;

    async fn balance_test(client: LedgerClient) -> Result<(), String> {
        // Test balance query
        // Balance should verify that the server has the attribute `2`.
        if !has_attribute(2, &client.test_config).await {
            return Err("Server does not support ledger attribute.".to_string());
        }

        // Checking faucet has enough funds for following tests.
        assert!(
            client
                .balance(
                    client.ledger_config.faucet.to_string(),
                    client.ledger_config.symbol
                )
                .await
                .unwrap()
                > 10000
        );
        Ok(())
    }

    match balance_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}
