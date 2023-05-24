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
    xhr.onload = e => {
        // If the group is not found, remove it from local storage and ask the user to login again
        if (xhr.status >= 400 && xhr.status < 500 && xhr.responseText.includes("not found")) {
            localStorage.removeItem("group_name")
            alert("Bitte logge dich erneut ein");
            window.location.href = "/login"
        }
        console.log(xhr.status.toString() + " " + xhr.responseText)
    }
    xhr.send(data);
}