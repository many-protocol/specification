use std::{cmp::Ordering, sync::Arc};

use cucumber::{given, then, when, WorldInit};
use many_client::client::ledger::{SendArgs, TokenAmount};
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
    let amount: TokenAmount = amount.into();
    let faucet = world.spec_config().faucet_identity.clone();
    let identity = world.identity(&id).unwrap().clone();
    let symbol = *world.symbol(&symbol).unwrap();
    let current_balance = world.balance(identity.identity, symbol).await;
    let faucet_balance = world.balance(faucet.identity, symbol).await;

    assert_ne!(faucet_balance, TokenAmount::zero());

    match amount.cmp(&current_balance) {
        Ordering::Greater => {
            world
                .faucet_ledger_client()
                .send(SendArgs {
                    from: Some(faucet.identity),
                    to: identity.identity,
                    amount: amount.clone() - current_balance,
                    symbol,
                })
                .await
                .expect("Should have sent");
        }
        Ordering::Less => {
            world
                .ledger_client(identity.identity)
                .send(SendArgs {
                    from: Some(identity.identity),
                    to: faucet.identity,
                    amount: current_balance - amount.clone(),
                    symbol,
                })
                .await
                .expect("Should have sent");
        }
        _ => {}
    }

    let new_balance = world.balance(identity.identity, symbol).await;
    assert_eq!(new_balance, amount);
}

#[when(expr = "{identity} sends {int} {word} to {identity}")]
async fn send_symbol(
    world: &mut World,
    sender_id: IdentityName,
    amount: u32,
    symbol: String,
    receiver_id: IdentityName,
) {
    let symbol = *world.symbol(&symbol).unwrap();
    let sender = world.identity(&sender_id).unwrap().identity;
    let receiver = world.identity(&receiver_id).unwrap().identity;
    world
        .ledger_client(sender)
        .send(SendArgs {
            from: Some(sender),
            to: receiver,
            amount: amount.into(),
            symbol,
        })
        .await
        .unwrap();
}

#[then(expr = "the balance of {identity} should be {int} {word}")]
async fn balance_should_be(world: &mut World, id: IdentityName, amount: BigUint, symbol: String) {
    let identity = world.identity(&id).unwrap().identity;
    let symbol = *world.symbol(&symbol).unwrap();
    let balance = world.balance(identity, symbol).await;
    assert_eq!(balance, TokenAmount::from(amount));
}

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
        .run(".")
        .await;
}
