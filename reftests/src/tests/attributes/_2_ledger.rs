use crate::helpers::{anonymous_message, generate_key, has_attribute, message, send};
use crate::tests::{ReadConfig, TestCaseResult, TestConfig};
use crate::types::Identity;
use ciborium::value::Value;
use minicbor::{Decode, Encode};
use reftests_macros::test_case;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use self::ledger::{Symbol, TokenAmount};

pub mod ledger;
#[derive(Clone, Encode, Decode)]
#[cbor(map)]
pub struct BalanceReturns {
    #[n(0)]
    pub balances: BTreeMap<ledger::Symbol, ledger::TokenAmount>,
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

    pub async fn balance(&self, account: Identity, symbol: Identity) -> u128 {
        let payload = format!("{{0:\"{}\", 1:\"{}\"}}", account, symbol);
        let envelope = anonymous_message("ledger.balance", payload);
        let response = send(&self.test_config, envelope).await;

        let payload = response.payload.expect("No payload from status");
        let response: BTreeMap<u8, Value> =
            ciborium::de::from_reader(payload.as_slice()).expect("Invalid payload.");

        let response_payload = response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
            .expect("Response return value was expected to be Bytes");

        let balance_returns: BalanceReturns =
            minicbor::decode(response_payload.as_slice()).unwrap();

        let balance = balance_returns.balances.get(&symbol);

        if balance == None {
            return 0;
        }

        balance.unwrap().to_string().parse::<u128>().unwrap()
    }

    pub async fn send(
        &self,
        key_seed: Option<u8>,
        from: Identity,
        to: Identity,
        amount: u64,
        symbol: Symbol,
        pem: Option<String>,
    ) {
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

        response
            .get(&4)
            .expect("Response return value was not found")
            .as_bytes()
            .expect("Response return value was expected to be Bytes");
    }
}

#[test_case]
async fn can_send(test_config: TestConfig) -> TestCaseResult {
    // // If the config file is not present, it should skip the test.
    // // This should return something that skips
    let ledger_config = test_config.read_config("ledger".to_string());

    // // This should probably make a call to `status` endpoint of the server,
    // // but should definitely cache the result (avoid calling it everytime).
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
                .balance(client.ledger_config.faucet, client.ledger_config.symbol)
                .await
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
        assert!(client.balance(id1, client.ledger_config.symbol).await == 0);

        // Sending 10000 from faucet to Identity:1
        client
            .send(
                None,
                client.ledger_config.faucet,
                id1,
                10000,
                client.ledger_config.symbol,
                Some(client.ledger_config.faucet_pk.to_owned()),
            )
            .await;

        // Checking balance on Identity:1
        assert!(client.balance(id1, client.ledger_config.symbol).await == 10000);

        // Getting Identity for seed: 2
        let id2 = client.get_identity(2);

        // Checking balance on Identity:2
        assert!(client.balance(id2, client.ledger_config.symbol).await == 0);

        // Sending 10000 from Identity:1 to Identity:2
        client
            .send(Some(1), id1, id2, 10000, client.ledger_config.symbol, None)
            .await;

        // Checking Identity:1 balance is now 0
        assert!(client.balance(id1, client.ledger_config.symbol).await == 0);

        // Checking Identity:2 balance is now 10000
        assert!(client.balance(id2, client.ledger_config.symbol).await == 10000);

        // Returning balance from Identity:2 to faucet
        client
            .send(
                Some(2),
                id2,
                client.ledger_config.faucet,
                10000,
                client.ledger_config.symbol,
                None,
            )
            .await;

        // Checking Identity:2 balance is now 0
        assert!(client.balance(id2, client.ledger_config.symbol).await == 0);
        Ok(())
    }

    match send_test(client).await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
}
