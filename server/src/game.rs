use std::{
    borrow::Borrow,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::card::{Card, Pairing};
use anyhow::{Context, Ok, Result};
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
    pub initialized: bool,
}
#[derive(Deserialize, Serialize, Clone, Copy)]
pub enum Mode {
    Wa,
    Ding,
    Normal,
}

#[derive(Clone, Serialize, Deserialize)]
enum ClientMessage {
    Ready { turn: u8 },
}
#[derive(Clone, Serialize, Deserialize)]
enum ServerMessage {
    Turn {
        to: Option<u8>,
        turn: u8,
        mode: Mode,
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
        }
    }

    pub fn to(&self) -> Option<u8> {
        match self {
            ServerMessage::Turn { to, .. } => *to,
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

impl Game {
    pub async fn on_connection(&self, socket: WebSocket) {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
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
            ClientMessage::Ready { turn } => todo!(),
        }
        Ok(())
    }
}
