let current_answer = [];
let letters = ['A', 'B', 'C', 'D']

function addAnswer(answer) {
    current_answer.push(answer);
    document.getElementById("answer_content").innerHTML += letters[answer]
    document.getElementById("answer").style.display = "block";
    document.getElementById("button" + letters[answer]).disabled = true;
    if (current_answer.length === 4) {
        document.getElementById("buttonSend").disabled = false;
    }
}

function clearAnswer() {
    current_answer = []
    document.getElementById("answer_content").innerHTML = ""
    document.getElementById("answer").style.display = "none";
    document.getElementById("buttonA").disabled = false;
    document.getElementById("buttonB").disabled = false;
    document.getElementById("buttonC").disabled = false;
    document.getElementById("buttonD").disabled = false;
    document.getElementById("buttonSend").disabled = true;
}

function sendAnswer() {
    if (current_answer.length !== 4) {
        return;
    }
    let answer_string = ""
    for (c of current_answer) {
        answer_string += letters[c]
    }
    var name = localStorage.getItem("group_name")
    var data = JSON.stringify({ name: name, type: "sortier", answer: answer_string })
    console.log(data)
    let xhr = new XMLHttpRequest();
    xhr.open("POST", "/groups/set_answer", true);
    xhr.setRequestHeader('Content-Type', 'application/json');
    xhr.send(data);

    xhr.onload = e => {
        console.log(xhr.status)
        console.log(xhr.statusText)
    }
}