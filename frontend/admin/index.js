updateGroups();
updateQuestion();
updateQuestionState();

let currentQuestionData = null;
let currentQuestionType = null;
let latestQuestionState = null;
const letters = ["A", "B", "C", "D", "E", "F"];

if (typeof EventSource !== "undefined") {
  var eSource = new EventSource("/events");
  eSource.onmessage = function (event) {
    if (event.data === "groups") {
      updateGroups();
    } else if (event.data === "question") {
      updateQuestion();
    }
    updateQuestionState();
  };
} else {
  alert("Whoops! Your browser doesn't receive server-sent events.");
}

function nextQuestion() {
  fetch("/questions/next").then((_) => updateQuestion());
}

function goToQuestion() {
  let questionNumber = parseInt(
    document.getElementById("questionPicker").value.trim()
  );
  if (isNaN(questionNumber)) {
    alert("Bitte Zahl eingeben");
    return;
  }
  fetch("/questions/set", {
    method: "POST",
    body: questionNumber,
  }).then((_) => updateQuestion());
}

let questionFilePicker = document.getElementById("questionFilePicker");

function loadQuestions() {
  let reader = new FileReader();
  reader.readAsText(questionFilePicker.files[0]);
  reader.onload = (e) => {
    fetch("/questions/load", {
      method: "POST",
      body: e.target.result,
    });
  };
  reader.onerror = () => {
    alert("error reading file");
  };
}

function formatSolution(type, solution) {
  if (type === "Zuordnen" && Array.isArray(solution)) {
    let values = [];
    for (let i = 0; i < solution.length; i++) {
      if (solution[i]) {
        values.push(letters[i]);
      }
    }
    return values.join("");
  }
  return solution;
}

function formatQuestionAnswers(type, answers) {
  if (!Array.isArray(answers)) {
    return answers;
  }
  if (type === "Zuordnen") {
    return answers
      .map((answer, idx) => `${letters[idx]}: ${answer}`)
      .join(" | ");
  }
  return answers;
}

function formatGroupAnswer(answer) {
  if (answer === null || answer === undefined) {
    return "Keine";
  }
  if (answer.Zuordnen !== undefined) {
    return answer.Zuordnen.map((idx) => letters[idx]).join("");
  }
  return JSON.stringify(answer);
}

function updateQuestion() {
  fetch("/questions/current")
    .then((e) => e.json())
    .then((data) => {
      let questionElem = document.getElementById("currentQuestion");
      if (data === null) {
        currentQuestionData = null;
        currentQuestionType = null;
        questionElem.innerHTML = "<tr><th>Frage:</b></tr><tr>Keine</tr>";
        renderPictureButtons(null);
        return;
      }

      questionElem.innerHTML = `<tr><td>Nummer</td><td>${data[0]}</td></tr>`;
      for (const [type, questionData] of Object.entries(data[1])) {
        currentQuestionType = type;
        currentQuestionData = questionData;
        questionElem.innerHTML += `<tr><td>Typ</td><td>${type}</td></tr>`;
        questionElem.innerHTML += `<tr><td>Frage</td><td>${JSON.stringify(
          questionData.question
        )}</td></tr>`;
        if (
          questionData.answers !== null &&
          questionData.answers !== undefined
        ) {
          questionElem.innerHTML += `<tr><td>Antworten</td><td>${JSON.stringify(
            formatQuestionAnswers(type, questionData.answers)
          )}</td></tr>`;
        }
        questionElem.innerHTML += `<tr><td>Lösung</td><td>${JSON.stringify(
          formatSolution(type, questionData.solution)
        )}</td></tr>`;
      }
      renderPictureButtons(latestQuestionState);
    });
}

function getPictureMeta() {
  if (currentQuestionData === null || currentQuestionData.picture === null) {
    return null;
  }
  return currentQuestionData.picture;
}

function createPictureButton(label, handler) {
  let button = document.createElement("button");
  button.textContent = label;
  button.onclick = handler;
  return button;
}

function renderPictureButtons(questionState) {
  let pictureButtons = document.getElementById("pictureButtons");
  pictureButtons.innerHTML = "";

  if (questionState === null) {
    return;
  }

  let picture = getPictureMeta();
  if (picture === null) {
    return;
  }

  if (picture.timing === "SolutionOnly" && !questionState.answerIsShown) {
    return;
  }

  if (currentQuestionType === "Estimate") {
    pictureButtons.appendChild(
      createPictureButton("Bild unten", showPictureInline)
    );
  }
  if (
    currentQuestionType === "Normal" ||
    currentQuestionType === "Sort" ||
    currentQuestionType === "Zuordnen"
  ) {
    pictureButtons.appendChild(
      createPictureButton("Bild über Frage", showPictureTop)
    );
  }
  pictureButtons.appendChild(
    createPictureButton("Bild Vollbild", showPictureFullscreen)
  );
  pictureButtons.appendChild(
    createPictureButton("Bild ausblenden", hidePicture)
  );
}

function updateGroups() {
  fetch("/groups/get")
    .then((e) => e.json())
    .then((data) => {
      let groupsElem = document.getElementById("groups");
      groupsElem.innerHTML = "<th>Gruppe</th> <th>Score</th> <th>Antwort</th>";
      for (const [groupName, groupData] of Object.entries(data).sort()) {
        groupsElem.innerHTML += `<tr><td><b>${groupName}</b></td> <td>${
          groupData.score
        }</td> <td>${formatGroupAnswer(groupData.answer)}</td></tr>`;
      }
    });
}

function updateQuestionState() {
  fetch("/questions/state", { method: "POST", body: "" })
    .then((e) => e.json())
    .then((data) => {
      let stateElem = document.getElementById("questionState");
      let questionState = data.questionState;
      latestQuestionState = questionState;
      stateElem.innerHTML = "";
      stateElem.innerHTML += `<tr><td>Antwort angezeigt</td><td>${
        !questionState.answerIsShown ? "Nein" : "Ja"
      }</td></tr>`;
      stateElem.innerHTML += `<tr><td>Lösung angezeigt</td><td>${
        !questionState.solutionIsShown ? "Nein" : "Ja"
      }</td></tr>`;
      renderPictureButtons(questionState);
    });
}

function showAnswers() {
  fetch("/questions/show_answers").await;
}

function toggleLeaderboard() {
  fetch("/toggle_leaderboard").await;
}

function showSolution() {
  fetch("/questions/show_solution").then((_) => updateQuestionState());
}

function allowResultsEntry() {
  fetch("/questions/allow_results_entry").then((_) => updateQuestionState());
}

function showPictureInline() {
  fetch("/questions/show_picture_inline").then((_) => updateQuestionState());
}

function showPictureTop() {
  fetch("/questions/show_picture_top").then((_) => updateQuestionState());
}

function showPictureFullscreen() {
  fetch("/questions/show_picture_fullscreen").then((_) => updateQuestionState());
}

function hidePicture() {
  fetch("/questions/hide_picture").then((_) => updateQuestionState());
}

function setScore() {
  let groupName = document.getElementById("groupNameInput").value;
  let score = parseFloat(
    document.getElementById("groupScoreInput").value.trim()
  );
  if (isNaN(score)) {
    alert("Bitte Zahl eingeben");
    return;
  }
  fetch("/groups/set_score", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      name: groupName,
      score: score,
    }),
  }).await;
  updateGroups();
}

function addScore() {
  let groupName = document.getElementById("groupNameInput").value;
  let score = parseFloat(
    document.getElementById("groupScoreInput").value.trim()
  );
  if (isNaN(score)) {
    alert("Bitte Zahl eingeben");
    return;
  }
  fetch("/groups/add_score", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      name: groupName,
      score: score,
    }),
  }).await;
  updateGroups();
}

function deleteGroup() {
  let groupName = document.getElementById("groupNameInputDelete").value;
  fetch("/groups/delete", {
    method: "POST",
    body: groupName,
  }).await;
  updateGroups();
}
