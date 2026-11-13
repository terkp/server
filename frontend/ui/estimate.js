// This script runs before the input and ui/index.js are loaded, so it must
// not touch the page at the top level. See the comment at the top of index.js.

let current_answer = undefined;

function hasCompleteAnswer() {
  return current_answer !== undefined && current_answer !== "";
}

function setAnswer() {
  current_answer = document.getElementById("answerInput").value;
  updateSendButton();
}

function sendAnswer() {
  if (!hasCompleteAnswer()) {
    return;
  }
  const answer = current_answer.toString();
  if (!Number.isFinite(Number(answer))) {
    alert("Bitte Zahl eingeben!");
    current_answer = undefined;
    document.getElementById("answerInput").value = "";
    updateSendButton();
    return;
  }
  submitAnswer("schaetzen", answer, answer);
}
