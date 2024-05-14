use core::num;
use std::collections::HashMap;

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
    prob: HashMap<u8, u8>,
    remaining: u8,
}

fn divide_into_group(hand: &Vec<Card>) -> Vec<Vec<Card>> {
    let mut group = vec![];
    let mut tmp = vec![];
    for i in 0..96 {
        if i % 12 == 0 {
            if tmp.len() != 0 {
                group.push(tmp.clone());
            }
            tmp.clear();
        }
        if hand.contains(&Card(i)) {
            tmp.push(Card(i));
        }
    }
    if tmp.len() != 0 {
        group.push(tmp.clone());
    }
    group
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

        self.update_probability();

        card
    }

    pub fn pao_card(&mut self, card: Card) -> bool {
        if self.test {
            return false;
        }
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

        self.update_probability();
        res
    }
    pub fn ding_card(&mut self, card: Card) -> bool {
        let res = if self.test {
            true
        } else {
            rand::random::<u8>() % 2 == 1
        };
        if res {
            self.pairing.push(Pairing::Triplet(card));
            let mut index = vec![];
            for (i, c) in self.hand.iter().enumerate() {
                if c.is_same_kind(&card) {
                    index.push(i);
                }
            }
            assert_eq!(index.len(), 2);
            for i in 0..2 {
                self.hand.remove(index[i] - i);
            }
        }

        self.update_probability();
        res
    }

    pub fn update_probability(&mut self) {
        let mut mmap: HashMap<u8, u8> = HashMap::default();
        for i in 0..24 {
            mmap.insert(i, 4);
        }
        for c in self
            .hand
            .iter()
            .chain(self.out.iter())
            .chain(self.player_left_out.iter())
            .chain(self.player_right_out.iter())
        {
            *mmap.entry(c.0 / 4).or_insert(4) -= 1;
        }
        for p in self
            .pairing
            .iter()
            .chain(self.player_left_pairing.iter())
            .chain(self.player_right_pairing.iter())
        {
            match p {
                Pairing::Triplet(c) => *mmap.entry(c.0 / 4).or_insert(4) -= 3,
                Pairing::Quadlet(c) => *mmap.entry(c.0 / 4).or_insert(4) -= 4,
            }
        }
        self.remaining = mmap.values().fold(0, |acc, x| acc + x);

        self.prob = mmap
    }

    fn get_prob_of(&self, card_type: u8, skip: usize) -> f32 {
        let number = self.prob.get(&card_type).unwrap();
        if *number == 0 {
            return 0.0;
        }
        let mut acc = 0.0;
        for i in 0..(1 << skip) {
            if i & 1 != 1 {
                continue;
            }
            let mut p = 0.0;
            let mut n = *number as f32;
            let mut remaining = self.remaining as f32;
            for k in (0..skip).rev() {
                if n == 0.0 {
                    p = 0.0;
                    break;
                }
                if 1 << k & i != 0 {
                    p *= n / remaining;
                    n -= 1.0;
                } else {
                    p *= 1.0 - n / remaining;
                }
                remaining -= 1.0;
            }
            acc += p;
        }
        acc
    }

    fn form_shun(&self, group: &Vec<Card>) -> f32 {
        fn minus_entry(cnt: &mut HashMap<u8, u8>, key: u8, val: u8) {
            cnt.entry(key).and_modify(|e| *e -= val);
            if cnt[&key] == 0 {
                cnt.remove(&key);
            }
        }
        let mut mmap: HashMap<u8, u8> = HashMap::new();
        for c in group {
            *mmap.entry(c.0 / 4).or_insert(0) += 1;
        }
        let cat = group[0].0 / 12;
        let (i, j, k) = (cat * 3, cat * 3 + 1, cat * 3 + 2);
        if mmap.contains_key(&i) && mmap.contains_key(&j) && mmap.contains_key(&k) {
            minus_entry(&mut mmap, i, 1);
            minus_entry(&mut mmap, j, 1);
            minus_entry(&mut mmap, k, 1);
        }
        if mmap.len() == 0 {
            return f32::MAX;
        }
        let mut need_card = vec![];
        for c in [i, j, k] {
            if !mmap.contains_key(&c) {
                need_card.push(c);
            }
        }
        if need_card.len() == 1 {
            self.get_prob_of(need_card[0], 3)
        } else {
            let (c1, c2) = (need_card[0], need_card[1]);
            let p1 = self.get_prob_of(c1, 3) * self.get_prob_of(c2, 6);
            let p2 = self.get_prob_of(c2, 3) * self.get_prob_of(c1, 6);
            p1 + p2
        }
    }

    fn form_ke(&self, group: &Vec<Card>) -> f32 {
        unimplemented!()
    }
}
