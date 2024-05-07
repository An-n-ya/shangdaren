use serde::{Deserialize, Serialize};

use crate::card::{Card, Pairing};
#[derive(Default, Deserialize, Serialize)]
pub struct Agent {
    pub hand: Vec<Card>,
    pub out: Vec<Card>,
    pub pairing: Vec<Pairing>,
    pub player_right_out: Vec<Card>,
    pub player_right_pairing: Vec<Pairing>,
    pub player_left_out: Vec<Card>,
    pub player_left_pairing: Vec<Pairing>,
    pub round: u8,
    pub turn: u8,
    pub is_robot: bool,
    pub ready: bool,
    pub test: bool,
}

impl Agent {
    pub fn discard_card(&mut self) -> Card {
        let index = if !self.test {
            rand::random::<usize>() % self.hand.len()
        } else {
            0
        };
        let card = *self.hand.get(index).unwrap();
        self.out.push(card);

        self.hand.remove(index);

        card
    }

    pub fn wa_card(&mut self, card: Card) -> bool {
        let res = rand::random::<u8>() % 2 == 1;
        if res {
            self.pairing.push(Pairing::Quadlet(card));
            let mut index = vec![];
            for (i, c) in self.hand.iter().enumerate() {
                if c.is_same_kind(&card) {
                    index.push(i);
                }
            }
            assert!(index.len() == 3);
            for i in 0..3 {
                self.hand.remove(index[i] - i);
            }
        }

        res
    }
    pub fn ding_card(&mut self, card: Card) -> bool {
        let res = rand::random::<u8>() % 2 == 1;
        if res {
            self.pairing.push(Pairing::Triplet(card));
            let mut index = vec![];
            for (i, c) in self.hand.iter().enumerate() {
                if c.is_same_kind(&card) {
                    index.push(i);
                }
            }
            assert!(index.len() == 2);
            for i in 0..2 {
                self.hand.remove(index[i] - i);
            }
        }

        res
    }
}
