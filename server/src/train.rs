use log::info;

use crate::{agent::Strategy, game::GameState};

struct Train {
    times: usize,
    game: GameState,
    records: Vec<u32>,
}

impl Train {
    pub fn new(times: usize) -> Self {
        let mut game = GameState::default();
        game.add_robot();
        game.add_robot();
        game.add_robot();
        game.players[0].strategy = Strategy::Level1;
        game.players[1].strategy = Strategy::Random;
        game.players[2].strategy = Strategy::Random;
        game.training = true;
        Self {
            times,
            game,
            records: vec![0; 4],
        }
    }

    pub fn run(&mut self) {
        for _ in 0..self.times {
            info!("=========================== new game =========================");
            self.game.start().unwrap();
            loop {
                if self.game.winner.is_some() {
                    break;
                }
                let card = self.game.robot_turn(None);
                if let Some(card) = card {
                    self.game.next_turn(&card);
                }
            }
            if let Some(winner) = self.game.winner {
                if winner == u8::MAX {
                    self.records[3] += 1;
                } else {
                    self.records[winner as usize] += 1;
                }
            }
        }
    }

    pub fn display(&self) {
        println!("id 0: {}", self.records[0]);
        println!("id 1: {}", self.records[1]);
        println!("id 2: {}", self.records[2]);
        println!("even: {}", self.records[3]);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn train() {
        let mut builder = env_logger::Builder::from_default_env();
        builder.target(env_logger::Target::Stdout);
        builder.init();
        let mut train = Train::new(1000);
        train.run();
        train.display();
    }
}
