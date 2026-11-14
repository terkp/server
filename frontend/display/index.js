let letters = ["A", "B", "C", "D"];

const PICTURE_MODE_HIDDEN = 0;
const PICTURE_MODE_INLINE = 1;
const PICTURE_MODE_TOP = 2;
const PICTURE_MODE_FULLSCREEN = 3;

function showSolutionContent() {
  // The waiting page has no solution element.
  let solution = document.getElementById("solution");
  if (solution !== null) {
    solution.style.display = "block";
  }
}

function applyPictureMode(mode) {
  let root = document.getElementById("root");
  let pictureContainer = document.getElementById("pictureContainer");
  if (pictureContainer === null) {
    return;
  }

  root.classList.remove("picture-top-mode");
  root.classList.remove("picture-fullscreen-mode");

  if (mode === PICTURE_MODE_HIDDEN) {
    pictureContainer.classList.add("picture-hidden");
    return;
  }

  pictureContainer.classList.remove("picture-hidden");
  if (mode === PICTURE_MODE_TOP) {
    root.classList.add("picture-top-mode");
  } else if (mode === PICTURE_MODE_FULLSCREEN) {
    root.classList.add("picture-fullscreen-mode");
  }
}

// The groups' answers are in the page all the time, hidden by the style
// sheet. This style element overrides that once they are revealed.
var editCSS = document.createElement("style");
document.body.appendChild(editCSS);

function setGroupAnswersVisible(visible) {
  editCSS.textContent = visible ? ".groupAnswerText { display: block; }" : "";
}

// Brings the page in line with the state fetched from the server.
function applyQuestionState(state) {
  if (!Quiz.pageIsCurrent(state)) {
    // The page shows another question than the server is on. It reloads.
    return;
  }
  if (state.leaderboardShown) {
    Quiz.navigate("/display/leaderboard");
    return;
  }

  let questionState = state.questionState;
  if (questionState.solutionIsShown) {
    showSolutionContent();
  }
  setGroupAnswersVisible(questionState.answerIsShown);

  let pictureMode = questionState.pictureMode;
  if (!questionState.pictureReleased) {
    pictureMode = PICTURE_MODE_HIDDEN;
  }
  applyPictureMode(pictureMode);
}

function answerToString(answer) {
  const lettersSix = ["A", "B", "C", "D", "E", "F"];
  let answerString = "";
  if (answer.Sort != undefined) {
    for (const sortedAnswer of answer.Sort) {
      answerString += letters[sortedAnswer];
    }
  } else if (answer.Zuordnen != undefined) {
    for (const selectedAnswer of answer.Zuordnen) {
      answerString += lettersSix[selectedAnswer];
    }
  } else if (answer.Estimate != undefined) {
    answerString += answer.Estimate;
  } else {
    answerString += letters[answer.Normal];
  }
  return answerString;
}

// Group names are chosen by the players, so they are only ever put into the
// page as text, never as HTML.
function renderGroups(groups) {
  let answersElement = document.getElementById("group_answers");
  answersElement.textContent = "";
  let names = Object.keys(groups).sort((a, b) => a.localeCompare(b));
  for (const name of names) {
    let groupName = name.trim();
    if (groupName.length > 20) {
      groupName = groupName.substring(0, 18) + "...";
    }

    let row = document.createElement("div");
    row.className = "groupAnswer";

    let nameElement = document.createElement("span");
    nameElement.className = "groupName";
    nameElement.textContent = groupName;
    row.appendChild(nameElement);
    row.appendChild(document.createTextNode(": "));

    let answer = groups[name]["answer"];
    if (answer != null) {
      let answerElement = document.createElement("span");
      answerElement.className = "groupAnswerText";
      answerElement.textContent = answerToString(answer).trim();
      row.appendChild(answerElement);
    }
    answersElement.appendChild(row);
  }
}

// Each of these fetches from the server and applies the result; see
// Quiz.createSync for how overlapping and failed requests are handled.
const updateGroups = Quiz.createSync(Quiz.fetchGroups, renderGroups, true);
const resyncState = Quiz.createSync(
  () => Quiz.fetchState(""),
  applyQuestionState
);

function refreshEverything() {
  resyncState();
  updateGroups();
}

Quiz.connectEvents((data) => {
  // The events with a visible effect are applied right away. The state is
  // fetched as well, which also covers events that were missed before.
  if (data === "show_solution") {
    showSolutionContent();
    resyncState();
  } else if (data === "show_picture_inline") {
    applyPictureMode(PICTURE_MODE_INLINE);
    resyncState();
  } else if (data === "show_picture_top") {
    applyPictureMode(PICTURE_MODE_TOP);
    resyncState();
  } else if (data === "show_picture_fullscreen") {
    applyPictureMode(PICTURE_MODE_FULLSCREEN);
    resyncState();
  } else if (data === "hide_picture") {
    applyPictureMode(PICTURE_MODE_HIDDEN);
    resyncState();
  } else if (data === "show_answers") {
    setGroupAnswersVisible(true);
    resyncState();
  } else if (data === "groups") {
    updateGroups();
  } else {
    // "question", "toggle_leaderboard", "resync" and anything unknown: the
    // state tells whether to reload or to switch to the leaderboard. The
    // page is not reloaded blindly, so a reconnect does not blank the screen.
    refreshEverything();
  }
  // Everything is fetched whenever the connection opens, which includes the
  // first time (or after a short wait if it does not open; see
  // Quiz.connectEvents).
}, refreshEverything);

// Safety net for everything that can make the page miss an event, and for a
// server whose event stream cannot be reached at all.
setInterval(refreshEverything, 12000);

/*
1. Anzeige öffnen
2. EventStream dingens erstellen
3. Wenn zb Lösung angezeigt werden soll, event an eventstream geben
 */
