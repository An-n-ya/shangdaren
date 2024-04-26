use card::{Card, Pairing};
use redis::Connection;
use rocket::serde::{json::Json, Deserialize};
use serde::Serialize;
use uuid::Uuid;

#[macro_use]
extern crate rocket;

mod card;

#[derive(Deserialize, Serialize)]
struct GameState {
    my_hand: Vec<Card>,
    my_out: Vec<Card>,
    my_pairing: Vec<Pairing>,
    player1_out: Vec<Card>,
    player1_pairing: Vec<Pairing>,
    player2_out: Vec<Card>,
    player2_pairing: Vec<Pairing>,
    round: u8,
}

impl GameState {
    pub fn new(hand: Vec<Card>) -> Self {
        Self {
            my_hand: hand,
            my_out: vec![],
            my_pairing: vec![],
            player1_out: vec![],
            player1_pairing: vec![],
            player2_out: vec![],
            player2_pairing: vec![],
            round: 0,
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct InitialData {
    hand: Vec<u8>,
    turn: u8,
}

#[get("/new_game")]
fn new_game() -> String {
    let id = Uuid::new_v4();
    id.to_string()
}

#[post("/initial/<session_id>", data = "<data>")]
fn initial(session_id: &str, data: Json<InitialData>) {
    assert!([0, 1, 2].contains(&data.turn));
    let turn = (0x30 + data.turn) as char;
    let mut hand = vec![];
    for n in &data.hand {
        hand.push(Card(*n));
    }
    let state = GameState::new(hand);
    let mut con = get_redis_con();
    let _: () = redis::cmd("HSET")
        .arg(&session_id)
        .arg(&format!("{turn}"))
        .arg(&serde_json::to_string(&state).unwrap())
        .query(&mut con)
        .unwrap();
}

#[get("/end_game/<session_id>")]
fn end_game(session_id: &str) {
    let mut con = get_redis_con();
    for i in ["0", "1", "2"] {
        let _: () = redis::cmd("HDEL")
            .arg(&session_id)
            .arg(i)
            .query(&mut con)
            .unwrap();
    }
}

fn get_redis_con() -> Connection {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    client.get_connection().unwrap()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/api", routes![new_game, initial, end_game])
}
