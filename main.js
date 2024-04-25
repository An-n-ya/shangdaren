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

let cards_arr = [];

let hand = [];
let total = [];

for (let j = 0; j < card_face.length; j++) {
    for (let i = 0; i < 4; i++) {
        let id = j * 4 + i;
        cards_arr.push(new Card(card_face[j], false, id));
    }
}
let btn = document.querySelector('#hello-btn');
btn.addEventListener('click', () => {
    request.get('', {}).then(function (res) {
        console.log(res.data);
    });
});

start();
render();

function getRandomInt(max) {
    return Math.floor(Math.random() * max);
}

function start() {
    let total = cards_arr.map(a => ({...a}));;
    for (let i = 0; i < 19; i++) {
        let ind = getRandomInt(total.length);
        let card = total[ind];
        total.splice(ind, 1);
        hand.push(card);
    }
}

function sort_hand() {
    hand.sort((a, b) =>
        a.id - b.id
    )
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