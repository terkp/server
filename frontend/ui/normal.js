let current_answer = undefined;
let letters = ['A', 'B', 'C', 'D']

function setAnswer(answer) {
    current_answer = answer;
    document.getElementById("answer_content").innerHTML = letters[current_answer]
    document.getElementById("answer").style.display = "block";
}

function sendAnswer() {
    localStorage.setItem("group_name", "meine gruppe")
}