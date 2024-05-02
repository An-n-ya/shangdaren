use std::{
    borrow::Borrow,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::card::{Card, Pairing};
use anyhow::{bail, Context, Ok, Result};
use futures::prelude::*;
use log::warn;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use warp::ws::{Message, WebSocket};

pub struct Game {
    count: AtomicU8,
    state: RwLock<GameState>,
    connection: broadcast::Sender<ServerMessage>,
}

struct GameState {
    pub players: Vec<Player>,
    remaining_cards: Vec<Card>,
    round: u8,
    pub turn: u8,
    jing: Card,
    mode: Mode,
}
#[derive(Deserialize, Serialize)]
pub struct Player {
    hand: Vec<Card>,
    out: Vec<Card>,
    pairing: Vec<Pairing>,
    pub is_robot: bool,
    pub ready: bool,
}
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Wa,
    Ding,
    Normal,
}

#[derive(Clone, Serialize, Deserialize)]
enum ClientMessage {
    Ready,
    AddRobot,
    Start,
    Discard { card: Card },
}
#[derive(Clone, Serialize, Deserialize)]
enum ServerMessage {
    Turn {
        to: Option<u8>,
        turn: u8,
        mode: Mode,
    },
    Initial {
        to: Option<u8>,
        cur_turn: u8,
        hand: Vec<Card>,
    },
    Draw {
        to: Option<u8>,
        card: Card,
    },
}

impl From<ServerMessage> for Message {
    fn from(value: ServerMessage) -> Self {
        let serialized = serde_json::to_string(&value).expect("failed to serialize");
        Message::text(serialized)
    }
}

impl ServerMessage {
    pub fn is_broadcast(&self) -> bool {
        match self {
            ServerMessage::Turn { to, .. } => to.is_none(),
            ServerMessage::Initial { to, .. } => to.is_none(),
            ServerMessage::Draw { to, .. } => to.is_none(),
        }
    }

    pub fn to(&self) -> Option<u8> {
        match self {
            ServerMessage::Turn { to, .. } => *to,
            ServerMessage::Initial { to, .. } => *to,
            ServerMessage::Draw { to, .. } => *to,
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            count: Default::default(),
            state: Default::default(),
            connection: broadcast::channel(16).0,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            players: vec![],
            remaining_cards: (0..Self::TOTAL)
                .into_iter()
                .map(|n| Card(n as u8))
                .collect(),
            round: 0,
            turn: 0,
            jing: Card(0),
            mode: Mode::Normal,
        }
    }
}
impl GameState {
    const TOTAL: usize = 96;
    const PLAYER_NUM: u8 = 3;
    pub fn add_player(&mut self) {
        self.players.push(Player {
            hand: vec![],
            out: vec![],
            pairing: vec![],
            is_robot: false,
            ready: false,
        })
    }
    pub fn add_robot(&mut self) {
        self.players.push(Player {
            hand: vec![],
            out: vec![],
            pairing: vec![],
            is_robot: true,
            ready: true,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.players.len() != 3 {
            bail!("wrong players number {}", self.players.len());
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
        Ok(())
    }

    pub fn is_turn(&self, turn: u8) -> Option<Mode> {
        if self.turn == turn {
            Some(self.mode)
        } else {
            None
        }
    }

    pub fn next_turn(&mut self, discard: &Card) -> ServerMessage {
        let next_player = ((self.turn + 1) % Self::PLAYER_NUM) as usize;
        let prev_player = ((self.turn + 1) % Self::PLAYER_NUM) as usize;
        if Self::can_form_triplet(&self.players[next_player].hand, discard) {
            self.turn = next_player as u8;
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Ding,
            }
        } else if Self::can_form_quadlet(&self.players[next_player].hand, discard) {
            self.turn = next_player as u8;
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Wa,
            }
        } else if Self::can_form_triplet(&self.players[prev_player].hand, discard) {
            self.turn = prev_player as u8;
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Ding,
            }
        } else if Self::can_form_quadlet(&self.players[prev_player].hand, discard) {
            self.turn = prev_player as u8;
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Wa,
            }
        } else {
            self.turn = (self.turn + 1) % Self::PLAYER_NUM;
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Normal,
            }
        }
    }

    pub fn draw_card(&mut self) -> ServerMessage {
        let card = self.remaining_cards.pop().unwrap();
        self.players[self.turn as usize].hand.push(card);
        ServerMessage::Draw {
            to: Some(self.turn),
            card,
        }
    }

    fn can_form_quadlet(hand: &Vec<Card>, card: &Card) -> bool {
        Self::count_same_type(hand, card) == 4
    }
    fn can_form_triplet(hand: &Vec<Card>, card: &Card) -> bool {
        Self::count_same_type(hand, card) == 3
    }

    fn count_same_type(hand: &Vec<Card>, card: &Card) -> u8 {
        let mut cnt = 0;
        for c in hand {
            if c.is_same_kind(card) {
                cnt += 1;
            }
        }
        cnt
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

    pub fn discard_card(&mut self, player_id: usize, card: Card) -> Result<()> {
        let index = {
            let mut index = None;
            for (i, v) in self.players[player_id].hand.iter().enumerate() {
                if *v == card {
                    index = Some(i);
                    break;
                }
            }
            index
        };
        if let Some(index) = index {
            self.players[player_id].hand.remove(index);
            Ok(())
        } else {
            bail!("cannot find card {card:?} in player {player_id}");
        }
    }
}

impl Game {
    pub async fn on_connection(&self, socket: WebSocket) {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        self.state.write().add_player();
        if let Err(e) = self.handle_connection(id, socket).await {
            warn!("connection terminated because of {e}");
        }
        self.state.write().players.remove(id as usize);
    }

    async fn handle_connection(&self, id: u8, mut socket: WebSocket) -> Result<()> {
        let mut rx = self.connection.subscribe();

        loop {
            tokio::select! {
                update = rx.recv() => {
                    let update = update.unwrap();
                    if update.is_broadcast() || update.to().is_some() && update.to().unwrap() == id {
                        socket.send(update.into()).await?;
                    }
                }
                result = socket.next() => {
                    match result {
                        None => break,
                        Some(message) => {
                            self.handle_message(id, message?).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_message(&self, id: u8, message: Message) -> Result<()> {
        let message: ClientMessage = match message.to_str() {
            Err(()) => return Ok(()),
            ::std::result::Result::Ok(text) => {
                serde_json::from_str(text).context("failed to deserialized client message")?
            }
        };
        match message {
            ClientMessage::Ready => {
                self.state.write().players[id as usize].ready = true;
            }
            ClientMessage::AddRobot => {
                self.state.write().add_robot();
            }
            ClientMessage::Start => {
                self.state.write().start()?;
                let state = self.state.read();
                for i in 0..3 {
                    self.connection
                        .send(ServerMessage::Initial {
                            to: Some(i),
                            cur_turn: state.turn,
                            hand: state.hand_of_player(i as usize),
                        })
                        .ok();
                }
            }
            ClientMessage::Discard { card } => {
                self.state.write().discard_card(id as usize, card)?;
                let msg = self.state.write().next_turn(&card);
                self.connection.send(msg.clone()).ok();

                if let ServerMessage::Turn { mode, .. } = msg {
                    if mode == Mode::Normal {
                        let msg = self.state.write().draw_card();
                        self.connection.send(msg).ok();
                    }
                }
            }
        }
        Ok(())
    }
}
