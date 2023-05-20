let current_answer = undefined;

function setAnswer() {
    let answer = document.getElementById("answerInput").value;
    console.log(answer)
    current_answer = answer;
    document.getElementById("buttonSend").disabled = false;
}

function sendAnswer() {
    if (current_answer === undefined) {
        return;
    }
    var name = localStorage.getItem("group_name")
    var data = JSON.stringify({ name: name, type: "schaetzen", answer: current_answer.toString() })
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