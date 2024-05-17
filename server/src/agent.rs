use std::collections::HashMap;

use log::{debug, info};

use crate::{
    card::{Card, Pairing},
    game::GameState,
};

#[derive(Debug)]
pub enum Strategy {
    Random,
    Level1,
    Test,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::Level1
    }
}

#[derive(Default)]
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
    pub strategy: Strategy,
    prob: HashMap<u8, u8>,
    remaining: u8,
    pub jing: Card,
    ting: Option<Vec<u8>>,
}

fn divide_into_group(hand: &Vec<Card>) -> Vec<Vec<Card>> {
    assert!(hand.len() > 0);
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
    pub fn clear(&mut self) {
        self.hand.clear();
        self.out.clear();
        self.pairing.clear();
        self.player_left_out.clear();
        self.player_right_out.clear();
        self.player_left_pairing.clear();
        self.player_right_pairing.clear();
        self.ting = None;
        self.round = 0;
        self.prob.clear();
        self.remaining = 0;
    }
    pub fn discard_card(&mut self) -> Card {
        let index = match self.strategy {
            Strategy::Random => rand::random::<usize>() % self.hand.len(),
            Strategy::Level1 => {
                if self.ting.is_some() {
                    self.ting_card()
                } else {
                    self.choose_discard_card()
                }
            }
            Strategy::Test => 0,
        };
        let card = *self
            .hand
            .get(index)
            .expect(&format!("discard index {}", index));
        info!("robot {} discard {:?}", self.turn, card);
        self.out.push(card);

        self.hand.remove(index);

        self.update_probability();
        self.ting = self.is_ting(&self.hand);

        card
    }

    fn ting_card(&mut self) -> usize {
        let mut hand = self.hand.clone();
        let (mut best_score, mut best_index) = (0, 0);
        for i in 0..hand.len() {
            hand.swap(0, i);

            if let Some(ting_cards) = self.is_ting(&hand[1..]) {
                let mut tmp = 0;
                for c in ting_cards {
                    tmp += self.prob[&c];
                }

                if tmp > best_score {
                    best_score = tmp;
                    best_index = i;
                }
            }

            hand.swap(0, i);
        }
        if best_score == 0 {
            self.ting = None;
            self.choose_discard_card()
        } else {
            best_index
        }
    }

    fn choose_discard_card(&self) -> usize {
        let groups = divide_into_group(&self.hand);
        let mut scores: Vec<f32> = groups
            .iter()
            .map(|group| self.form_ke(group) + self.form_shun(group))
            .collect();
        let (_, ind) = scores
            .iter()
            .enumerate()
            .map(|(ind, value)| ((*value * 1000.0) as usize, ind))
            .min()
            .expect(&format!(
                "cannot find min value in scores: {scores:?}, groups {groups:?}"
            ));

        // debug!(
        //     "[choose discard_card] groups: {:?}, scores: {:?}, choose ind: {ind}",
        //     groups, scores
        // );
        let card = self.select_worst_one_from_group(&groups[ind]);
        self.hand.iter().position(|&c| c == card).unwrap()
    }

    fn select_worst_one_from_group(&self, group: &Vec<Card>) -> Card {
        let mut group = group.clone();
        let mut scores = vec![];
        for i in 0..group.len() {
            group.swap(0, i);

            let score = self.form_ke(&group[1..]) + self.form_shun(&group[1..]);
            scores.push(score);

            group.swap(0, i);
        }
        let (_, ind) = scores
            .iter()
            .enumerate()
            .map(|(ind, value)| ((*value * 1000.0) as usize, ind))
            .min()
            .unwrap();

        group[ind]
    }

    pub fn pao_card(&mut self, card: Card) -> bool {
        let res = match self.strategy {
            Strategy::Random => rand::random::<u8>() % 2 == 1,
            Strategy::Level1 => true,
            Strategy::Test => return false,
        };
        if res {
            self.pairing.push(Pairing::Quadlet(card));
            let mut index = vec![];
            for (i, c) in self.hand.iter().enumerate() {
                if c.is_same_kind(&card) {
                    index.push(i);
                }
            }
            debug!("[pao_card] hand: {:?}", self.hand);
            debug!("[pao_card] discard_card: {:?}", card);
            debug!("[pao_card] index: {:?}", index);
            assert!(index.len() == 3);
            for i in 0..3 {
                self.hand.remove(index[i] - i);
            }
            if let Some(c) = self.player_right_out.last() {
                if c.is_same_kind(&card) {
                    self.player_right_out.pop();
                }
            }
            if let Some(c) = self.player_left_out.last() {
                if c.is_same_kind(&card) {
                    self.player_left_out.pop();
                }
            }
        }

        self.update_probability();
        res
    }
    pub fn ding_card(&mut self, card: Card) -> bool {
        let res = match self.strategy {
            Strategy::Random => rand::random::<u8>() % 2 == 1,
            _ => true,
        };
        if res {
            self.pairing.push(Pairing::Triplet(card));
            let mut index = vec![];
            for (i, c) in self.hand.iter().enumerate() {
                if c.is_same_kind(&card) {
                    index.push(i);
                }
            }
            debug!("[ding_card] hand: {:?}", self.hand);
            debug!("[ding_card] discard_card: {:?}", card);
            debug!("[ding_card] index: {:?}", index);
            assert_eq!(index.len(), 2);
            for i in 0..2 {
                self.hand.remove(index[i] - i);
            }
            if let Some(c) = self.player_right_out.last() {
                if c.is_same_kind(&card) {
                    self.player_right_out.pop();
                }
            }
            if let Some(c) = self.player_left_out.last() {
                if c.is_same_kind(&card) {
                    self.player_left_out.pop();
                }
            }
        }

        self.update_probability();
        res
    }

    fn is_ting(&self, hand: &[Card]) -> Option<Vec<u8>> {
        let mut ting_card = vec![];
        let mut hand = hand.to_vec();
        let mut score = 0;
        for p in &self.pairing {
            match p {
                Pairing::Triplet(_) => score += 2,
                Pairing::Quadlet(_) => score += 6,
            }
        }
        for i in 0..24 {
            if self.prob[&i] == 0 {
                continue;
            }
            let c = Card(i * 4);
            hand.push(c);
            if GameState::is_hu(&hand, score, self.jing) {
                ting_card.push(i);
            }
            hand.pop();
        }
        if ting_card.len() == 0 {
            return None;
        }
        Some(ting_card)
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
            *mmap.entry(c.0 / 4).or_insert(4) = mmap
                .entry(c.0 / 4)
                .or_insert(4)
                .checked_sub(1)
                .expect(&format!("cannot perform update on card {:?}", c));
        }
        // let mut sort_map: Vec<_> = mmap
        //     .iter()
        //     .map(|(key, value)| (key, value))
        //     .filter(|(_, value)| **value != 0)
        //     .collect();
        // sort_map.sort();
        // self.hand.sort();
        // self.out.sort();
        // self.player_left_out.sort();
        // self.player_right_out.sort();
        // debug!("[update_probability] mmap: {sort_map:?}");
        // debug!("[update_probability] hand: {:?}", self.hand);
        // debug!("[update_probability] out: {:?}", self.out);
        // debug!("[update_probability] left_out: {:?}", self.player_left_out);
        // debug!("[update_probability] rigt_out: {:?}", self.player_right_out);
        // debug!("[update_probability] pairing: {:?}", self.pairing);
        // debug!(
        //     "[update_probability] left_pairing: {:?}",
        //     self.player_left_pairing
        // );
        // debug!(
        //     "[update_probability] rigt_pairing: {:?}",
        //     self.player_right_pairing
        // );
        for p in self
            .pairing
            .iter()
            .chain(self.player_left_pairing.iter())
            .chain(self.player_right_pairing.iter())
        {
            match p {
                Pairing::Triplet(c) => {
                    *mmap.entry(c.0 / 4).or_insert(4) = mmap
                        .entry(c.0 / 4)
                        .or_insert(4)
                        .checked_sub(3)
                        .expect(&format!("cannot perform update on card {c:?}"))
                }
                Pairing::Quadlet(c) => {
                    *mmap.entry(c.0 / 4).or_insert(4) = mmap
                        .entry(c.0 / 4)
                        .or_insert(4)
                        .checked_sub(4)
                        .expect(&format!("cannot perform update on card {c:?}"))
                }
            };
        }
        self.remaining = mmap.values().fold(0, |acc, x| acc + x);

        let mut sort_map: Vec<_> = mmap
            .iter()
            .map(|(key, value)| (key, value))
            .filter(|(_, value)| **value != 0)
            .collect();
        sort_map.sort();

        self.prob = mmap
    }

    fn get_prob_of(&self, card_type: u8, skip: usize) -> f32 {
        let number = self.prob.get(&card_type).unwrap();
        // debug!("[get_prob_of] {:?}", self.prob);
        // debug!("    card_type: {card_type}, number {number}");
        if *number == 0 {
            return 0.0;
        }
        let mut acc = 0.0;
        for i in 0..(1 << skip) {
            if i % 2 != 1 {
                continue;
            }
            let mut p = 1.0;
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
            if p != 1.0 {
                acc += p;
            }
            // debug!("        p: {p}, acc {acc}, i: {i:#b}, n: {n}, remaining: {remaining}");
        }
        acc
    }

    fn get_same_card_prob_of(&self, card_type: u8) -> f32 {
        let number = self
            .prob
            .get(&card_type)
            .expect(&format!("cannot find the prob of type {card_type}"));
        if *number == 0 {
            return 0.0;
        }
        let mut acc = 0.0;
        for i in 0..(1 << 6) {
            if i & 1 != 1 || i & (1 << 3) != 1 {
                continue;
            }
            let mut p = 1.0;
            let mut n = *number as f32;
            let mut remaining = self.remaining as f32;
            for k in (0..6).rev() {
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
            if p != 1.0 {
                acc += p;
            }
        }
        acc
    }

    fn form_shun(&self, group: &[Card]) -> f32 {
        if group.len() == 0 {
            return 0.0;
        }
        let mut mmap: HashMap<u8, u8> = HashMap::new();
        for c in group {
            *mmap.entry(c.0 / 4).or_insert(0) += 1;
        }
        let cat = group[0].0 / 12;
        let (i, j, k) = (cat * 3, cat * 3 + 1, cat * 3 + 2);
        while mmap.contains_key(&i) && mmap.contains_key(&j) && mmap.contains_key(&k) {
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
        // debug!("[form_shun], group: {group:?}, mmap: {mmap:?}, need_card: {need_card:?}");
        let mut res = if need_card.len() == 1 {
            self.get_prob_of(need_card[0], 3)
        } else {
            let (c1, c2) = (need_card[0], need_card[1]);
            let p1 = self.get_prob_of(c1, 3) * self.get_prob_of(c2, 6);
            let p2 = self.get_prob_of(c2, 3) * self.get_prob_of(c1, 6);
            // debug!("    p1: {p1}, p2: {p2}");
            p1 + p2
        };
        // debug!("    res: {res}");
        if cat == 0 || self.jing.0 / 12 == cat {
            res *= 2.0;
        }

        res
    }

    fn form_ke(&self, group: &[Card]) -> f32 {
        if group.len() == 0 {
            return 0.0;
        }
        let mut mmap: HashMap<u8, u8> = HashMap::new();
        for c in group {
            *mmap.entry(c.0 / 4).or_insert(0) += 1;
        }
        let cat = group[0].0 / 12;
        let (i, j, k) = (cat * 3, cat * 3 + 1, cat * 3 + 2);
        for c in [i, j, k] {
            if mmap.contains_key(&c) && *mmap.get(&c).unwrap() >= 3 {
                // TODO: how should we handle 4 same card case
                minus_entry(&mut mmap, c, 3);
            }
        }

        if mmap.len() == 0 {
            return f32::MAX;
        }
        let mut res = 0.0;
        for (key, value) in &mmap {
            assert!(*value < 3);
            assert!(*value > 0);
            let cnt = 3 - value;
            let number = *value;
            if cnt == 1 {
                // draw prob
                let p1 = self.get_prob_of(*key, 3);
                // peng prob
                let p2 = number as f32 / (self.remaining + 19 * 2) as f32 * 2.0;
                res = p1 + p2;
            } else if cnt == 2 {
                // draw prob
                let p1 = self.get_same_card_prob_of(*key);
                // 1 draw 1 peng prob
                let p_prob = number as f32 / (self.remaining - 3 + 19 * 2) as f32 * 2.0;
                let p2 = self.get_prob_of(*key, 3) * p_prob;
                res = p1 + p2;
            } else {
                unreachable!()
            }
        }
        if cat == 0 || self.jing.0 / 12 == cat {
            res *= 2.0;
        }
        res
    }
}
fn minus_entry(cnt: &mut HashMap<u8, u8>, key: u8, val: u8) {
    cnt.entry(key).and_modify(|e| *e -= val);
    if cnt[&key] == 0 {
        cnt.remove(&key);
    }
}
