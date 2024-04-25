import './style.css'
import axios from 'axios';

// 规则
/*

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

let card_face = [
    "上", "大", "人", "孔", "乙", "己",
    "化", "三", "千", "七", "十", "士",
    "尔", "小", "生", "八", "九", "子",
    "佳", "作", "仁", "福", "禄", "寿",
];

let cur_turn = 0;

let cards_arr = [];

let hand = [];
let left_hand = [];
let right_hand = [];
let total = [];

let left_out = [];
let right_out = [];
let my_out = [];

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
    return Math.floor(Math.random() * max);
}


function start() {
    total = cards_arr.map(a => ({...a}));;
    play_card_btn_enable();
    for (let i = 0; i < 19; i++) {
        left_hand.push(draw_card());
    }
    for (let i = 0; i < 19; i++) {
        right_hand.push(draw_card());
    }
    for (let i = 0; i < 19; i++) {
        hand.push(draw_card());
    }
}

function draw_card() {
    let ind = getRandomInt(total.length);
    let card = total[ind];
    total.splice(ind, 1);
    return card
}

function my_turn() {
    let card = draw_card();
    hand.push(card);
    render();
    play_card_btn_enable();
}

function play_card() {
    for (let i = 0; i < hand.length; i++) {
        let card = hand[i];
        if (card.selected) {
            hand.splice(i, 1);
            card.selected = false;
            let container = document.querySelector('#my-out-cards');
            append_out(container, card, my_out);
            render();
            break;
        }
    }
    // request.get('', {}).then(function (res) {
    //     console.log(res.data);
    // });
    hide_btn();
    cur_turn = (cur_turn + 1) % 3;
    wait_others();
}

function play_card_btn_enable() {
    btn.removeAttribute("hidden");
    btn.addEventListener('click', play_card);
}

function wait_others() {
    setTimeout(() => {
        let new_card = draw_card();
        right_hand.push(new_card);
        let card = discard_card(right_hand);
        let container = document.querySelector('#right-cards');
        append_out(container, card, right_hand);
        cur_turn = (cur_turn + 1) % 3;
    }, 400);
    setTimeout(() => {
        let new_card = draw_card();
        left_hand.push(new_card);
        let card = discard_card(left_hand);
        let container = document.querySelector('#left-cards');
        append_out(container, card, left_hand);
        cur_turn = (cur_turn + 1) % 3;
        my_turn();
    }, 800);
}

function discard_card(hand) {
    let ind = getRandomInt(hand.length);
    let card = hand[ind];
    hand.splice(ind, 1);
    return card;
}

function hide_btn() {
    btn.setAttribute("hidden", "true");
    btn.removeEventListener("click", play_card);
}

function sort_hand() {
    hand.sort((a, b) =>
        a.id - b.id
    )
}

function append_out(container, card, out) {
    container.appendChild(create_card(card))
}


function render() {
    let container = document.querySelector('#cards');
    container.innerHTML = "";
    sort_hand();
    let cur_group = 1;
    let group = document.createElement("div");
            group.id = "group";
    for (let i = 0; i < hand.length; i++) {
        let card = hand[i];
        if (card.id >= (cur_group - 1) * 12 && card.id < cur_group * 12) {
            group.appendChild(create_card(card));
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
            group.appendChild(create_card(card));
        }
    }
    if (group.childNodes.length != 0) {
        group.style.maxHeight = "" + ((group.childNodes.length)*50) + "px";
        container.appendChild(group);
    }
}

function create_card(card) {
    let div = document.createElement("div");
    div.id = "card-wrapper";
    let img = document.createElement("img");
    img.src = "上大人/" + card.type + ".png";
    img.addEventListener('click', () => {
        for (let i = 0; i < hand.length; i++) {
            if (hand[i].id == card.id) {
                hand[i].selected = true;
            } else {
                hand[i].selected = false;
            }
        }
        render();
    })
    img.id = "card";
    if (card.selected) {
        img.classList.add("selected");
        div.classList.add("selected");
    }
    div.appendChild(img);

    return div;
}