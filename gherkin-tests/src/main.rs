use std::convert::Infallible;

use async_trait::async_trait;
use cucumber::{given, then, when, WorldInit};

#[derive(Debug, WorldInit)]
struct World;

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World)
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

#[given(expr = "server is started")]
fn start_server(_world: &mut World) {}

#[when(expr = "{word} sends {int} {word} to {word}")]
fn send_symbol(_world: &mut World, _id1: String, _amount: u32, _symbol: String, _id2: String) {}

#[then(expr = "the balance of {word} should be {int} {word}")]
fn balance(_world: &mut World, _id: String, _amount: u32, _symbol: String) {}

#[tokio::main]
async fn main() {
    World::cucumber()
        // Skips can be confusing
        .fail_on_skipped()
        .run(".")
        .await;
}
