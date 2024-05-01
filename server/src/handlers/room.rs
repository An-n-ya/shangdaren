use rocket::{
    futures::{SinkExt, StreamExt},
    State,
};
use std::sync::Arc;

use crate::{
    room::{Room, User},
    GlobalState,
};

#[get("/create_room")]
pub fn create_room<'a, 'b>(ws: ws::WebSocket, state: &State<Arc<GlobalState>>) -> ws::Channel<'_>
where
    'a: 'b,
{
    ws.channel(move |mut stream| {
        Box::pin(async move {
            let mut is_success = false;
            {
                let name = "";
                let user = "";
                let mut rooms = state.rooms.lock().unwrap();
                if !rooms.contains_key(name) {
                    let user = User::new(&user);
                    let mut room = Room::new();
                    room.add_user(user);
                    rooms.insert(name.to_string(), room);
                    is_success = true;
                }
            }
            if !is_success {
                let message = format!("room name already exist");
                stream.send(message.into()).await.unwrap();
            }

            while let Some(msg) = stream.next().await {
                if let Ok(msg) = msg {
                    state.handle_message(msg)
                } else {
                    break;
                }
            }

            return Ok(());
        })
    })
}
