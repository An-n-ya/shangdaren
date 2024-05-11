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
                this.sendTest();
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
                console.log("[handleMessage] mode:", mode);
                if (mode == "Normal") {
                    play_card_btn_enable("出牌");
                } else {
                    if (mode.Pao != undefined) {
                        cur_pao_or_ding = mode.Pao;
                        console.log("cur_pao_or_ding", cur_pao_or_ding);
                        play_card_btn_enable("抛");
                    } else if (mode.Ding != undefined) {
                        cur_pao_or_ding = mode.Ding;
                        console.log("cur_pao_or_ding", cur_pao_or_ding);
                        play_card_btn_enable("钉");
                    }
                }
            }

        } else if (msg.Initial !== undefined) {
            const {to, cur_turn, hand} = msg.Initial;
            my_turn = to;
            current_turn = cur_turn;
            for (let i = 0; i < 3; i++) {
                let player = new Player(i, [], []);
                players.push(player);
            }
            for (let c_id of hand) {
                players[my_turn].hand.push(new Card(c_id));
            }
            console.log("my_turn: ", my_turn);
            let right = (my_turn + 1) % 3;
            let left = (right + 1) % 3;
            players[right].name = "right";
            players[left].name = "left";

            render();

            if (current_turn == my_turn) {
                play_card_btn_enable("出牌");
            }

        } else if (msg.Draw !== undefined) {
            const {to, card: id} = msg.Draw;
            players[my_turn].hand.push(new Card(id));
            render();
            if (current_turn == my_turn) {
                play_card_btn_enable("出牌");
            }

        } else if (msg.Discard !== undefined) {
            const {to, card: id} = msg.Discard;
            let card = new Card(id);
            players[current_turn].out.push(card);
            let container = document.querySelector("#" + players[current_turn].name + "-cards");
            append_out(container, card);
            render();

        } else if (msg.Pao !== undefined){
            const {to, card: id} = msg.Pao;
            if (to == my_turn) {return;}
            let card = new Card(id);
        } else if (msg.Ding !== undefined){
            const {to, card: id} = msg.Ding;
            if (to == my_turn) {return;}
            let card = new Card(id);
        } else if (msg.Hu !== undefined) {
            let result = document.querySelector("#result");
            result.className = "";
            if (current_turn == my_turn) {
                let win = document.querySelector("#win");
                win.className = "";
                lose.className = "hide";
            } else {
                let lose = document.querySelector("#lose");
                lose.className = "";
                win.className = "hide";
            }
            window.setTimeout(() => {
                win.className = "hide";
                lose.className = "hide";
                result.className = "hide";
                render_room();
            }, 3000);
        } else {
            console.log("unrecognized message");
        }
    }

    sendTest() {
        this.ws.send(`{"Test": true}`);
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
    sendDiscard(card_id) {
        this.ws.send(`{"Discard": {"card": ${card_id}}}`)
    }
    sendPao(confirm) {
        this.ws.send(`{"Pao": {"confirm": ${confirm}}}`)
    }
    sendDing(confirm) {
        this.ws.send(`{"Ding": {"confirm": ${confirm}}}`)
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
let cur_pao_or_ding;

let players = [];

let game;

let play_btn = document.querySelector('#play-btn');
let cancel_btn = document.querySelector('#cancel-btn');
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
    hide_btn();
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


function play_card() {
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

    game.sendDiscard(card.id);
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
function discard_type(hand, id) {
    console.log("discard_type, hand:", hand, "id:", id);
    let index = [];
    for (let i = 0 ; i < hand.length; i++) {
        if (Math.floor(hand[i].id / 4) == Math.floor(id / 4)) {
            index.push(i);
            let card = hand[i];
        }
    }
    hand.splice(index[0], index.length);
    console.log("discard_type, hand:", hand, "index:", index);
}

function play_card_btn_enable(action) {
    play_btn.removeAttribute("hidden");
    if (action == "出牌") {
        play_btn.textContent = action;
        play_btn.addEventListener('click', play_btn.play=play_card, false);
    } else if (action == "抛") {
        cancel_btn.removeAttribute("hidden");
        play_btn.textContent = action;
        play_btn.addEventListener('click', play_btn.play=function play() {
            game.sendPao(true);
            discard_type(players[current_turn].hand, cur_pao_or_ding);
            render();
            // TODO: remove other same card
            hide_btn();
        }, false);
        cancel_btn.addEventListener('click', cancel_btn.cancel=function cancel() {
            game.sendPao(false);
            hide_btn();
        }, false)
    } else if (action == "钉") {
        cancel_btn.removeAttribute("hidden");
        play_btn.textContent = action;
        play_btn.addEventListener('click', play_btn.play=function play() {
            game.sendDing(true);
            discard_type(players[current_turn].hand, cur_pao_or_ding);
            render();
            // TODO: remove other same card
            hide_btn();
        }, false);
        cancel_btn.addEventListener('click', cancel_btn.cancel=function cancel() {
            game.sendDing(false);
            hide_btn();
        }, false)
    }
}

function hide_btn() {
    play_btn.setAttribute("hidden", "true");
    cancel_btn.setAttribute("hidden", "true");
    play_btn.removeEventListener("click", play_btn.play, false);
    cancel_btn.removeEventListener("click", cancel_btn.cancel, false);
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