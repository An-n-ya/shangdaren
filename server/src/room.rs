use std::sync::Arc;

use dashmap::DashSet;
use tokio::sync::broadcast;

use crate::game::Game;

pub struct Room {
    users: Arc<DashSet<User>>,
    pub game: Arc<Game>,
}

#[derive(PartialEq, Eq, Hash)]
pub struct User {
    name: String,
}

impl User {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Room {
    pub fn new() -> Self {
        Self {
            users: Default::default(),
            game: Default::default(),
        }
    }

    pub fn add_user(&mut self, user: User) {
        self.users.insert(user);
    }
}
