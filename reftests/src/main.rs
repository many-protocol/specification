use clap::Parser;
use reqwest::Url;

mod helpers;
mod tests;

use crate::tests::{TestCaseResult, TestConfig};
use tests::TEST_CASES;

#[derive(Debug, Parser)]
struct Opts {
    #[clap()]
    /// Server URL to contact.
    server: Url,
}

#[tokio::main]
async fn main() {
    let opts = Opts::parse();

    let config = TestConfig { url: opts.server };
    let mut succeeded = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for case in TEST_CASES {
        eprint!("Running {}... ", case.name());
        match case.run(config.clone()).await {
            TestCaseResult::Success() => {
                succeeded += 1;
                eprintln!("SUCCESS");
            }
            TestCaseResult::Skip(msg) => {
                skipped += 1;
                eprintln!("SKIPPED ({})", msg);
            }
            TestCaseResult::Fail(msg) => {
                failed += 1;
                eprintln!("FAILED\n{}\n", msg);
            }
        }
    }

    eprintln!(
        "\nTest results: {} succeeded, {} failed {}",
        succeeded,
        failed,
        if skipped != 0 {
            format!("({} skipped)", skipped)
        } else {
            "".to_string()
        }
    );
    if failed != 0 {
        std::process::exit(1);
    }
}
