use crate::helpers::{anonymous_message, has_attribute, send};
use crate::tests::{TestCaseResult, TestConfig};
use reftests_macros::test_case;

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
