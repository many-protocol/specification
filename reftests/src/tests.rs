use self::attributes::LedgerConfig;
use futures::FutureExt;
use reqwest::Url;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::io::Read;
use std::panic::UnwindSafe;
use std::path::PathBuf;
use std::pin::Pin;

mod attributes;
mod protocol;

pub enum TestCaseResult {
    Success(),
    Skip(String),
    Fail(String),
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub url: Url,
    pub config: BTreeMap<String, PathBuf>,
}

pub trait ReadConfig<T> {
    fn read_config(&self, key: String) -> T;
}

impl ReadConfig<LedgerConfig> for TestConfig {
    fn read_config(&self, key: String) -> LedgerConfig {
        serde_json::from_str(&std::fs::read_to_string(&self.config[&key]).unwrap()).unwrap()
    }
}

pub struct TestCaseFn(
    fn(TestConfig) -> Pin<Box<dyn Future<Output = TestCaseResult> + Send + UnwindSafe>>,
);
impl UnwindSafe for TestCaseFn {}

impl Debug for TestCaseFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TestCaseFn")
    }
}

#[derive(Debug)]
pub struct TestCase {
    name: &'static str,
    filename: &'static str,
    test_fn: TestCaseFn,
}

impl TestCase {
    pub const fn new(name: &'static str, filename: &'static str, test_fn: TestCaseFn) -> Self {
        Self {
            name,
            filename,
            test_fn,
        }
    }

    pub async fn run(&self, config: TestConfig) -> TestCaseResult {
        let _stdout = gag::BufferRedirect::stdout().unwrap();
        let mut stderr = gag::BufferRedirect::stderr().unwrap();

        self.test_fn.0(config)
            .catch_unwind()
            .await
            .unwrap_or_else(|_err| {
                let mut message = String::new();
                stderr.read_to_string(&mut message).unwrap();

                TestCaseResult::Fail(message)
            })
    }

    pub fn name(&self) -> String {
        let p = self.filename.strip_suffix(".rs").unwrap_or(self.filename);
        let this_p = PathBuf::from(file!().to_string());
        let parent_p = this_p.parent().expect("Should have some parent");
        let p = p
            .strip_prefix(parent_p.to_string_lossy().as_ref())
            .unwrap_or(p)
            .strip_prefix('/')
            .unwrap_or(p);
        format!("{}::{}", p.replace('/', "::"), self.name)
    }
}

#[linkme::distributed_slice]
pub static TEST_CASES: [TestCase] = [..];
