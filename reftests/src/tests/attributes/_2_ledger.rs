use crate::helpers::{anonymous_message, has_attribute, send};
use crate::tests::{TestCaseResult, TestConfig};
use ciborium::cbor;
use reftests_macros::test_case;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LedgerConfig {
    /// The identity of an account with tokens.
    pub faucet: Identity,

    /// The symbol for those tokens.
    pub symbol: Identity,
}

#[test_case]
async fn can_send(registry: TestConfig) -> TestCaseResult {
    let id1 = registry.identity(1);
    let id2 = registry.identity(2);

    let config = registry.read_config::<LedgerConfig>("ledger")?;
    let client = LedgerClient::new(&registry.url);

    let symbol = config.symbol;
    let faucet = config.faucet;

    assert!(client.balance(faucet, symbol) >= 10000);
    client.send(faucet, id1, symbol, 10000);
    assert_eq!(client.balance(id1, symbol), 10000);
    assert_eq!(client.balance(id2, symbol), 0);
    client.send(id1, id2, symbol, 10000);
    assert_eq!(client.balance(id1, symbol), 0);
    assert_eq!(client.balance(id2, symbol), 10000);
    client.send(id2, faucent, symbol, 10000);

    TestCaseResult::Success()
}

#[test_case]
async fn list_works_with_range(config: TestConfig) -> TestCaseResult {
    if !has_attribute(4, &config).await {
        return TestCaseResult::Skip("Server does not support ledger attribute.".to_string());
    }

    // Create a few transactions.
    let envelope = anonymous_message("status", "null");
    let _response = send(&config, envelope).await;

    TestCaseResult::Success()
}
