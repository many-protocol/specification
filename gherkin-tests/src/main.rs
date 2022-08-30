use std::sync::Arc;

use cucumber::WorldInit;
use opts::{read_spec_config, CmdOpts};
use world::World;

mod cose;
mod opts;
pub mod params;
pub mod steps;
mod world;

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
