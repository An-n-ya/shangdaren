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

function Card(type, selected, id) {
    this.type = type;
    this.selected = selected;
    this.id = id;
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

let cur_turn = 0;
let my_turn = 0;
let session_id = "";

let cards_arr = [];

let total = [];


let players = [];

for (let j = 0; j < card_face.length; j++) {
    for (let i = 0; i < 4; i++) {
        let id = j * 4 + i;
        cards_arr.push(new Card(card_face[j], false, id));
    }
}
let btn = document.querySelector('#btn');

start();
render();

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


async function start() {
    total = cards_arr.map(a => ({...a}));;
    for (let i = 0; i < 3; i++) {
        let player = new Player(i, [], []);
        for (let j = 0; j < 19; j++) {
            player.hand.push(draw_card());
        }
        players.push(player);
    }
    my_turn = getRandomInt(3);
    console.log("my_turn: ", my_turn);
    let right = (my_turn + 1) % 3;
    let left = (right + 1) % 3;
    players[right].name = "right";
    players[left].name = "left";


    let res = await request.get("/new_game", {});
    session_id = res.data;
    let initial_url = "/initial/" + session_id;
    for (let i = 0; i < 3; i++) {
        if (i != my_turn) {
            request.post(initial_url, {
                hand: hand_to_data(players[i].hand),
                turn: i
            });
        }
    }

    while (my_turn != cur_turn) {
        await wait_player(players[cur_turn]);
    }
    my_turn_begin();
}

function hand_to_data(hand) {
    let data = [];
    for (let i = 0; i < hand.length; i++) {
        data.push(hand[i].id);
    }
    return data;
}

function draw_card() {
    let ind = getRandomInt(total.length);
    let card = total[ind];
    total.splice(ind, 1);
    return card
}

function my_turn_begin() {
    let card = draw_card();
    let hand = players[my_turn].hand;
    hand.push(card);
    render();
    play_card_btn_enable();
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
    cur_turn = (cur_turn + 1) % 3;
    await broadcast_discard(card);

    await wait_player(players[cur_turn]);
    await wait_player(players[cur_turn]);

    my_turn_begin();
}

async function broadcast_discard(card) {
    for (let i = 0; i < 3; i++) {
        if (i != my_turn && i != cur_turn) {
            request.post("/discard/" + session_id, {
                card: card.id,
                turn: i,
                cur_turn: cur_turn
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
            turn: cur_turn
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
        cur_turn = (cur_turn + 1) % 3;
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
    players[my_turn].hand.sort((a, b) =>
        a.id - b.id
    )
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