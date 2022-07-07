use crate::helpers::{anonymous_message, send};
use crate::tests::{TestCaseResult, TestConfig};
use reftests_macros::test_case;

pub mod _2_ledger;

#[test_case]
async fn status_works(config: TestConfig) -> TestCaseResult {
    let envelope = anonymous_message("status", "null");
    let _response = send(&config, envelope).await;

    TestCaseResult::Success()
}
