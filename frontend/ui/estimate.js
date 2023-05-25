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
    if (isNaN(parseInt(current_answer))) {
        alert("Bitte Zahl eingeben!")
        current_answer = undefined;
        document.getElementById("buttonSend").disabled = true;
        document.getElementById("answerInput").value = "";
        return
    }
    var name = localStorage.getItem("group_name")
    var data = JSON.stringify({ name: name, type: "schaetzen", answer: current_answer.toString() })
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
    }
    xhr.send(data);
}