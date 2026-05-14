let letters = ["A", "B", "C", "D"];

const PICTURE_MODE_HIDDEN = 0;
const PICTURE_MODE_INLINE = 1;
const PICTURE_MODE_TOP = 2;
const PICTURE_MODE_FULLSCREEN = 3;

function showSolutionContent() {
  document.getElementById("solution").style.display = "block";
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

function applyQuestionState(groupQuestionState) {
  localStorage.setItem(
    "answerIsShown",
    groupQuestionState.questionState.answerIsShown
  );
  localStorage.setItem(
    "solutionIsShown",
    groupQuestionState.questionState.solutionIsShown
  );
  if (groupQuestionState.questionState.solutionIsShown) {
    showSolutionContent();
  }
  if (groupQuestionState.questionState.answerIsShown) {
    editCSS.innerHTML += ".groupAnswerText { display: block; }";
  }

  let pictureMode = groupQuestionState.questionState.pictureMode;
  if (!groupQuestionState.questionState.pictureReleased) {
    pictureMode = PICTURE_MODE_HIDDEN;
  }
  applyPictureMode(pictureMode);
}

var editCSS = document.createElement("style");
editCSS.innerHTML = ".answerText {display: none;}";
document.body.appendChild(editCSS);
updateGroups();

{
  let xhr = new XMLHttpRequest();
  xhr.open("POST", "/questions/state", true);
  xhr.onload = (e) => {
    console.log(xhr.status + ": " + xhr.responseText);
    if (xhr.status === 200) {
      applyQuestionState(JSON.parse(xhr.responseText));
    }
  };
  xhr.send(localStorage.getItem("group_name"));
}

if (typeof EventSource !== "undefined") {
  //create an object, passing it the name and location of the server side script
  var eSource = new EventSource("/events");
  //detect message receipt
  eSource.onmessage = function (event) {
    //write the received data to the page
    if (event.data === "show_solution") {
      showSolutionContent();
      localStorage.setItem("solutionIsShown", true);
    } else if (event.data === "show_picture_inline") {
      applyPictureMode(PICTURE_MODE_INLINE);
    } else if (event.data === "show_picture_top") {
      applyPictureMode(PICTURE_MODE_TOP);
    } else if (event.data === "show_picture_fullscreen") {
      applyPictureMode(PICTURE_MODE_FULLSCREEN);
    } else if (event.data === "hide_picture") {
      applyPictureMode(PICTURE_MODE_HIDDEN);
    } else if (event.data === "show_answers") {
      editCSS.innerHTML = ".groupAnswerText {display: block;}";
      localStorage.setItem("answerIsShown", true);
    } else if (event.data === "toggle_leaderboard") {
      window.location.href = "/display/leaderboard";
    } else if (event.data === "groups") {
      updateGroups();
    } else {
      location.reload(true);
    }
  };
} else {
  alert("Whoops! Your browser doesn't receive server-sent events.");
}

function answerToString(answer) {
  const lettersSix = ["A", "B", "C", "D", "E", "F"];
  let answerString = "";
  if (answer.Sort != undefined) {
    for (sortedAnswer of answer.Sort) {
      answerString += letters[sortedAnswer];
    }
  } else if (answer.Zuordnen != undefined) {
    for (selectedAnswer of answer.Zuordnen) {
      answerString += lettersSix[selectedAnswer];
    }
  } else if (answer.Estimate != undefined) {
    answerString += answer.Estimate;
  } else {
    answerString += letters[answer.Normal];
  }
  return answerString;
}

function updateGroups() {
  let answersElement = document.getElementById("group_answers");
  let xhr = new XMLHttpRequest();
  xhr.open("GET", "/groups/get");
  xhr.send();
  xhr.onload = (e) => {
    answersElement.innerHTML = "";
    let answers = JSON.parse(xhr.responseText);
    for (let [groupName, answer] of Object.entries(answers).sort()) {
      if (groupName.length > 20) {
        groupName = groupName.substring(0, 18) + "...";
      }
      let answerText = "";
      if (answer["answer"] != null) {
        answerText =
          '<span class="groupAnswerText">' +
          answerToString(answer["answer"]).trim() +
          "</span>";
      }
      answersElement.innerHTML +=
        '<div class="groupAnswer"><span class="groupName">' +
        groupName.trim() +
        "</span>: " +
        answerText.trim() +
        "<br></div>";
    }
  };
}

/*
1. Anzeige öffnen 
2. EventStream dingens erstellen
3. Wenn zb Lösung angezeigt werden soll, event an eventstream geben
 */
