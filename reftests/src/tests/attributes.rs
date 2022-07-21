use crate::helpers::{message, send};
use crate::tests::{TestCaseResult, TestConfig};
use many_identity::Address;
use reftests_macros::test_case;
use serde::{Deserialize, Serialize};

pub mod _2_ledger;

#[test_case]
async fn status_works(config: TestConfig) -> TestCaseResult {
    let envelope = message("status", "null", None, None);
    let _response = send(&config, envelope).await;

    TestCaseResult::Success()
}

#[derive(Serialize, Deserialize)]
pub struct LedgerConfig {
    // The identity of an account with tokens.
    pub faucet: Address,

    // The private key of the faucet identity.
    pub faucet_pk: String,

    // The symbol for those tokens.
    pub symbol: Address,
}
