use log::debug;
use warp::{filters::ws::Ws, reject::Rejection, reply::Reply};

use crate::{room::Room, GlobalState};

pub async fn socket_handler(
    id: String,
    ws: Ws,
    state: GlobalState,
) -> Result<impl Reply, Rejection> {
    use dashmap::mapref::entry::Entry;
    debug!("id: {id}");

    let entry = match state.rooms.entry(id.clone()) {
        Entry::Occupied(e) => e.into_ref(),
        Entry::Vacant(e) => {
            let room = Room::new();
            e.insert(room)
        }
    };

    let room = entry.value();
    let game = room.game.clone();

    Ok(ws.on_upgrade(|socket| async move { game.on_connection(socket).await }))
}
