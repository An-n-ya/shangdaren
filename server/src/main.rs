use std::sync::Arc;

use dashmap::DashMap;
use handler::socket_handler;
use room::Room;
use warp::Filter;

mod agent;
mod card;
mod game;
mod room;

mod handler;

pub struct GlobalState {
    rooms: Arc<DashMap<String, Room>>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            rooms: Default::default(),
        }
    }
}

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.target(env_logger::Target::Stdout);

    builder.init();
    let w = warp::path!("api" / "ws" / String)
        .and(warp::ws())
        .and(warp::any().map(move || GlobalState::new()))
        .and_then(socket_handler);
    warp::serve(w).run(([0, 0, 0, 0], 3131)).await
}
