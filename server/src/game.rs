use crate::card::{Card, Pairing};
use rocket::serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize)]
pub struct Game {
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
