import './style.css'

// document.querySelector('#app').innerHTML = `
//   <div>
//     <a href="https://vitejs.dev" target="_blank">
//       <img src="${viteLogo}" class="logo" alt="Vite logo" />
//     </a>
//     <a href="https://developer.mozilla.org/en-US/docs/Web/JavaScript" target="_blank">
//       <img src="${javascriptLogo}" class="logo vanilla" alt="JavaScript logo" />
//     </a>
//     <h1>Hello Vite!</h1>
//     <div class="card">
//       <button id="counter" type="button"></button>
//     </div>
//     <p class="read-the-docs">
//       Click on the Vite logo to learn more
//     </p>
//   </div>
// `

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

for (let i = 0; i < 4; i++) {
    for (let j = 0; j < card_face.length; j++) {
        let id = i * card_face.length + j;
        cards_arr.push(new Card(card_face[j], false, id));
    }
}

start();
render();

function getRandomInt(max) {
    return Math.floor(Math.random() * max);
}

function start() {
    let total = cards_arr.map(a => ({...a}));;
    for (let i = 0; i < 12; i++) {
        let ind = getRandomInt(total.length);
        let card = total[ind];
        total.splice(ind, 1);
        hand.push(card);
    }
}


function render() {
    let container = document.querySelector('#cards');
    container.innerHTML = "";
    for (let i = 0; i < hand.length; i++) {
        let card = hand[i];
        container.appendChild(create_card(card));
    }
}

function create_card(card) {
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
    }

    return img;
}