use card::{Card, Pairing};
use redis::Connection;
use rocket::serde::{json::Json, Deserialize};
use serde::Serialize;
use uuid::Uuid;

#[macro_use]
extern crate rocket;

mod card;

#[derive(Deserialize, Serialize)]
struct Agent {
    my_hand: Vec<Card>,
    my_out: Vec<Card>,
    my_pairing: Vec<Pairing>,
    player1_out: Vec<Card>,
    player1_pairing: Vec<Pairing>,
    player2_out: Vec<Card>,
    player2_pairing: Vec<Pairing>,
    round: u8,
    turn: u8,
}

#[derive(Deserialize, Serialize)]
struct Player {
    hand: Vec<Card>,
    out: Vec<Card>,
    pairing: Vec<Pairing>,
    pub is_robot: bool,
    pub initialized: bool,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
enum Mode {
    Wa,
    Ding,
    Normal,
}

#[derive(Deserialize, Serialize)]
struct Game {
    pub players: Vec<Player>,
    remaining_cards: Vec<Card>,
    round: u8,
    turn: u8,
    jing: Card,
    mode: Mode,
}

impl Game {
    const TOTAL: usize = 96;
    pub fn new() -> Self {
        Self {
            players: vec![],
            remaining_cards: (0..96).into_iter().map(|n| Card(n)).collect(),
            round: 0,
            turn: 0,
            jing: Card(0),
            mode: Mode::Normal,
        }
    }

    pub fn add_player(&mut self) {
        self.players.push(Player {
            hand: vec![],
            out: vec![],
            pairing: vec![],
            is_robot: false,
            initialized: false,
        })
    }
    pub fn add_robot(&mut self) {
        self.players.push(Player {
            hand: vec![],
            out: vec![],
            pairing: vec![],
            is_robot: true,
            initialized: true,
        })
    }

    pub fn start(&mut self) {
        if self.players.len() != 3 {
            panic!("wrong players number {}", self.players.len());
        }
        self.shuffle_cards();
        self.jing = Card(rand::random::<u8>() % Self::TOTAL as u8);
        for i in 0..3 {
            for _ in 0..19 {
                self.players[i]
                    .hand
                    .push(self.remaining_cards.pop().unwrap());
            }
        }
        self.turn = rand::random::<u8>() % 3;
    }

    pub fn is_turn(&self, turn: u8) -> Option<Mode> {
        if self.turn == turn {
            Some(self.mode)
        } else {
            None
        }
    }

    fn shuffle_cards(&mut self) {
        for i in 0..96usize {
            let j = rand::random::<usize>() % Self::TOTAL;
            (self.remaining_cards[i], self.remaining_cards[j]) =
                (self.remaining_cards[j], self.remaining_cards[i]);
        }
    }

    pub fn hand_of_player(&self, ind: usize) -> Vec<Card> {
        self.players[ind].hand.clone()
    }
}

impl Agent {
    pub fn new(hand: Vec<Card>, turn: u8) -> Self {
        Self {
            my_hand: hand,
            my_out: vec![],
            my_pairing: vec![],
            player1_out: vec![],
            player1_pairing: vec![],
            player2_out: vec![],
            player2_pairing: vec![],
            round: 0,
            turn,
        }
    }

    pub fn discard_card(&mut self) -> Card {
        let index = rand::random::<usize>() % self.my_hand.len();
        let card = *self.my_hand.get(index).unwrap();
        self.my_out.push(card);

        self.my_hand.remove(index);

        card
    }
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
    rocket::build().mount("/api", routes![new_game, initial, end_game, turn, discard])
}
