// This script runs before the buttons and ui/index.js are loaded, so it must
// not touch the page at the top level. See the comment at the top of index.js.

let selectedAnswers = new Set();
// An empty selection is a valid answer, but only once the player has touched
// the buttons, so that a stray tap on "send" does not submit nothing.
let selectionTouched = false;
let letters = ["A", "B", "C", "D", "E", "F"];

function hasCompleteAnswer() {
  return selectionTouched;
}

function selectedIndices() {
  return Array.from(selectedAnswers).sort((a, b) => a - b);
}

function updateSelectionView() {
  let currentAnswer = document.getElementById("currentAnswer");
  let selected = selectedIndices();

  if (selected.length === 0) {
    currentAnswer.textContent = "";
    currentAnswer.style.display = "none";
  } else {
    currentAnswer.textContent = selected.map((idx) => letters[idx]).join(" ");
    currentAnswer.style.display = "block";
  }

  for (let i = 0; i < 6; i++) {
    let button = document.getElementById("button" + letters[i]);
    if (button === null) {
      continue;
    }
    button.classList.toggle("selectedAnswer", selectedAnswers.has(i));
  }

  updateSendButton();
}

function toggleAnswer(answerIndex) {
  selectionTouched = true;
  if (selectedAnswers.has(answerIndex)) {
    selectedAnswers.delete(answerIndex);
  } else {
    selectedAnswers.add(answerIndex);
  }
  updateSelectionView();
}

function clearAnswerSelection() {
  selectionTouched = true;
  selectedAnswers.clear();
  updateSelectionView();
}

function sendAnswer() {
  if (!hasCompleteAnswer()) {
    return;
  }
  let selected = selectedIndices();
  submitAnswer(
    "zuordnen",
    selected.map((idx) => letters[idx]).join(""),
    selected.map((idx) => letters[idx]).join(" ")
  );
}
