use serde::{Deserialize, Serialize};

use crate::card::{Card, Pairing};
#[derive(Deserialize, Serialize)]
pub struct Agent {
    pub my_hand: Vec<Card>,
    pub my_out: Vec<Card>,
    pub my_pairing: Vec<Pairing>,
    pub player1_out: Vec<Card>,
    pub player1_pairing: Vec<Pairing>,
    pub player2_out: Vec<Card>,
    pub player2_pairing: Vec<Pairing>,
    pub round: u8,
    pub turn: u8,
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
