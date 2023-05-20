let current_answer = undefined;
let letters = ['A', 'B', 'C', 'D']

function setAnswer(answer) {
    current_answer = answer;
    document.getElementById("answer_content").innerHTML = letters[current_answer]
    document.getElementById("answer").style.display = "block";
    document.getElementById("buttonSend").disabled = false;
}

function sendAnswer() {
    if (current_answer === undefined) {
        return;
    }
    var name = localStorage.getItem("group_name")
    var data = JSON.stringify({ name: name, type: "normal", answer:current_answer.toString() })
    let xhr = new XMLHttpRequest();
    xhr.open("POST", "/groups/set_answer", true);
    xhr.setRequestHeader('Content-Type', 'application/json');
    xhr.send(data);
}