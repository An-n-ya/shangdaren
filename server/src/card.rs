use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Card(pub u8);

#[derive(Deserialize, Serialize)]
pub enum Pairing {
    Triplet(Card),
    Quadlet(Card),
}
