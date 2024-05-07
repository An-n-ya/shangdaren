use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
    agent::Agent,
    card::{Card, Pairing},
};
use anyhow::{bail, Context, Ok, Result};
use futures::prelude::*;
use log::{debug, warn};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{self, Sender};
use warp::ws::{Message, WebSocket};

pub struct Game {
    count: AtomicU8,
    state: RwLock<GameState>,
    connection: broadcast::Sender<ServerMessage>,
}

struct GameState {
    pub players: Vec<Agent>,
    remaining_cards: Vec<Card>,
    round: u8,
    pub turn: u8,
    prev_turn: Option<u8>,
    jing: Card,
    mode: Mode,
    test: bool,
}
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Wa(Card),
    Ding(Card),
    Normal,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
enum ClientMessage {
    Ready(bool),
    Test(bool),
    AddRobot(bool),
    Start(bool),
    Discard { card: Card },
    Ding { confirm: bool },
    Wa { confirm: bool },
}
#[derive(Clone, Serialize, Deserialize, Debug)]
enum ServerMessage {
    Turn {
        to: Option<u8>,
        turn: u8,
        mode: Mode,
    },
    Initial {
        to: Option<u8>,
        cur_turn: u8,
        hand: Vec<Card>,
    },
    Draw {
        to: Option<u8>,
        card: Card,
    },
    Wa {
        to: Option<u8>,
        card: Card,
    },
    Ding {
        to: Option<u8>,
        card: Card,
    },
    Discard {
        to: Option<u8>,
        card: Card,
    },
}

impl From<ServerMessage> for Message {
    fn from(value: ServerMessage) -> Self {
        let serialized = serde_json::to_string(&value).expect("failed to serialize");
        Message::text(serialized)
    }
}

impl ServerMessage {
    pub fn is_broadcast(&self) -> bool {
        match self {
            ServerMessage::Turn { to, .. } => to.is_none(),
            ServerMessage::Initial { to, .. } => to.is_none(),
            ServerMessage::Draw { to, .. } => to.is_none(),
            ServerMessage::Discard { to, .. } => to.is_none(),
            ServerMessage::Wa { to, .. } => to.is_none(),
            ServerMessage::Ding { to, .. } => to.is_none(),
        }
    }

    pub fn to(&self) -> Option<u8> {
        match self {
            ServerMessage::Turn { to, .. } => *to,
            ServerMessage::Initial { to, .. } => *to,
            ServerMessage::Draw { to, .. } => *to,
            ServerMessage::Discard { to, .. } => *to,
            ServerMessage::Wa { to, .. } => *to,
            ServerMessage::Ding { to, .. } => *to,
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            count: Default::default(),
            state: Default::default(),
            connection: broadcast::channel(16).0,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            players: vec![],
            remaining_cards: (0..Self::TOTAL)
                .into_iter()
                .map(|n| Card(n as u8))
                .collect(),
            round: 0,
            turn: 0,
            prev_turn: None,
            jing: Card(0),
            mode: Mode::Normal,
            test: false,
        }
    }
}
impl GameState {
    const TOTAL: usize = 96;
    const PLAYER_NUM: u8 = 3;
    pub fn add_player(&mut self) {
        self.players.push(Agent::default())
    }
    pub fn add_robot(&mut self) {
        let mut agent = Agent::default();
        agent.is_robot = true;
        agent.ready = true;
        if self.test {
            agent.test = true;
        }
        self.players.push(agent)
    }

    pub fn start(&mut self) -> Result<()> {
        if self.players.len() != 3 {
            bail!("wrong players number {}", self.players.len());
        }
        if !self.test {
            self.shuffle_cards();
            self.jing = Card(rand::random::<u8>() % Self::TOTAL as u8);
            self.turn = rand::random::<u8>() % 3;
        } else {
            self.remaining_cards.reverse();
            self.jing = Card(95);
            self.turn = 0;
        }
        for i in 0..3 {
            for _ in 0..19 {
                self.players[i]
                    .hand
                    .push(self.remaining_cards.pop().unwrap());
            }
        }
        debug!("current turn {}", self.turn);
        Ok(())
    }

    pub fn restore_turn(&mut self) -> ServerMessage {
        self.turn = self.prev_turn.expect("missing previous turn");
        ServerMessage::Turn {
            to: None,
            turn: self.turn,
            mode: Mode::Normal,
        }
    }

    pub fn next_turn(&mut self, discard: &Card) -> ServerMessage {
        let next_player = ((self.turn + 1) % Self::PLAYER_NUM) as usize;
        let prev_player = ((next_player as u8 + 1) % Self::PLAYER_NUM) as usize;
        if discard.0 == 19 {
            debug!("next_player {}, prev_player {}", next_player, prev_player);
            debug!(
                "can next player form qualet {}",
                Self::can_form_quadlet(&self.players[next_player].hand, discard)
            );
            debug!(
                "can prev player form qualet {}",
                Self::can_form_quadlet(&self.players[prev_player].hand, discard)
            );
        }
        self.prev_turn = Some((self.turn + 1) % Self::PLAYER_NUM);
        if Self::can_form_triplet(&self.players[next_player].hand, discard) {
            self.turn = next_player as u8;
            self.mode = Mode::Ding(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Ding(*discard),
            }
        } else if Self::can_form_quadlet(&self.players[next_player].hand, discard) {
            self.turn = next_player as u8;
            self.mode = Mode::Wa(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Wa(*discard),
            }
        } else if Self::can_form_triplet(&self.players[prev_player].hand, discard) {
            self.turn = prev_player as u8;
            self.mode = Mode::Ding(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Ding(*discard),
            }
        } else if Self::can_form_quadlet(&self.players[prev_player].hand, discard) {
            self.turn = prev_player as u8;
            self.mode = Mode::Wa(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Wa(*discard),
            }
        } else {
            self.turn = (self.turn + 1) % Self::PLAYER_NUM;
            self.mode = Mode::Normal;
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Normal,
            }
        }
    }

    pub fn draw_card(&mut self) -> ServerMessage {
        let card = self.remaining_cards.pop().unwrap();
        self.players[self.turn as usize].hand.push(card);
        ServerMessage::Draw {
            to: Some(self.turn),
            card,
        }
    }

    pub fn is_robot_turn(&self) -> bool {
        self.players[self.turn as usize].is_robot
    }

    pub fn robot_turn(&mut self, con: &Sender<ServerMessage>) -> Option<Card> {
        assert!(self.is_robot_turn());
        let mut card = Card(0);
        let right = (self.turn + 1) % Self::PLAYER_NUM;
        let left = (right + 1) % Self::PLAYER_NUM;

        enum Pos {
            Right,
            Left,
        }

        debug!("mode: {:?}", self.mode);

        match self.mode {
            m @ Mode::Wa(discard) => {
                if self.players[self.turn as usize].wa_card(discard) {
                    for (pos, n) in [(Pos::Right, right), (Pos::Left, left)] {
                        let is_robot = match pos {
                            Pos::Right => {
                                if self.players[right as usize].is_robot {
                                    self.players[right as usize]
                                        .player_left_pairing
                                        .push(Pairing::Quadlet(discard));
                                    true
                                } else {
                                    false
                                }
                            }
                            Pos::Left => {
                                if self.players[left as usize].is_robot {
                                    self.players[left as usize]
                                        .player_right_pairing
                                        .push(Pairing::Quadlet(discard));
                                    true
                                } else {
                                    false
                                }
                            }
                        };
                        if !is_robot {
                            con.send(ServerMessage::Turn {
                                to: Some(n),
                                turn: self.turn,
                                mode: m,
                            })
                            .ok();
                        }
                    }
                    let draw_card = self.remaining_cards.pop().unwrap();
                    self.players[self.turn as usize].hand.push(draw_card);
                    card = self.players[self.turn as usize].discard_card();
                } else {
                    let msg = self.restore_turn();
                    con.send(msg).ok();
                    return None;
                }
            }
            m @ Mode::Ding(discard) => {
                if self.players[self.turn as usize].ding_card(discard) {
                    for (pos, n) in [(Pos::Right, right), (Pos::Left, left)] {
                        let is_robot = match pos {
                            Pos::Right => {
                                if self.players[right as usize].is_robot {
                                    self.players[right as usize]
                                        .player_left_pairing
                                        .push(Pairing::Triplet(discard));
                                    true
                                } else {
                                    false
                                }
                            }
                            Pos::Left => {
                                if self.players[left as usize].is_robot {
                                    self.players[left as usize]
                                        .player_right_pairing
                                        .push(Pairing::Triplet(discard));
                                    true
                                } else {
                                    false
                                }
                            }
                        };
                        if !is_robot {
                            con.send(ServerMessage::Turn {
                                to: Some(n),
                                turn: self.turn,
                                mode: m,
                            })
                            .ok();
                        }
                    }
                    card = self.players[self.turn as usize].discard_card();
                } else {
                    let msg = self.restore_turn();
                    con.send(msg).ok();
                    return None;
                }
            }
            Mode::Normal => {
                self.draw_card();
                card = self.players[self.turn as usize].discard_card();
            }
        }
        for (pos, n) in [(Pos::Right, right), (Pos::Left, left)] {
            let player = &mut self.players[n as usize];
            if player.is_robot {
                match pos {
                    Pos::Right => player.player_left_out.push(card),
                    Pos::Left => player.player_right_out.push(card),
                }
            } else {
                con.send(ServerMessage::Discard { to: Some(n), card }).ok();
            }
        }
        Some(card)
    }

    fn can_form_quadlet(hand: &Vec<Card>, card: &Card) -> bool {
        Self::count_same_type(hand, card) == 3
    }
    fn can_form_triplet(hand: &Vec<Card>, card: &Card) -> bool {
        Self::count_same_type(hand, card) == 2
    }

    fn count_same_type(hand: &Vec<Card>, card: &Card) -> u8 {
        let mut cnt = 0;
        for c in hand {
            if c.is_same_kind(card) {
                cnt += 1;
            }
        }
        cnt
    }

    fn shuffle_cards(&mut self) {
        for i in 0..96usize {
            let j = rand::random::<usize>() % Self::TOTAL;
            (self.remaining_cards[i], self.remaining_cards[j]) =
                (self.remaining_cards[j], self.remaining_cards[i]);
        }
    }

    pub fn hand_of_player(&self, ind: usize) -> Vec<Card> {
        self.players[ind].hand.clone()
    }

    pub fn discard_card(&mut self, player_id: usize, card: Card) -> Result<()> {
        let index = {
            let mut index = None;
            for (i, v) in self.players[player_id].hand.iter().enumerate() {
                if *v == card {
                    index = Some(i);
                    break;
                }
            }
            index
        };
        if let Some(index) = index {
            self.players[player_id].hand.remove(index);
            Ok(())
        } else {
            bail!("cannot find card {card:?} in player {player_id}");
        }
    }
}

impl Game {
    pub async fn on_connection(&self, socket: WebSocket) {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        self.state.write().add_player();
        if let Err(e) = self.handle_connection(id, socket).await {
            warn!("connection terminated because of {e}");
        }
        self.state.write().players.remove(id as usize);
    }

    async fn handle_connection(&self, id: u8, mut socket: WebSocket) -> Result<()> {
        let mut rx = self.connection.subscribe();

        loop {
            tokio::select! {
                update = rx.recv() => {
                    let update = update.unwrap();
                    debug!("server message {update:?}");
                    if update.is_broadcast() || update.to().is_some() && update.to().unwrap() == id {
                        socket.send(update.into()).await?;
                    }
                }
                result = socket.next() => {
                    match result {
                        None => break,
                        Some(message) => {
                            self.handle_message(id, message?).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_message(&self, id: u8, message: Message) -> Result<()> {
        debug!("message {}", message.to_str().unwrap());
        let message: ClientMessage = match message.to_str() {
            Err(()) => return Ok(()),
            ::std::result::Result::Ok(text) => {
                serde_json::from_str(text).context("failed to deserialized client message")?
            }
        };
        debug!("message {message:?}");
        match message {
            ClientMessage::Test(_) => {
                self.state.write().test = true;
            }
            ClientMessage::Ready(_) => {
                self.state.write().players[id as usize].ready = true;
            }
            ClientMessage::AddRobot(_) => {
                self.state.write().add_robot();
            }
            ClientMessage::Start(_) => {
                self.state.write().start()?;
                {
                    let state = self.state.read();
                    for i in 0..3 {
                        self.connection
                            .send(ServerMessage::Initial {
                                to: Some(i),
                                cur_turn: state.turn,
                                hand: state.hand_of_player(i as usize),
                            })
                            .ok();
                    }
                }
                debug!("initial messages have sent");
                loop {
                    let is_robot_turn = self.state.read().is_robot_turn();
                    debug!("is robot turn {is_robot_turn}");
                    if !self.state.read().is_robot_turn() {
                        break;
                    }
                    let card = self.state.write().robot_turn(&self.connection);
                    let msg = if let Some(card) = card {
                        self.state.write().next_turn(&card)
                    } else {
                        ServerMessage::Turn {
                            to: None,
                            turn: self.state.read().turn,
                            mode: Mode::Normal,
                        }
                    };
                    self.connection.send(msg).ok();
                }

                if Mode::Normal == self.state.read().mode {
                    let msg = self.state.write().draw_card();
                    debug!("write draw card message success");
                    self.connection.send(msg).ok();
                }
            }
            ClientMessage::Discard { card } => {
                self.state.write().discard_card(id as usize, card)?;
                let mut card = Some(card);
                loop {
                    debug!("next turn, discard card {:?}", card);
                    let msg = if let Some(card) = card {
                        self.state.write().next_turn(&card)
                    } else {
                        ServerMessage::Turn {
                            to: None,
                            turn: self.state.read().turn,
                            mode: Mode::Normal,
                        }
                    };
                    debug!("next turn, msg {:?}", msg);
                    self.connection.send(msg).ok();
                    if !self.state.read().is_robot_turn() {
                        break;
                    }
                    card = self.state.write().robot_turn(&self.connection);
                }

                if Mode::Normal == self.state.read().mode {
                    let msg = self.state.write().draw_card();
                    self.connection.send(msg).ok();
                }
            }
            ClientMessage::Ding { confirm } => {
                let mode = self.state.read().mode.clone();
                self.state.write().mode = Mode::Normal;
                if !confirm {
                    let msg = self.state.write().restore_turn();
                    self.connection.send(msg).ok();
                    while self.state.read().is_robot_turn() {
                        self.state.write().robot_turn(&self.connection);
                    }
                } else {
                    let card = match mode {
                        Mode::Ding(c) => c,
                        _ => bail!("wrong mode, expect Ding mode, got {:?}", mode),
                    };
                    let msg = ServerMessage::Ding { to: None, card };
                    self.connection.send(msg).ok();
                }
                let msg = self.state.write().draw_card();
                self.connection.send(msg).ok();
            }
            ClientMessage::Wa { confirm } => {
                let mode = self.state.read().mode.clone();
                self.state.write().mode = Mode::Normal;
                if !confirm {
                    let msg = self.state.write().restore_turn();
                    self.connection.send(msg).ok();
                    while self.state.read().is_robot_turn() {
                        self.state.write().robot_turn(&self.connection);
                    }
                } else {
                    let card = match mode {
                        Mode::Wa(c) => c,
                        _ => bail!("wrong mode, expect Wa mode, got {:?}", mode),
                    };
                    let msg = ServerMessage::Wa { to: None, card };
                    self.connection.send(msg).ok();
                }
                let msg = self.state.write().draw_card();
                self.connection.send(msg).ok();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use warp::{test::WsClient, Filter};

    use crate::{handler::socket_handler, GlobalState};

    use super::*;

    struct JsonWsClient(WsClient);
    impl JsonWsClient {
        pub async fn send(&mut self, msg: ClientMessage) {
            self.0.send_text(serde_json::to_string(&msg).unwrap()).await
        }
        pub async fn recv(&mut self) -> ServerMessage {
            let s_msg = self.0.recv().await.unwrap();
            let s_msg = s_msg.to_str().unwrap();
            let s_msg: ServerMessage = serde_json::from_str(s_msg).unwrap();
            s_msg
        }
    }

    async fn connect() -> JsonWsClient {
        let mut builder = env_logger::Builder::from_default_env();
        builder.target(env_logger::Target::Stdout);

        builder.init();
        let w = warp::path!("api" / "ws" / String)
            .and(warp::ws())
            .and(warp::any().map(move || GlobalState::new()))
            .and_then(socket_handler);
        let client = warp::test::ws()
            .path(&format!("/api/ws/{}", "test"))
            .handshake(w)
            .await
            .unwrap();
        JsonWsClient(client)
    }

    // #[tokio::test]
    async fn basic_test1() {
        let mut client = connect().await;
        client.send(ClientMessage::Test(true)).await;
        client.send(ClientMessage::Ready(true)).await;
        for _ in 0..2 {
            client.send(ClientMessage::AddRobot(true)).await;
        }
        client.send(ClientMessage::Start(true)).await;
        let s_msg = client.recv().await;
        let initial_hand: Vec<Card> = (0..19).into_iter().map(|n| Card(n)).collect();
        let is_initial = match s_msg {
            ServerMessage::Initial { to, cur_turn, hand } => {
                assert_eq!(to.unwrap(), 0);
                assert_eq!(cur_turn, 0);
                assert_eq!(hand, initial_hand);
                true
            }
            _ => false,
        };
        assert_eq!(is_initial, true);

        let draw_msg = client.recv().await;
        match draw_msg {
            ServerMessage::Draw { to, card } => {
                assert_eq!(to.unwrap(), 0);
                assert_eq!(card, Card(57));
            }
            _ => panic!("expect draw message, got {:?}", draw_msg),
        }
        client.send(ClientMessage::Discard { card: Card(57) }).await;
        let turn_msg = client.recv().await;
        match turn_msg {
            ServerMessage::Turn { to, turn, mode } => {
                assert!(to.is_none());
                assert_eq!(turn, 1);
                assert_eq!(mode, Mode::Normal);
            }
            _ => panic!("expect turn message, got {:?}", turn_msg),
        }
        let discard_msg = client.recv().await;
        match discard_msg {
            ServerMessage::Discard { to, card } => {
                assert_eq!(to.unwrap(), 0);
                assert_eq!(card, Card(19));
            }
            _ => panic!("expect discard message, got {:?}", discard_msg),
        }
        let turn_msg = client.recv().await;
        println!("{:?}", turn_msg);
        match turn_msg {
            ServerMessage::Turn { to, turn, mode } => {
                assert!(to.is_none());
                assert_eq!(turn, 0);
                assert_eq!(mode, Mode::Wa(Card(19)));
            }
            _ => panic!("expect turn message, got {:?}", turn_msg),
        }
        client.send(ClientMessage::Wa { confirm: false }).await;
    }

    #[tokio::test]
    async fn basic_test2() {
        let mut client = connect().await;
        client.send(ClientMessage::Test(true)).await;
        client.send(ClientMessage::Ready(true)).await;
        for _ in 0..2 {
            client.send(ClientMessage::AddRobot(true)).await;
        }
        client.send(ClientMessage::Start(true)).await;
        let s_msg = client.recv().await;
        let initial_hand: Vec<Card> = (0..19).into_iter().map(|n| Card(n)).collect();
        let is_initial = match s_msg {
            ServerMessage::Initial { to, cur_turn, hand } => {
                assert_eq!(to.unwrap(), 0);
                assert_eq!(cur_turn, 0);
                assert_eq!(hand, initial_hand);
                true
            }
            _ => false,
        };
        assert_eq!(is_initial, true);

        let draw_msg = client.recv().await;
        match draw_msg {
            ServerMessage::Draw { to, card } => {
                assert_eq!(to.unwrap(), 0);
                assert_eq!(card, Card(57));
            }
            _ => panic!("expect draw message, got {:?}", draw_msg),
        }
        client.send(ClientMessage::Discard { card: Card(57) }).await;
        let turn_msg = client.recv().await;
        match turn_msg {
            ServerMessage::Turn { to, turn, mode } => {
                assert!(to.is_none());
                assert_eq!(turn, 1);
                assert_eq!(mode, Mode::Normal);
            }
            _ => panic!("expect turn message, got {:?}", turn_msg),
        }
        let discard_msg = client.recv().await;
        match discard_msg {
            ServerMessage::Discard { to, card } => {
                assert_eq!(to.unwrap(), 0);
                assert_eq!(card, Card(19));
            }
            _ => panic!("expect discard message, got {:?}", discard_msg),
        }
        let turn_msg = client.recv().await;
        println!("{:?}", turn_msg);
        match turn_msg {
            ServerMessage::Turn { to, turn, mode } => {
                assert!(to.is_none());
                assert_eq!(turn, 0);
                assert_eq!(mode, Mode::Wa(Card(19)));
            }
            _ => panic!("expect turn message, got {:?}", turn_msg),
        }
        client.send(ClientMessage::Wa { confirm: true }).await;
        let wa_msg = client.recv().await;
        match wa_msg {
            ServerMessage::Wa { to, card } => {
                assert!(to.is_none());
                assert_eq!(card, Card(19));
            }
            _ => panic!("expect wa message, got {:?}", wa_msg),
        }
        let draw_msg = client.recv().await;
        match draw_msg {
            ServerMessage::Draw { to, card } => {
                assert_eq!(to.unwrap(), 0);
                assert_eq!(card, Card(59));
            }
            _ => panic!("expect draw message, got {:?}", draw_msg),
        }
        // let turn_msg = client.recv().await;
        // match turn_msg {
        //     ServerMessage::Turn { to, turn, mode } => {
        //         assert!(to.is_none());
        //         assert_eq!(turn, 2);
        //         assert_eq!(mode, Mode::Normal);
        //     }
        //     _ => panic!("expect turn message, got {:?}", turn_msg),
        // }
        // let discard_msg = client.recv().await;
        // match discard_msg {
        //     ServerMessage::Discard { to, card } => {
        //         assert_eq!(to.unwrap(), 0);
        //         assert_eq!(card, Card(38));
        //     }
        //     _ => panic!("expect discard message, got {:?}", discard_msg),
        // }
    }
}
