use cucumber::given;

use crate::world::World;

#[given(expr = "the server has a heartbeat")]
async fn heartbeat(world: &mut World) {
    world.base_client().heartbeat().await.unwrap();
}

#[given(expr = "the server has a status")]
async fn status(world: &mut World) {
    let status = world.base_client().status().await.unwrap();
    assert!(status.version > 0);
}

#[given(expr = "the server has endpoints")]
async fn endpoints(world: &mut World) {
    let endpoints = world.base_client().endpoints().await.unwrap();
    assert!(!endpoints.0.is_empty())
}
