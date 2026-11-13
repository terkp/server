// This script runs before the buttons and ui/index.js are loaded, so it must
// not touch the page at the top level. See the comment at the top of index.js.

let current_answer = [];
let letters = ["A", "B", "C", "D"];

function hasCompleteAnswer() {
  return current_answer.length === 4;
}

function addAnswer(answer) {
  current_answer.push(answer);
  const currentAnswerElement = document.getElementById("currentAnswer");
  currentAnswerElement.textContent += letters[answer];
  currentAnswerElement.style.display = "block";
  document.getElementById("button" + letters[answer]).disabled = true;
  updateSendButton();
}

function clearAnswer() {
  current_answer = [];
  const currentAnswerElement = document.getElementById("currentAnswer");
  currentAnswerElement.textContent = "";
  currentAnswerElement.style.display = "none";
  document.getElementById("buttonA").disabled = false;
  document.getElementById("buttonB").disabled = false;
  document.getElementById("buttonC").disabled = false;
  document.getElementById("buttonD").disabled = false;
  updateSendButton();
}

function sendAnswer() {
  if (current_answer.length !== 4) {
    return;
  }
  const answerString = current_answer.map((idx) => letters[idx]).join("");
  submitAnswer("sortier", answerString, answerString, clearAnswer);
}
