// Logic shared by all player pages.
//
// The per-type scripts (normal.js, estimate.js, sort.js, zuordnen.js) are
// loaded before this file and before the buttons below them exist. They only
// define functions; what they need from here (updateSendButton, submitAnswer)
// they call from their click handlers, when everything is loaded.
// Each of them defines hasCompleteAnswer(), which says whether the player has
// entered something that can be sent.

let pictureFullscreenActive = false;

// True once answers are no longer accepted for this question. Until the
// server state is known it is false: a failed state request must not keep
// the send button disabled.
let quizLocked = false;

// True from sending an answer until the server state was fetched again
// afterwards. Sending is disabled meanwhile.
let answerRequestInFlight = false;

let resyncState = function () {};

function currentGroupName() {
  return localStorage.getItem("group_name");
}

function showGroupLabel() {
  document.getElementById("currentGroup").textContent =
    "Deine Gruppe: " + currentGroupName();
}

// Enables the send button if there is something to send and sending is allowed.
function updateSendButton() {
  const button = document.getElementById("buttonSend");
  if (button === null) {
    return;
  }
  const complete =
    typeof hasCompleteAnswer === "function" && hasCompleteAnswer();
  button.disabled = quizLocked || answerRequestInFlight || !complete;
}

function setLocked(locked) {
  quizLocked = locked;
  updateSendButton();
}

function showSolution() {
  document.getElementById("solution").style.display = "block";
  updatePictureFullscreenButton();
}

function showSubmittedAnswer(text) {
  document.getElementById("answerContent").textContent = text;
  document.getElementById("answer").style.display = "block";
}

// Shows the answer the server has stored for the group (null: none).
function showServerAnswer(answer) {
  if (answer === null || answer === undefined) {
    document.getElementById("answer").style.display = "none";
    return;
  }
  const answerLetters = ["A", "B", "C", "D", "E", "F"];
  let text = "";
  if (answer.Normal !== undefined) {
    text = answerLetters[answer.Normal];
  } else if (answer.Estimate !== undefined) {
    text = String(answer.Estimate);
  } else if (answer.Sort !== undefined) {
    text = answer.Sort.map((idx) => answerLetters[idx]).join("");
  } else if (answer.Zuordnen !== undefined) {
    text = answer.Zuordnen.map((idx) => answerLetters[idx]).join(" ");
  }
  showSubmittedAnswer(text);
}

function goToLogin() {
  localStorage.removeItem("group_name");
  Quiz.navigate("/login");
}

// Brings the page in line with the state fetched from the server.
function applyState(state) {
  if (!Quiz.pageIsCurrent(state)) {
    // The page shows another question than the server is on. It reloads.
    return;
  }
  if (!state.groupExists) {
    // The group was deleted, or the server was restarted.
    goToLogin();
    return;
  }
  if (state.groupName !== null && state.groupName !== currentGroupName()) {
    // Take over the server's spelling of the name (it trims names).
    localStorage.setItem("group_name", state.groupName);
    showGroupLabel();
  }

  const questionState = state.questionState;
  setLocked(
    questionState.answerIsShown ||
      questionState.solutionIsShown ||
      state.answersBlocked
  );
  if (questionState.solutionIsShown) {
    showSolution();
  }
  if (!answerRequestInFlight) {
    showServerAnswer(state.answer);
  }
  updatePictureFullscreenButton();
}

// Sends an answer for the group.
//   type: the question type as the server names it ("normal", "schaetzen", ...)
//   answerString: the answer in the server's format
//   displayText: what to show as "Deine Antwort" if the answer was stored.
//     Captured by the caller when sending, because the player can keep
//     changing the selection while the request is on its way.
//   onSuccess: optional, called once the answer was stored
function submitAnswer(type, answerString, displayText, onSuccess) {
  if (answerRequestInFlight) {
    return;
  }
  answerRequestInFlight = true;
  updateSendButton();

  // A page without a version sends none, and the server then skips the
  // check. (Number(null) and Number("") would both be 0, which the server
  // would refuse as an outdated version every time.)
  const pageVersion = Quiz.pageVersion();
  const version =
    pageVersion === null || pageVersion.trim() === ""
      ? NaN
      : Number(pageVersion);
  Quiz.fetchWithTimeout(
    "/groups/set_answer",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: currentGroupName(),
        type: type,
        answer: answerString,
        // Lets the server refuse an answer that was meant for another question.
        question_version: Number.isFinite(version) ? version : null,
      }),
    },
    8000
  )
    .then(
      (result) => {
        if (result.status === 200) {
          showSubmittedAnswer(displayText);
          if (onSuccess) {
            onSuccess();
          }
        } else if (result.status === 410) {
          alert("Bitte logge dich erneut ein");
          goToLogin();
        } else if (result.status === 403) {
          setLocked(true);
          alert(
            "Die Antworten wurden schon aufgedeckt. Deine Antwort wurde nicht mehr angenommen."
          );
        } else if (result.status === 409) {
          alert(
            "Die Frage hat gewechselt. Deine Antwort wurde nicht gespeichert."
          );
        } else {
          alert(
            "Die Antwort konnte nicht gespeichert werden (Fehler " +
              result.status +
              "). Bitte versuche es noch einmal."
          );
        }
      },
      () => {
        // No answer from the server. The request may or may not have arrived.
        alert(
          "Keine Verbindung zum Server. Sobald die Verbindung wieder da ist, " +
            'steht unter "Deine Antwort", ob deine Antwort angekommen ist. ' +
            "Wenn nicht, sende sie bitte noch einmal."
        );
      }
    )
    .catch((error) => {
      console.error(error);
    })
    .then(() => {
      // Whatever happened, fetch what the server has stored now and show
      // that. Sending stays disabled until then, so that after a timeout an
      // old request cannot arrive after a newer one unnoticed.
      return resyncState.whenApplied();
    })
    .then((state) => {
      answerRequestInFlight = false;
      if (String(state.questionVersion) === Quiz.pageVersion()) {
        showServerAnswer(state.answer);
      }
      updateSendButton();
    });
}

function updatePictureFullscreenButton() {
  const buttons = document.querySelectorAll(".pictureFullscreenButton");
  const hasPicture =
    document.getElementById("questionImageBefore") !== null ||
    document.getElementById("questionImageAfter") !== null;
  if (buttons.length === 0) {
    return;
  }
  if (!hasPicture) {
    buttons.forEach((button) => {
      button.style.display = "none";
    });
    return;
  }
  buttons.forEach((button) => {
    button.style.display = "inline-flex";
    button.textContent = pictureFullscreenActive ? "Verlassen" : "Vollbild";
  });
}

async function enterPictureFullscreen() {
  const root = document.getElementById("root");
  if (root === null) {
    return;
  }
  root.classList.add("picture-fullscreen-mode");
  pictureFullscreenActive = true;
  updatePictureFullscreenButton();

  if (root.requestFullscreen) {
    try {
      await root.requestFullscreen({ navigationUI: "hide" });
    } catch (error) {
      console.warn("Fullscreen request failed", error);
    }
  }

  // Do not force landscape orientation for player fullscreen.
}

async function exitPictureFullscreen() {
  const root = document.getElementById("root");
  if (root !== null) {
    root.classList.remove("picture-fullscreen-mode");
  }
  pictureFullscreenActive = false;
  updatePictureFullscreenButton();

  if (document.fullscreenElement && document.exitFullscreen) {
    try {
      await document.exitFullscreen();
    } catch (error) {
      console.warn("Exit fullscreen failed", error);
    }
  }

  // No orientation lock to release.
}

function togglePictureFullscreen() {
  if (pictureFullscreenActive) {
    exitPictureFullscreen();
  } else {
    enterPictureFullscreen();
  }
}

document.addEventListener("fullscreenchange", () => {
  pictureFullscreenActive = document.fullscreenElement !== null;
  updatePictureFullscreenButton();
  if (!pictureFullscreenActive) {
    const root = document.getElementById("root");
    if (root !== null) {
      root.classList.remove("picture-fullscreen-mode");
    }
  }
});

window.addEventListener("load", updatePictureFullscreenButton);

function handleServerEvent(data) {
  if (data === "show_solution") {
    setLocked(true);
    showSolution();
    resyncState();
  } else if (data === "show_answers") {
    setLocked(true);
    resyncState();
  } else if (data === "question" || data === "resync") {
    // A new question shows up as a new question version in the state, and
    // the page reloads then.
    resyncState();
  }
  // Everything else (groups, pictures, leaderboard) only concerns the display.
}

function startQuiz() {
  showGroupLabel();
  updateSendButton();

  resyncState = Quiz.createSync(
    () => Quiz.fetchState(currentGroupName()),
    applyState
  );
  // The state is fetched whenever the event connection opens, which includes
  // the first time (or after a short wait if it does not open; see
  // Quiz.connectEvents).
  //
  // The connection is closed while the page is hidden (third argument), so
  // that background tabs do not use up the browser's connections. When the
  // player comes back, a new one is opened, which also fetches the state.
  Quiz.connectEvents(handleServerEvent, () => resyncState(), true);

  // Safety net for everything that can make the page miss an event, and for
  // a server whose event stream cannot be reached at all. It runs for hidden
  // pages too, in case a browser reports a page as hidden that is not.
  setInterval(() => resyncState(), 12000);
}

if (currentGroupName() === null) {
  window.location.href = "/login";
} else {
  startQuiz();
}
