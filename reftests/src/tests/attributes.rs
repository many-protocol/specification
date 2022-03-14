use crate::helpers::{anonymous_message, send};
use crate::tests::{TestCaseResult, TestConfig};
use ciborium::value::Value;
use reftests_macros::test_case;

mod _2_ledger;

#[test_case]
async fn status_works(config: TestConfig) -> TestCaseResult {
    let envelope = anonymous_message("status", Value::Null);
    let _response = send(&config, envelope).await;

    TestCaseResult::Success()
}
