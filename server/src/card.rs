use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Card(pub u8);

#[derive(Deserialize, Serialize, Debug)]
pub enum Pairing {
    Triplet(Card),
    Quadlet(Card),
}

impl Card {
    pub fn is_same_kind(&self, other: &Card) -> bool {
        self.0 / 4 == other.0 / 4
    }
}
