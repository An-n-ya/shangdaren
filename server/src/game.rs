use std::{
    collections::HashMap,
    sync::atomic::{AtomicU8, Ordering},
};

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
    #[allow(unused)]
    round: u8,
    pub turn: u8,
    prev_turn: Option<u8>,
    jing: Card,
    mode: Mode,
    test: bool,
}
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Pao(Card),
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
    Pao { confirm: bool },
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
    Pao {
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
    Hu {
        to: Option<u8>,
    },
    End {
        to: Option<u8>,
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
            ServerMessage::Pao { to, .. } => to.is_none(),
            ServerMessage::Ding { to, .. } => to.is_none(),
            ServerMessage::Hu { to, .. } => to.is_none(),
            ServerMessage::End { to, .. } => to.is_none(),
        }
    }

    pub fn to(&self) -> Option<u8> {
        match self {
            ServerMessage::Turn { to, .. } => *to,
            ServerMessage::Initial { to, .. } => *to,
            ServerMessage::Draw { to, .. } => *to,
            ServerMessage::Discard { to, .. } => *to,
            ServerMessage::Pao { to, .. } => *to,
            ServerMessage::Ding { to, .. } => *to,
            ServerMessage::Hu { to, .. } => *to,
            ServerMessage::End { to, .. } => *to,
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

    pub fn end(&mut self) {
        self.players.clear();
        self.turn = 0;
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
        self.prev_turn = Some((self.turn + 1) % Self::PLAYER_NUM);
        if Self::can_form_quadlet(&self.players[next_player].hand, discard) {
            self.turn = next_player as u8;
            self.mode = Mode::Pao(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Pao(*discard),
            }
        } else if Self::can_form_quadlet(&self.players[prev_player].hand, discard) {
            self.turn = prev_player as u8;
            self.mode = Mode::Pao(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Pao(*discard),
            }
        } else if Self::can_form_triplet(&self.players[next_player].hand, discard) {
            self.turn = next_player as u8;
            self.mode = Mode::Ding(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Ding(*discard),
            }
        } else if Self::can_form_triplet(&self.players[prev_player].hand, discard) {
            self.turn = prev_player as u8;
            self.mode = Mode::Ding(*discard);
            ServerMessage::Turn {
                to: None,
                turn: self.turn,
                mode: Mode::Ding(*discard),
            }
        } else {
            debug!("[next_turn] self.turn {}", self.turn);
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
        if let Some(card) = self.remaining_cards.pop() {
            self.players[self.turn as usize].hand.push(card);
            ServerMessage::Draw {
                to: Some(self.turn),
                card,
            }
        } else {
            ServerMessage::End { to: None }
        }
    }

    pub fn is_robot_turn(&self) -> bool {
        self.players[self.turn as usize].is_robot
    }

    pub fn robot_turn(&mut self, con: &Sender<ServerMessage>) -> Option<Card> {
        assert!(self.is_robot_turn());
        #[allow(unused)]
        let mut card = Card(0);
        let right = (self.turn + 1) % Self::PLAYER_NUM;
        let left = (right + 1) % Self::PLAYER_NUM;

        enum Pos {
            Right,
            Left,
        }

        debug!("[robot_turn] mode: {:?}", self.mode);

        match self.mode {
            Mode::Pao(discard) => {
                if self.players[self.turn as usize].pao_card(discard) {
                    for (pos, _) in [(Pos::Right, right), (Pos::Left, left)] {
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
                            con.send(ServerMessage::Pao {
                                to: None,
                                card: discard,
                            })
                            .ok();
                        }
                    }
                    let draw_card = self.remaining_cards.pop().unwrap();
                    self.players[self.turn as usize].hand.push(draw_card);
                    if self.is_player_hu() {
                        con.send(ServerMessage::Hu { to: None }).ok();
                        self.end();
                        return None;
                    }
                    card = self.players[self.turn as usize].discard_card();
                } else {
                    let msg = self.restore_turn();
                    con.send(msg).ok();
                    return None;
                }
            }
            Mode::Ding(discard) => {
                if self.players[self.turn as usize].ding_card(discard) {
                    for (pos, _) in [(Pos::Right, right), (Pos::Left, left)] {
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
                            con.send(ServerMessage::Ding {
                                to: None,
                                card: discard,
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
                if self.is_player_hu() {
                    con.send(ServerMessage::Hu { to: None }).ok();
                    self.end();
                    return None;
                }
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

    pub fn is_player_hu(&self) -> bool {
        // TODO: we have to calculate the score to judge wether a player is hu
        let mut score = 0;
        for p in &self.players[self.turn as usize].pairing {
            match p {
                Pairing::Triplet(_) => score += 4,
                Pairing::Quadlet(_) => score += 8,
            }
        }
        Self::is_hu(&self.players[self.turn as usize].hand, score, self.jing)
    }

    fn is_hu(hand: &Vec<Card>, mut score: u8, jing: Card) -> bool {
        let mut hand_cnt = HashMap::new();
        for c in hand {
            hand_cnt.entry(c.0 / 4).and_modify(|e| *e += 1).or_insert(1);
        }
        debug!("hand_cnt: {hand_cnt:?}");
        fn minus_entry(cnt: &mut HashMap<u8, i32>, key: u8, val: i32) {
            cnt.entry(key).and_modify(|e| *e -= val);
            if cnt[&key] == 0 {
                cnt.remove(&key);
            }
        }
        for i in 0..8 {
            let i = 3 * i;
            let j = i + 1;
            let k = i + 2;
            while hand_cnt.contains_key(&i)
                && hand_cnt.contains_key(&j)
                && hand_cnt.contains_key(&k)
            {
                if i == 0 {
                    score += 4;
                }
                for x in [i, j, k] {
                    if jing.is_same_kind(&Card(x * 4)) {
                        score += 4;
                    }
                }
                minus_entry(&mut hand_cnt, i, 1);
                minus_entry(&mut hand_cnt, j, 1);
                minus_entry(&mut hand_cnt, k, 1);
            }
        }
        debug!("hand_cnt: {hand_cnt:?}, score: {score}");

        for i in 0..24 {
            if hand_cnt.contains_key(&i) {
                if hand_cnt[&i] >= 3 {
                    minus_entry(&mut hand_cnt, i, 3);
                    score += 4;
                    if i == 0 {
                        score += 8;
                    }
                    if jing.is_same_kind(&Card(i * 4)) {
                        score += 8;
                    }
                }
            }
        }
        debug!("hand_cnt: {hand_cnt:?}, score: {score}");

        if hand_cnt.len() != 2 {
            return false;
        }
        let keys: Vec<&u8> = hand_cnt.keys().collect();
        if score >= 12 {
            keys[0] / 3 == keys[1] / 3
        } else {
            false
        }
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
                    debug!("[send message] {update:?}");
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

    fn wait_robot(&self) {
        loop {
            let is_robot_turn = self.state.read().is_robot_turn();
            debug!(
                "is robot turn {is_robot_turn}, turn {}",
                self.state.read().turn
            );
            if !is_robot_turn {
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
    }

    async fn handle_message(&self, id: u8, message: Message) -> Result<()> {
        let message: ClientMessage = match message.to_str() {
            Err(()) => return Ok(()),
            ::std::result::Result::Ok(text) => {
                serde_json::from_str(text).context("failed to deserialized client message")?
            }
        };
        debug!("[handle message] message {message:?}");
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
                self.wait_robot();

                if Mode::Normal == self.state.read().mode {
                    let msg = self.state.write().draw_card();
                    debug!("write draw card message success");
                    self.connection.send(msg).ok();
                    let is_hu = self.state.read().is_player_hu();
                    if is_hu {
                        self.connection.send(ServerMessage::Hu { to: None }).ok();
                        self.state.write().end();
                    }
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
                    let is_hu = self.state.read().is_player_hu();
                    if is_hu {
                        self.connection.send(ServerMessage::Hu { to: None }).ok();
                        self.state.write().end();
                    }
                }
            }
            ClientMessage::Ding { confirm } => {
                let mode = self.state.read().mode.clone();
                self.state.write().mode = Mode::Normal;
                if !confirm {
                    let msg = self.state.write().restore_turn();
                    self.connection.send(msg).ok();
                    self.wait_robot();
                } else {
                    let card = match mode {
                        Mode::Ding(c) => c,
                        _ => bail!("wrong mode, expect Ding mode, got {:?}", mode),
                    };
                    let msg = ServerMessage::Ding { to: None, card };
                    self.connection.send(msg).ok();
                }
            }
            ClientMessage::Pao { confirm } => {
                let mode = self.state.read().mode.clone();
                self.state.write().mode = Mode::Normal;
                if !confirm {
                    let msg = self.state.write().restore_turn();
                    self.connection.send(msg).ok();
                    self.wait_robot();
                } else {
                    let card = match mode {
                        Mode::Pao(c) => c,
                        _ => bail!("wrong mode, expect Wa mode, got {:?}", mode),
                    };
                    let msg = ServerMessage::Pao { to: None, card };
                    self.connection.send(msg).ok();
                }
                let msg = self.state.write().draw_card();
                self.connection.send(msg).ok();
                let is_hu = self.state.read().is_player_hu();
                if is_hu {
                    self.connection.send(ServerMessage::Hu { to: None }).ok();
                    self.state.write().end();
                }
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

        pub async fn expect_draw(&mut self, expect_card: Card) {
            let msg = self.recv().await;
            match msg {
                ServerMessage::Draw { to, card } => {
                    assert_eq!(to.unwrap(), 0);
                    assert_eq!(card, expect_card);
                }
                _ => panic!("expect draw message, got {msg:?}"),
            }
        }
        pub async fn expect_turn(&mut self, expect_turn: u8, expect_mode: Mode) {
            let msg = self.recv().await;
            match msg {
                ServerMessage::Turn { to, turn, mode } => {
                    assert_eq!(to.is_none(), true);
                    assert_eq!(turn, expect_turn);
                    assert_eq!(mode, expect_mode);
                }
                _ => panic!("expect turn message, got {msg:?}"),
            }
        }
        pub async fn expect_discard(&mut self, expect_card: Card) {
            let msg = self.recv().await;
            match msg {
                ServerMessage::Discard { to, card } => {
                    assert_eq!(to.unwrap(), 0);
                    assert_eq!(card, expect_card);
                }
                _ => panic!("expect discard message, got {msg:?}"),
            }
        }
        pub async fn expect_ding(&mut self, expect_card: Card) {
            let msg = self.recv().await;
            match msg {
                ServerMessage::Ding { to, card } => {
                    assert!(to.is_none());
                    assert_eq!(card, expect_card);
                }
                _ => panic!("expect ding message, got {msg:?}"),
            }
        }
        pub async fn expect_pao(&mut self, expect_card: Card) {
            let msg = self.recv().await;
            match msg {
                ServerMessage::Pao { to, card } => {
                    assert!(to.is_none());
                    assert_eq!(card, expect_card);
                }
                _ => panic!("expect pao message, got {msg:?}"),
            }
        }
        pub async fn expect_initial(&mut self, expect_turn: u8, expect_hand: &Vec<Card>) {
            let msg = self.recv().await;
            match msg {
                ServerMessage::Initial { to, cur_turn, hand } => {
                    assert_eq!(to.unwrap(), 0);
                    assert_eq!(cur_turn, expect_turn);
                    assert_eq!(&hand, expect_hand);
                }
                _ => panic!("expect initial message, got {msg:?}"),
            }
        }
    }

    async fn connect() -> JsonWsClient {
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

    #[tokio::test]
    async fn basic_test1() {
        let mut builder = env_logger::Builder::from_default_env();
        builder.target(env_logger::Target::Stdout);
        builder.init();

        let mut client = connect().await;
        client.send(ClientMessage::Test(true)).await;
        client.send(ClientMessage::Ready(true)).await;
        for _ in 0..2 {
            client.send(ClientMessage::AddRobot(true)).await;
        }
        client.send(ClientMessage::Start(true)).await;
        let initial_hand: Vec<Card> = (0..19).into_iter().map(|n| Card(n)).collect();
        client.expect_initial(0, &initial_hand).await;

        client.expect_draw(Card(57)).await;

        client.send(ClientMessage::Discard { card: Card(57) }).await;
        client.expect_turn(1, Mode::Normal).await;
        client.expect_discard(Card(19)).await;
        client.expect_turn(0, Mode::Pao(Card(19))).await;

        client.send(ClientMessage::Pao { confirm: false }).await;
        client.expect_turn(2, Mode::Normal).await;
        client.expect_discard(Card(38)).await;
        client.expect_turn(1, Mode::Ding(Card(38))).await;
        client.expect_ding(Card(38)).await;
        client.expect_discard(Card(20)).await;
        client.expect_turn(2, Mode::Normal).await;
        client.expect_discard(Card(39)).await;
        client.expect_turn(0, Mode::Normal).await;
        client.expect_draw(Card(61)).await;
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
        let initial_hand: Vec<Card> = (0..19).into_iter().map(|n| Card(n)).collect();
        client.expect_initial(0, &initial_hand).await;

        client.expect_draw(Card(57)).await;

        client.send(ClientMessage::Discard { card: Card(57) }).await;
        client.expect_turn(1, Mode::Normal).await;
        client.expect_discard(Card(19)).await;
        client.expect_turn(0, Mode::Pao(Card(19))).await;

        client.send(ClientMessage::Pao { confirm: true }).await;
        client.expect_pao(Card(19)).await;
        client.expect_draw(Card(59)).await;
    }

    #[test]
    fn test_hu() {
        let mut builder = env_logger::Builder::from_default_env();
        builder.target(env_logger::Target::Stdout);
        builder.init();
        // 0 1 2 / 0 1 2 / 3 3 3 / 3 4 5 / 6 7 8 / 6 7 8 / 8 6
        let hand = [
            0, 4, 8, 1, 5, 9, 12, 13, 14, 15, 16, 20, 24, 28, 32, 25, 29, 33, 34, 26,
        ];
        let hand = hand.into_iter().map(|n| Card(n)).collect();
        assert_eq!(GameState::is_hu(&hand, 0, Card(90)), true);
        // 0 1 2 / 0 1 2 / 3 3 3 / 3 4 5 / 6 7 8 / 6 7 8 / 8 9
        let hand = [
            0, 4, 8, 1, 5, 9, 12, 13, 14, 15, 16, 20, 24, 28, 32, 25, 29, 33, 34, 36,
        ];
        let hand = hand.into_iter().map(|n| Card(n)).collect();
        assert_eq!(GameState::is_hu(&hand, 0, Card(90)), false);
        // 1 2 / 0 1 2 / 3 3 3 / 3 4 5 / 6 7 8 / 6 7 8 / 7 8 9
        let hand = [
            4, 8, 1, 5, 9, 12, 13, 14, 15, 16, 20, 24, 28, 32, 25, 29, 33, 30, 34, 36,
        ];
        let hand = hand.into_iter().map(|n| Card(n)).collect();
        assert_eq!(GameState::is_hu(&hand, 0, Card(90)), false);
    }
}
