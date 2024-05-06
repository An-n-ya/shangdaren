import './style.css'
import axios from 'axios';

// TODO:
/*
    - is_hu() for client and server
    - wa and ding action
    - discard card action
    - display wa and ding
*/
const request = axios.create({
  baseURL: '/api',
  timeout: 1000,
  headers: {}
});

let room_id = "";
const printable_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

function get_hash() {
    if (!window.location.hash) {
        room_id = "";
        for (let i = 0; i < 5; i++) {
            room_id += printable_chars[getRandomInt(printable_chars.length)];
        }
        window.history.replaceState(null, "", "#" + room_id);
    } else {
        room_id = window.location.hash.slice(1);
    }
    console.log("room_id: ", room_id);
}

window.addEventListener("hashchange", get_hash);

class Game {
    ws;

    constructor() {
        this.tryConnect();
        window.setInterval(() => this.tryConnect(), 2);
    }

    tryConnect() {
        if (this.ws == undefined) {
            let uri = "ws://" + window.location.host + "/api/ws/" + room_id;
            const ws = new WebSocket(uri)
            ws.onopen = () => {
                this.ws = ws;
                this.sendReady();
            }
            ws.onmessage = ({data}) => {
                if (typeof data === "string") {
                    this.handleMessage(JSON.parse(data));
                }
            }
            ws.onclose = () => {
                if (this.ws) {
                    this.ws = undefined;
                }
            }
        }

    }

    handleMessage(msg) {
        if (msg.Turn !== undefined) {
            const {to, turn, mode} = msg.Turn;
            current_turn = turn;
            if (turn == my_turn) {
                play_card_btn_enable();
            }

        } else if (msg.Initial !== undefined) {
            const {to, cur_turn, hand} = msg.Initial;
            my_turn = to;
            current_turn = cur_turn;
            for (let i = 0; i < 3; i++) {
                let player = new Player(i, [], []);
                players.push(player);
            }
            console.log(hand);
            for (let c_id of hand) {
                console.log(c_id);
                players[my_turn].hand.push(new Card(c_id));
            }
            console.log("my_turn: ", my_turn);
            let right = (my_turn + 1) % 3;
            let left = (right + 1) % 3;
            players[right].name = "right";
            players[left].name = "left";

            render();

            if (current_turn == my_turn) {
                play_card_btn_enable();
            }

        } else if (msg.Draw !== undefined) {
            const {to, card: id} = msg.Draw;
            players[my_turn].hand.push(new Card(id));
            render();

        } else if (msg.Discard !== undefined) {
            const {to, card: id} = msg.Discard;
            players[current_turn].out.push(new Card(id));
            render();

        } else {
            console.log("unrecognized message");
        }
    }

    sendReady() {
        this.ws.send(`{"Ready": true}`);
    }
    sendAddRobot() {
        this.ws.send(`{"AddRobot": true}`);
    }
    sendStart() {
        this.ws.send(`{"Start": true}`)
    }
}

class Card {
    type;
    selected;
    id;

    constructor(id) {
        this.type = card_face[Math.floor(id / 4)];
        this.selected = false;
        this.id = id;
    }

}

function Player(turn, hand, out) {
    this.name = "";
    this.turn = turn;
    this.hand = hand;
    this.out = out;
}

let card_face = [
    "上", "大", "人", "孔", "乙", "己",
    "化", "三", "千", "七", "十", "士",
    "尔", "小", "生", "八", "九", "子",
    "佳", "作", "仁", "福", "禄", "寿",
];

let current_turn = 0;
let my_turn = 0;
let session_id = "";

let cards_arr = [];

let players = [];

let game;

let btn = document.querySelector('#btn');
let start_btn = document.querySelector('#start');
let playground = document.querySelector('#playground');
let room = document.querySelector('#room');

start();

function getRandomInt(max) {
    while (true) {
        let rand = Math.random();
        if (rand != 1) {
            return Math.floor(rand * max);
        }
    }
}

async function end() {
    await request.get("/end_game/" + session_id, {});
    console.log("game end!");
    while (true) {}
}


function render_playground()
{
    room.className = "hide";
    playground.className = "";
}
function render_room()
{
    room.className = "";
    playground.className = "hide";
    if (!room.firstChild.hasChildNodes()) {
        for (let i = 0; i < 3; i++) {
            room.firstElementChild.appendChild(create_player_slot(i == 0));
        }
        start_btn.addEventListener('click', () => {
            game.sendStart();
            render_playground();
        });
    }
}

async function start() {
    get_hash();
    render_room();

    game = new Game();

    // while (my_turn != current_turn) {
    //     await wait_player(players[current_turn]);
    // }
}


async function play_card() {
    let hand = players[my_turn].hand;
    let card = undefined;
    for (let i = 0; i < hand.length; i++) {
        card = hand[i];
        if (card.selected) {
            hand.splice(i, 1);
            card.selected = false;
            let container = document.querySelector('#my-out-cards');
            append_out(container, card);
            render();
            break;
        }
    }
    hide_btn();
    current_turn = (current_turn + 1) % 3;
    await broadcast_discard(card);

    await wait_player(players[current_turn]);
    await wait_player(players[current_turn]);

    my_turn_begin();
}

async function broadcast_discard(card) {
    for (let i = 0; i < 3; i++) {
        if (i != my_turn && i != current_turn) {
            request.post("/discard/" + session_id, {
                card: card.id,
                turn: i,
                cur_turn: current_turn
            });
        }
    }
}

function play_card_btn_enable() {
    btn.removeAttribute("hidden");
    btn.addEventListener('click', play_card);
}

async function wait_player(player) {
        let new_card = draw_card();
        let res = await request.post("/turn/" + session_id, {
            card: new_card.id,
            turn: current_turn
        });
        let is_win = res.data.win;
        if (is_win) {
            end();
            return
        }
        let discard_id = res.data.discard;
        let card = discard_card(player.hand,discard_id);
        let container = document.querySelector("#" + player.name + "-cards");
        append_out(container, card, player.hand);
        broadcast_discard(card);
        current_turn = (current_turn + 1) % 3;
}

function discard_card(hand, id) {
    for (let i = 0 ; i < hand.length; i++) {
        if (hand[i].id == id) {
            let card = hand[i];
            hand.splice(i, 1);
            return card;
        }
    }
}

function hide_btn() {
    btn.setAttribute("hidden", "true");
    btn.removeEventListener("click", play_card);
}

function sort_hand() {
    if (players.length > 0) {
    players[my_turn].hand.sort((a, b) =>
        a.id - b.id
    )
    }
}

function append_out(container, card) {
    container.appendChild(create_card(card, false))
}


function render() {
    let container = document.querySelector('#cards');
    container.innerHTML = "";
    sort_hand();
    let cur_group = 1;
    let group = document.createElement("div");
    group.id = "group";
    for (let i = 0; i < players[my_turn].hand.length; i++) {
        let card = players[my_turn].hand[i];
        if (card.id >= (cur_group - 1) * 12 && card.id < cur_group * 12) {
            group.appendChild(create_card(card, true));
        } else {
            while (cur_group * 12 <= card.id) {
                cur_group += 1;
            }
            if (group.childNodes.length != 0) {
                group.style.maxHeight = "" + ((group.childNodes.length)*50) + "px";
                container.appendChild(group);
            }
            group = document.createElement("div");
            group.id = "group";
            group.appendChild(create_card(card, true));
        }
    }
    if (group.childNodes.length != 0) {
        group.style.maxHeight = "" + ((group.childNodes.length)*50) + "px";
        container.appendChild(group);
    }
}

function create_player_slot(is_me) {
    let div = document.createElement("div");
    div.id = "player";
    let img = document.createElement("img");
    img.src = is_me ? "上大人/user.svg" : "上大人/add.svg";
    img.id = "icon";
    if (!is_me) {
        img.addEventListener('click', () => {
            console.log("hello", img.src)
            if (img.src.endsWith("add.svg")) {
                img.src = "上大人/robot.svg";
                game.sendAddRobot();
            }
        });
    }

    div.appendChild(img);
    return div;
}

function create_card(card, clickable) {
    let div = document.createElement("div");
    div.id = "card-wrapper";
    let img = document.createElement("img");
    img.src = "上大人/" + card.type + ".png";
    if (clickable) {
        img.addEventListener('click', () => {
            let hand = players[my_turn].hand;
            for (let i = 0; i < hand.length; i++) {
                if (hand[i].id == card.id) {
                    hand[i].selected = true;
                } else {
                    hand[i].selected = false;
                }
            }
            render();
        })
        if (card.selected) {
            img.classList.add("selected");
            div.classList.add("selected");
        }
    }
    img.id = "card";
    div.appendChild(img);

    return div;
}