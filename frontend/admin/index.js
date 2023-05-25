updateGroups();
updateQuestion();
updateQuestionState();
if (typeof EventSource !== "undefined") {
  //create an object, passing it the name and location of the server side script
  var eSource = new EventSource("/events");
  //detect message receipt
  eSource.onmessage = function (event) {
    //write the received data to the page
    if (event.data === "show_solution" || event.data === "show_answers") {
    } else if (event.data === "groups") {
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
  let score = parseInt(document.getElementById("questionPicker").value.trim());
  if (isNaN(score)) {
    alert("Bitte Zahl eingeben");
    return;
  }
  fetch("/questions/set", {
    method: "POST",
    body: score
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
  reader.onerror = (e) => {
    alert("error reading file");
  };
}

function getResults() {
  fetch("/questions/results").then((e) => {
    if (e.status == 200) {
      alert("Ergebnisse eingetragen");
    } else {
      alert("Fehler " + e.status + ": " + e.statusText);
    }
  });
}

function updateQuestion() {
  fetch("/questions/current")
    .then((e) => e.json())
    .then((data) => {
      let questionElem = document.getElementById("currentQuestion");
      console.log(data);
      if (data === null) {
        questionElem.innerHTML = "<tr><th>Frage:</b></tr><tr>Keine</tr>";
        return;
      }
      questionElem.innerHTML = "";
      for (const [type, questionData] of Object.entries(data[1])) {
        questionElem.innerHTML += `<tr><td>Typ</td><td>${type}</td></tr>`;
        questionElem.innerHTML += `<tr><td>Frage</td><td>${JSON.stringify(
          questionData.question
        )}</td></tr>`;
        if (
          questionData.answers !== null &&
          questionData.answers !== undefined
        ) {
          questionElem.innerHTML += `<tr><td>Antworten</td><td>${JSON.stringify(
            questionData.answers
          )}</td></tr>`;
        }
        questionElem.innerHTML += `<tr><td>Lösung</td><td>${JSON.stringify(
          questionData.solution
        )}</td></tr>`;
      }
    });
}

function updateGroups() {
  fetch("/groups/get")
    .then((e) => e.json())
    .then((data) => {
      let groupsElem = document.getElementById("groups");
      groupsElem.innerHTML = "<th>Gruppe</th> <th>Score</th> <th>Antwort</th>";
      for (const [groupName, groupData] of Object.entries(data).sort()) {
        let answer = JSON.stringify(groupData.answer);
        groupsElem.innerHTML += `<tr><td><b>${groupName}</b></td> <td>${
          groupData.score
        }</td> <td>${groupData.answer === null ? "Keine" : answer}</td></tr>`;
      }
    });
}

function updateQuestionState() {
  fetch("/questions/state", { method: "POST", body: "" })
    .then((e) => e.json())
    .then((data) => {
      let stateElem = document.getElementById("questionState");
      let questionState = data.questionState;
      stateElem.innerHTML = "";
      stateElem.innerHTML += `<tr><td>Antwort angezeigt</td><td>${
        !questionState.answerIsShown ? "Nein" : "Ja"
      }</td></tr>`;
      stateElem.innerHTML += `<tr><td>Lösung angezeigt</td><td>${
        !questionState.solutionIsShown ? "Nein" : "Ja"
      }</td></tr>`;
    });
}

function showAnswers() {
  fetch("/questions/show_answers").await;
}

function toggleLeaderboard() {
  fetch("/toggle_leaderboard").await;
}

function showSolution() {
  fetch("/questions/show_solution").await;
}

function setScore() {
  let groupName = document.getElementById("groupNameInput").value;
  let score = parseInt(document.getElementById("groupScoreInput").value.trim());
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
  let score = parseInt(document.getElementById("groupScoreInput").value.trim());
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