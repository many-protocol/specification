use crate::helpers::{anonymous_message, has_attribute, send};
use crate::tests::{TestCaseResult, TestConfig};
use ciborium::cbor;
use reftests_macros::test_case;
use serde::{Deserialize, Serialize};

/// This should be passed as a `--config=ledger=/some/path/to/config.json`.
/// This should be read using `serde_json::read`.
#[derive(Serialize, Deserialize)]
pub struct LedgerConfig {
    /// The identity of an account with tokens.
    pub faucet: Identity,

    /// The symbol for those tokens.
    pub symbol: Identity,
}

// This should return `Ok(())` if success, or `Err(String)` if skip,
// or should panic if error.
#[test_case]
async fn can_send(registry: TestConfig) -> TestCaseResult {
    // This should return an Identity type. But it needs to be able to
    // find the private key of that identity.
    // The number passed in argument should be consistent; if I pass the number
    // 1 twice, it should give me the same identity.
    let id1 = registry.identity(1);
    let id2 = registry.identity(2);

    // If the config file is not present, it should skip the test.
    // This should return something that skips
    let config = registry.read_config::<LedgerConfig>("ledger")?;

    // This should probably make a call to `status` endpoint of the server,
    // but should definitely cache the result (avoid calling it everytime).
    let client = LedgerClient::new(&registry.url);

    let symbol = config.symbol;
    let faucet = config.faucet;

    async fn test_fn() -> Result<(), String> {
        // Balance should verify that the server has the attribute `2`.
        // If it doesn't, it should skip the test.
        // If there is an error
        assert!(client.balance(faucet, symbol).await? >= 10000);

        // Same as above, with attribute 6.
        client.send(faucet, id1, symbol, 10000).await?;
        assert_eq!(client.balance(id1, symbol).await?, 10000);
        assert_eq!(client.balance(id2, symbol).await?, 0);
        client.send(id1, id2, symbol, 10000).await?;
        assert_eq!(client.balance(id1, symbol).await?, 0);
        assert_eq!(client.balance(id2, symbol).await?, 10000);
        client.send(id2, faucent, symbol, 10000).await?;

        Ok(())
    }

    match test_fn().await {
        Ok(_) => TestCaseResult::Success(),
        Err(s) => TestCaseResult::Skip(s),
    }
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
