use std::{convert::Infallible, sync::Arc};

use async_trait::async_trait;
use cucumber::{given, then, when, WorldInit};
use opts::{read_spec_config, CmdOpts, SpecConfig};

mod opts;

#[derive(Debug, WorldInit)]
struct World {
    spec_config: Option<Arc<SpecConfig>>,
}

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World { spec_config: None })
    }
}

impl Drop for World {
    fn drop(&mut self) {}
}

#[given(expr = "an identity {word}")]
fn setup_identity(_world: &mut World, _id: String) {}

#[given(expr = "a symbol {word}")]
fn setup_symbol(_world: &mut World, _symbol: String) {}

#[given(expr = "{word} has {int} {word}")]
fn id_has_x_symbols(_world: &mut World, _id: String, _amount: u32, _symbol: String) {}

#[when(expr = "{word} sends {int} {word} to {word}")]
fn send_symbol(_world: &mut World, _id1: String, _amount: u32, _symbol: String, _id2: String) {}

#[then(expr = "the balance of {word} should be {int} {word}")]
fn balance(_world: &mut World, _id: String, _amount: u32, _symbol: String) {}

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
            Box::pin(async move {
                world.spec_config = Some(spec_config);
            })
        })
        .with_cli(opts)
        // Skips can be confusing
        .fail_on_skipped()
        .max_concurrent_scenarios(1)
        .run(".")
        .await;
}
