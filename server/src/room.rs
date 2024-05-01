use std::sync::Mutex;

use ws::stream::DuplexStream;

pub struct Room {
    users: Mutex<Vec<User>>,
}

pub struct User {
    name: String,
    connection: Mutex<DuplexStream>,
}
