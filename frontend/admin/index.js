let currentQuestionData = null;
let currentQuestionType = null;
// The last answer of /questions/state, null until it was fetched once.
let latestState = null;
const letters = ["A", "B", "C", "D", "E", "F"];

// Sends a command to the server and afterwards refreshes everything shown on
// this page. `action` names the command in error messages. A failure is
// reported, except for the statuses listed in `quietStatuses`.
// Resolves to { ok, status, text }, or to null if the server did not answer.
function sendCommand(action, url, options, quietStatuses) {
  return Quiz.fetchWithTimeout(url, options, 8000)
    .then(
      (result) => {
        if (!result.ok && !(quietStatuses || []).includes(result.status)) {
          let message = action + " fehlgeschlagen (Status " + result.status + ")";
          // Our own handlers answer with a plain text reason, Rocket's
          // built-in error pages with HTML that is of no use here.
          if (result.text !== "" && !result.text.startsWith("<")) {
            message += ":\n" + result.text;
          }
          alert(message);
        }
        return result;
      },
      () => {
        alert(action + " fehlgeschlagen: Der Server antwortet nicht.");
        return null;
      }
    )
    .then((result) => {
      refreshEverything();
      return result;
    });
}

function nextQuestion() {
  // Telling the server which question we are coming from makes a double
  // click harmless: the second request no longer matches and is refused (409).
  let url = "/questions/next";
  if (latestState !== null) {
    url += "?from=" + latestState.questionIndex;
  }
  sendCommand("Nächste Frage", url, {}, [409]);
}

function goToQuestion() {
  let questionNumber = parseInt(
    document.getElementById("questionPicker").value.trim()
  );
  if (isNaN(questionNumber)) {
    alert("Bitte Zahl eingeben");
    return;
  }
  sendCommand("Gehe zu Frage", "/questions/set", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: String(questionNumber),
  });
}

let questionFilePicker = document.getElementById("questionFilePicker");

function loadQuestions() {
  if (questionFilePicker.files.length === 0) {
    alert("Bitte zuerst eine Datei auswählen");
    return;
  }
  let reader = new FileReader();
  reader.onload = (e) => {
    sendCommand("Fragen laden", "/questions/load", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: e.target.result,
    }).then((result) => {
      if (result !== null && result.ok) {
        alert(result.text + " Fragen geladen");
      }
    });
  };
  reader.onerror = () => {
    alert("Die Datei konnte nicht gelesen werden");
  };
  reader.readAsText(questionFilePicker.files[0]);
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

// Appends a table row and returns it. This is the only way rows are built on
// this page. The cells are inserted as text, never as HTML: group names and
// question texts must not be able to inject markup into this page.
function addRow(table, cells, cellTag) {
  let row = document.createElement("tr");
  for (const cell of cells) {
    let cellElement = document.createElement(cellTag || "td");
    cellElement.textContent = cell;
    row.appendChild(cellElement);
  }
  table.appendChild(row);
  return row;
}

// `data` is null if there is no current question, else [index, question].
function renderQuestion(data) {
  let questionElem = document.getElementById("currentQuestion");
  questionElem.textContent = "";
  if (data === null) {
    currentQuestionData = null;
    currentQuestionType = null;
    addRow(questionElem, ["Frage:"], "th");
    addRow(questionElem, ["Keine"]);
    renderPictureButtons(null);
    return;
  }

  addRow(questionElem, ["Nummer", data[0]]);
  for (const [type, questionData] of Object.entries(data[1])) {
    currentQuestionType = type;
    currentQuestionData = questionData;
    addRow(questionElem, ["Typ", type]);
    addRow(questionElem, ["Frage", JSON.stringify(questionData.question)]);
    if (questionData.answers !== null && questionData.answers !== undefined) {
      addRow(questionElem, [
        "Antworten",
        JSON.stringify(formatQuestionAnswers(type, questionData.answers)),
      ]);
    }
    addRow(questionElem, [
      "Lösung",
      JSON.stringify(formatSolution(type, questionData.solution)),
    ]);
  }
  renderPictureButtons(latestState === null ? null : latestState.questionState);
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

function renderGroups(data) {
  let groupsElem = document.getElementById("groups");
  groupsElem.textContent = "";
  addRow(groupsElem, ["Gruppe", "Score", "Antwort"], "th");
  let names = Object.keys(data).sort((a, b) => a.localeCompare(b));
  for (const name of names) {
    let row = addRow(groupsElem, [
      name,
      data[name].score,
      formatGroupAnswer(data[name].answer),
    ]);
    row.firstChild.style.fontWeight = "bold";
  }
}

function renderState(state) {
  latestState = state;
  let questionState = state.questionState;
  let stateElem = document.getElementById("questionState");
  stateElem.textContent = "";

  let finished =
    state.questionCount > 0 && state.questionIndex >= state.questionCount;
  addRow(stateElem, [
    "Fragen geladen",
    state.questionCount + (finished ? " (alle Fragen durch)" : ""),
  ]);
  addRow(stateElem, [
    "Antwort angezeigt",
    questionState.answerIsShown ? "Ja" : "Nein",
  ]);
  addRow(stateElem, [
    "Lösung angezeigt",
    questionState.solutionIsShown ? "Ja" : "Nein",
  ]);
  addRow(stateElem, [
    "Antworten gesperrt",
    state.answersBlocked ? "Ja" : "Nein",
  ]);
  addRow(stateElem, [
    "Display zeigt Punktestand",
    state.leaderboardShown ? "Ja" : "Nein",
  ]);

  document.getElementById("leaderboardButton").textContent =
    state.leaderboardShown ? "Punktestand ausblenden" : "Punktestand anzeigen";
  renderPictureButtons(questionState);
}

// Each of these fetches from the server and applies the result; see
// Quiz.createSync for how overlapping and failed requests are handled.
const updateGroups = Quiz.createSync(Quiz.fetchGroups, renderGroups, true);
const updateQuestion = Quiz.createSync(
  // Without a current question the answer is a 406 with the body "null".
  () =>
    Quiz.fetchWithTimeout("/questions/current", { cache: "no-store" }, 5000).then(
      (result) => JSON.parse(result.text)
    ),
  renderQuestion
);
const updateQuestionState = Quiz.createSync(
  () => Quiz.fetchState(""),
  renderState
);

function refreshEverything() {
  updateGroups();
  updateQuestion();
  updateQuestionState();
}

function showAnswers() {
  sendCommand("Antworten anzeigen", "/questions/show_answers");
}

function toggleLeaderboard() {
  // Send the wanted state instead of "toggle", so that a repeated request
  // does not flip the display back.
  let url = "/toggle_leaderboard";
  if (latestState !== null) {
    url += "?show=" + !latestState.leaderboardShown;
  }
  sendCommand("Punktestand umschalten", url);
}

function showSolution() {
  sendCommand("Lösung anzeigen", "/questions/show_solution");
}

function allowResultsEntry() {
  sendCommand("Ergebnisse erneut erlauben", "/questions/allow_results_entry");
}

function showPictureInline() {
  sendCommand("Bild anzeigen", "/questions/show_picture_inline");
}

function showPictureTop() {
  sendCommand("Bild anzeigen", "/questions/show_picture_top");
}

function showPictureFullscreen() {
  sendCommand("Bild anzeigen", "/questions/show_picture_fullscreen");
}

function hidePicture() {
  sendCommand("Bild ausblenden", "/questions/hide_picture");
}

function readScoreInputs() {
  let groupName = document.getElementById("groupNameInput").value;
  let score = parseFloat(
    document.getElementById("groupScoreInput").value.trim()
  );
  if (isNaN(score)) {
    alert("Bitte Zahl eingeben");
    return null;
  }
  return {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      name: groupName,
      score: score,
    }),
  };
}

function setScore() {
  let request = readScoreInputs();
  if (request !== null) {
    sendCommand("Punktzahl setzen", "/groups/set_score", request);
  }
}

function addScore() {
  let request = readScoreInputs();
  if (request !== null) {
    sendCommand("Punktzahl addieren", "/groups/add_score", request);
  }
}

function deleteGroup() {
  let groupName = document.getElementById("groupNameInputDelete").value;
  sendCommand("Gruppe löschen", "/groups/delete", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: groupName,
  });
}

Quiz.connectEvents((data) => {
  if (data === "groups") {
    updateGroups();
  } else if (data === "question" || data === "resync") {
    refreshEverything();
  } else {
    updateQuestionState();
  }
  // Everything is fetched whenever the connection opens, which includes the
  // first time, so there is no separate fetch when the page loads.
}, refreshEverything);

// Safety net for everything that can make the page miss an event, and for a
// server whose event stream cannot be reached at all.
setInterval(refreshEverything, 15000);
