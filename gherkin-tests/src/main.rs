use std::{cmp::Ordering, sync::Arc};

use cucumber::{given, then, when, WorldInit};
use num_bigint::BigUint;
use opts::{read_spec_config, CmdOpts};
use world::{IdentityName, World};

mod cose;
mod opts;
mod world;

#[given(expr = "an identity {identity}")]
fn setup_identity(world: &mut World, id: IdentityName) {
    world.insert_identity(id);
}

#[given(expr = "a symbol {word}")]
fn setup_symbol(world: &mut World, symbol: String) {
    assert!(world.symbols().contains_key(&symbol));
}

#[given(expr = "{identity} has {int} {word}")]
async fn id_has_x_symbols(world: &mut World, id: IdentityName, amount: BigUint, symbol: String) {
    let faucet = world.spec_config().faucet_identity.clone();
    let identity = world.identity(&id).unwrap().clone();
    let symbol = *world.symbol(&symbol).unwrap();
    let current_balance = world
        .client()
        .balance(identity.identity, symbol)
        .await
        .unwrap();

    let faucet_balance = world
        .client()
        .balance(faucet.identity, symbol)
        .await
        .unwrap();
    assert_ne!(faucet_balance, 0u32.into());

    match amount.cmp(&current_balance) {
        Ordering::Greater => world
            .client()
            .send(
                faucet,
                identity.identity,
                amount.clone() - current_balance,
                symbol,
            )
            .await
            .expect("Should have sent"),
        Ordering::Less => world
            .client()
            .send(
                identity.clone(),
                faucet.identity,
                current_balance - amount.clone(),
                symbol,
            )
            .await
            .expect("Should have sent"),
        _ => {}
    }

    let new_balance = world
        .client()
        .balance(identity.identity, symbol)
        .await
        .unwrap();
    assert_eq!(new_balance, amount);
}

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
fn balance_should_be(_world: &mut World, _id: IdentityName, _amount: u32, _symbol: String) {}

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
            Box::pin(world.init_config(spec_config))
        })
        .with_cli(opts)
        // Skips can be confusing
        .fail_on_skipped()
        .max_concurrent_scenarios(1)
        .run(".")
        .await;
}
