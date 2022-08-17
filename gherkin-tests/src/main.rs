use std::{collections::BTreeMap, convert::Infallible, str::FromStr, sync::Arc};

use async_trait::async_trait;
use cose::new_identity;
use cucumber::{given, then, when, Parameter, WorldInit};
use many_identity::CoseKeyIdentity;
use opts::{read_spec_config, CmdOpts, SpecConfig};

mod cose;
mod opts;

#[derive(Parameter, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[param(regex = r"[\w\d]+", name = "identity")]
pub struct IdentityName([u8; 4]);

impl FromStr for IdentityName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        if len > 4 {
            return Err("Identity name is too big".into());
        }
        let v: Vec<u8> = std::iter::repeat(&0u8)
            .take(4 - len)
            .chain(bytes)
            .copied()
            .collect();
        Ok(IdentityName([v[0], v[1], v[2], v[3]]))
    }
}

impl From<&IdentityName> for u32 {
    fn from(name: &IdentityName) -> u32 {
        u32::from_be_bytes(name.0)
    }
}

#[derive(Debug, WorldInit)]
struct World {
    spec_config: Option<Arc<SpecConfig>>,
    identities: BTreeMap<IdentityName, CoseKeyIdentity>,
    symbols: Vec<String>,
}

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World {
            spec_config: None,
            identities: BTreeMap::new(),
            symbols: vec![],
        })
    }
}

impl Drop for World {
    fn drop(&mut self) {}
}

#[given(expr = "an identity {identity}")]
fn setup_identity(world: &mut World, id: IdentityName) {
    let identity = new_identity(&id).expect("Should have generated an identity");
    world.identities.insert(id, identity);
}

#[given(expr = "a symbol {word}")]
fn setup_symbol(world: &mut World, symbol: String) {
    assert!(world.symbols.contains(&symbol));
}

#[given(expr = "{identity} has {int} {word}")]
fn id_has_x_symbols(_world: &mut World, _id: IdentityName, _amount: u32, _symbol: String) {}

#[when(expr = "{identity} sends {int} {word} to {identity}")]
fn send_symbol(
    _world: &mut World,
    _id1: IdentityName,
    _amount: u32,
    _symbol: String,
    _id2: IdentityName,
) {
}

#[then(expr = "the balance of {identity} should be {int} {word}")]
fn balance(_world: &mut World, _id: IdentityName, _amount: u32, _symbol: String) {}

#[tokio::main]
async fn main() {
    let opts = cucumber::cli::Opts::<_, _, _, CmdOpts>::parsed();
    let spec_config = Arc::new(
        read_spec_config(&opts.custom.spec_config)
            .await
            .expect("Error while reading spec config"),
    );

    World::cucumber()
        .before(move |_, _, _, world| {
            let spec_config = spec_config.clone();
            let url = spec_config.server_url.clone();
            let faucet_identity = spec_config.faucet_identity.clone();
            world.spec_config = Some(spec_config);
            Box::pin(async {
                world.symbols = many_async_client::symbols(url, faucet_identity)
                    .await
                    .unwrap();
            })
        })
        .with_cli(opts)
        // Skips can be confusing
        .fail_on_skipped()
        .max_concurrent_scenarios(1)
        .run(".")
        .await;
}
