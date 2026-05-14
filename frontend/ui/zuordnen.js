let selectedAnswers = new Set();
let letters = ["A", "B", "C", "D", "E", "F"];

function updateSelectionView() {
  let currentAnswer = document.getElementById("currentAnswer");
  let selected = Array.from(selectedAnswers).sort((a, b) => a - b);

  if (selected.length === 0) {
    currentAnswer.innerHTML = "";
    currentAnswer.style.display = "none";
  } else {
    currentAnswer.innerHTML = selected.map((idx) => letters[idx]).join(" ");
    currentAnswer.style.display = "block";
  }

  for (let i = 0; i < 6; i++) {
    let button = document.getElementById("button" + letters[i]);
    if (button === null) {
      continue;
    }
    button.classList.toggle("selectedAnswer", selectedAnswers.has(i));
  }

  document.getElementById("buttonSend").disabled =
    localStorage.getItem("answerIsShown") === "true" ||
    localStorage.getItem("solutionIsShown") === "true";
}

function toggleAnswer(answerIndex) {
  if (selectedAnswers.has(answerIndex)) {
    selectedAnswers.delete(answerIndex);
  } else {
    selectedAnswers.add(answerIndex);
  }
  updateSelectionView();
}

function clearAnswerSelection() {
  selectedAnswers.clear();
  updateSelectionView();
}

function sendAnswer() {
  let selected = Array.from(selectedAnswers).sort((a, b) => a - b);
  let answerString = selected.map((idx) => letters[idx]).join("");
  let name = localStorage.getItem("group_name");
  let data = JSON.stringify({
    name: name,
    type: "zuordnen",
    answer: answerString,
  });

  let xhr = new XMLHttpRequest();
  xhr.open("POST", "/groups/set_answer", true);
  xhr.setRequestHeader("Content-Type", "application/json");
  xhr.onload = () => {
    if (
      xhr.status >= 400 &&
      xhr.status < 500 &&
      xhr.responseText.includes("not found")
    ) {
      localStorage.removeItem("group_name");
      alert("Bitte logge dich erneut ein");
      window.location.href = "/login";
    } else if (xhr.status == 200) {
      document.getElementById("answerContent").innerHTML =
        selected.map((idx) => letters[idx]).join(" ");
      document.getElementById("answer").style.display = "block";
    }
  };
  xhr.send(data);
}

updateSelectionView();
