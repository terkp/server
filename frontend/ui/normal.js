// This script runs before the buttons and ui/index.js are loaded, so it must
// not touch the page at the top level. See the comment at the top of index.js.

let current_answer = undefined;
let letters = ["A", "B", "C", "D"];

function hasCompleteAnswer() {
  return current_answer !== undefined;
}

function setAnswer(answer) {
  current_answer = answer;
  document.querySelectorAll("#answerContainer button").forEach((b) => {
    b.disabled = false;
  });
  document.getElementById("button" + letters[answer]).disabled = true;
  updateSendButton();
}

function sendAnswer() {
  if (current_answer === undefined) {
    return;
  }
  const answer = current_answer;
  submitAnswer("normal", answer.toString(), letters[answer]);
}
