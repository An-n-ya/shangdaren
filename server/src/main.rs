use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use agent::Agent;
use card::Card;
use dashmap::DashMap;
use game::{Game, Mode};
use handler::socket_handler;
use redis::Connection;
use room::Room;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::{filters::BoxedFilter, reply::Reply, Filter};

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

#[derive(Serialize)]
struct InitialData {
    hand: Vec<Card>,
    turn: u8,
    cur_turn: u8,
}
#[derive(Deserialize)]
struct DiscardData {
    card: u8,
    turn: u8,
    cur_turn: u8,
}
#[derive(Serialize)]
struct IsMyTurnResponse {
    is_my_turn: bool,
    is_wa: bool,
    is_ding: bool,
    is_normal: bool,
}
#[derive(Serialize)]
struct DrawResponse {
    discard: u8,
    win: bool,
}

#[derive(Deserialize)]
struct DrawData {
    card: u8,
    turn: u8,
}

fn save_agent(session_id: &str, turn: u8, state: &Agent) {
    let state = serde_json::to_string(&state).unwrap();
    let mut con = get_redis_con();
    let _: () = redis::cmd("HSET")
        .arg(&session_id)
        .arg(&format!("{turn}"))
        .arg(&state)
        .query(&mut con)
        .unwrap();
}

fn get_agent(session_id: &str, turn: u8) -> Agent {
    let mut con = get_redis_con();
    let s: String = redis::cmd("HGET")
        .arg(&session_id)
        .arg(&format!("{turn}"))
        .query(&mut con)
        .unwrap();
    let state: Agent = serde_json::from_str(&s).unwrap();
    state
}
fn get_turn(session_id: &str) -> String {
    let mut con = get_redis_con();
    let s: String = redis::cmd("HGET")
        .arg(session_id)
        .arg("turn")
        .query(&mut con)
        .unwrap();
    return s;
}
fn set_turn(session_id: &str, turn: u8) {
    let mut con = get_redis_con();
    let _: () = redis::cmd("HSET")
        .arg(session_id)
        .arg("turn")
        .arg(&format!("{turn}"))
        .query(&mut con)
        .unwrap();
}

fn get_redis_con() -> Connection {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    client.get_connection().unwrap()
}

#[tokio::main]
async fn main() {
    let w = warp::path!("api" / "ws" / String)
        .and(warp::ws())
        .and(warp::any().map(move || GlobalState::new()))
        .and_then(socket_handler);
    warp::serve(w).run(([0, 0, 0, 0], 3131)).await
}
