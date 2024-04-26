use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Card(pub u8);

#[derive(Deserialize, Serialize)]
pub enum Pairing {
    Triplet(Card),
    Quadlet(Card),
}
