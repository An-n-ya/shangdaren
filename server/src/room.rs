use std::sync::Mutex;

use tokio::sync::broadcast;
use ws::stream::DuplexStream;

pub struct Room {
    users: Mutex<Vec<User>>,
}

pub struct User {
    name: String,
    connection: broadcast::Sender<String>,
}

impl User {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            connection: broadcast::channel(16).0,
        }
    }
}

impl Room {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(vec![]),
        }
    }

    pub fn add_user(&mut self, user: User) {
        let mut users = self.users.lock().unwrap();
        users.push(user)
    }
}
