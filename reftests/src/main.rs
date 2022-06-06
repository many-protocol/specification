use clap::Parser;
use reqwest::Url;

mod helpers;
mod tests;

use crate::tests::{TestCase, TestCaseResult, TestConfig};
use tests::TEST_CASES;

#[derive(Debug, Parser)]
struct Opts {
    #[clap()]
    /// Server URL to contact.
    server: Url,

    #[clap(long)]
    /// Only run the tests which name contains the pattern (regex).
    filter: Option<regex::Regex>,
}

#[tokio::main]
async fn main() {
    let Opts { server, filter } = Opts::parse();

    let config = TestConfig { url: server };
    let mut succeeded = 0;
    let mut skipped = 0;
    let mut failed = 0;

    let test_cases: Box<dyn Iterator<Item = &TestCase>> = if let Some(f) = filter {
        let f = f.clone();
        Box::new(
            TEST_CASES
                .iter()
                .filter(move |case| f.is_match(case.name().as_str())),
        )
    } else {
        Box::new(TEST_CASES.iter())
    };

    for case in test_cases {
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
