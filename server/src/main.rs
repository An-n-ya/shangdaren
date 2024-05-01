use std::{collections::HashMap, sync::Mutex};

use agent::Agent;
use card::Card;
use game::{Game, Mode};
use handlers::room::create_room;
use redis::Connection;
use rocket::serde::{json::Json, Deserialize};
use room::Room;
use serde::Serialize;
use uuid::Uuid;
use ws::Message;

#[macro_use]
extern crate rocket;

mod agent;
mod card;
mod game;
mod room;

mod handlers;

pub struct GlobalState {
    rooms: Mutex<HashMap<String, Room>>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            rooms: Mutex::new(HashMap::new()),
        }
    }

    pub fn handle_message(&self, msg: Message) {}
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct InitialData {
    hand: Vec<Card>,
    turn: u8,
    cur_turn: u8,
}
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct DiscardData {
    card: u8,
    turn: u8,
    cur_turn: u8,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct IsMyTurnResponse {
    is_my_turn: bool,
    is_wa: bool,
    is_ding: bool,
    is_normal: bool,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct DrawResponse {
    discard: u8,
    win: bool,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct DrawData {
    card: u8,
    turn: u8,
}

#[get("/new_game")]
fn new_game() -> String {
    let id = Uuid::new_v4();
    let session_id = id.to_string();
    let mut game = Game::new();
    game.add_player();
    game.add_robot();
    game.add_robot();
    game.start();
    let mut con = get_redis_con();
    let _: () = redis::cmd("HSET")
        .arg("game")
        .arg(&session_id)
        .arg(&serde_json::to_string(&game).unwrap())
        .query(&mut con)
        .unwrap();

    session_id
}

#[get("/is_my_turn/<session_id>/<turn>")]
fn is_my_turn(session_id: &str, turn: u8) -> Json<IsMyTurnResponse> {
    let game = get_game(session_id);
    if let Some(mode) = game.is_turn(turn) {
        let (is_wa, is_ding, is_normal) = match mode {
            Mode::Wa => (true, false, false),
            Mode::Ding => (false, true, false),
            Mode::Normal => (false, false, true),
        };
        Json(IsMyTurnResponse {
            is_my_turn: true,
            is_wa,
            is_ding,
            is_normal,
        })
    } else {
        Json(IsMyTurnResponse {
            is_my_turn: false,
            is_wa: false,
            is_ding: false,
            is_normal: false,
        })
    }
}

#[post("/initial/<session_id>")]
fn initial(session_id: &str) -> Json<InitialData> {
    let mut game = get_game(session_id);
    for i in 0..3 {
        if !game.players[i].initialized {
            game.players[i].initialized = true;
            let res = InitialData {
                hand: game.hand_of_player(i),
                turn: i as u8,
                cur_turn: game.turn,
            };
            save_game(session_id, &game);
            return Json(res);
        }
    }
    unreachable!()
}

#[get("/end_game/<session_id>")]
fn end_game(session_id: &str) {
    let mut con = get_redis_con();
    let _: () = redis::cmd("HDEL")
        .arg("game")
        .arg(&session_id)
        .query(&mut con)
        .unwrap();
}

#[post("/turn/<session_id>", data = "<data>")]
fn turn(session_id: &str, data: Json<DrawData>) -> Json<DrawResponse> {
    assert!([0, 1, 2].contains(&data.turn));
    let turn = data.turn;
    let card = Card(data.card);

    let mut state = get_agent(session_id, turn);
    state.my_hand.push(card);
    state.round += 1;

    let card = state.discard_card();

    save_agent(session_id, turn, &state);

    Json(DrawResponse {
        discard: card.0,
        win: false,
    })
}

#[post("/discard/<session_id>", data = "<data>")]
fn discard(session_id: &str, data: Json<DiscardData>) {
    let mut state = get_agent(session_id, data.turn);
    state.round += 1;
    if data.cur_turn - state.turn == 1 {
        state.player1_out.push(Card(data.card));
    } else {
        state.player2_out.push(Card(data.card));
    }
    save_agent(session_id, data.turn, &state);
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

fn save_game(session_id: &str, game: &Game) {
    let mut con = get_redis_con();
    let _: () = redis::cmd("HSET")
        .arg(session_id)
        .arg("game")
        .arg(&serde_json::to_string(game).unwrap())
        .query(&mut con)
        .unwrap();
}
fn get_game(session_id: &str) -> Game {
    let mut con = get_redis_con();
    let s: String = redis::cmd("HGET")
        .arg(session_id)
        .arg("game")
        .query(&mut con)
        .unwrap();
    let game: Game = serde_json::from_str(&s).unwrap();
    game
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

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/api",
            routes![new_game, initial, end_game, turn, discard, create_room],
        )
        .manage(GlobalState::new())
}
