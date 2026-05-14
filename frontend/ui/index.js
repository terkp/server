console.log("current group: " + localStorage.getItem("group_name"));

let isLoginPage = window.location.href.endsWith("/login");
let pictureFullscreenActive = false;

if (localStorage.getItem("group_name") === null && !isLoginPage) {
  window.location.href = "/login";
}

if (localStorage.getItem("group_name") !== null) {
  document.getElementById("currentGroup").innerHTML =
    "Deine Gruppe: " + localStorage.getItem("group_name");
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

  if (screen.orientation && screen.orientation.lock) {
    try {
      await screen.orientation.lock("landscape");
    } catch (error) {
      console.warn("Orientation lock failed", error);
    }
  }
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

  if (screen.orientation && screen.orientation.unlock) {
    try {
      screen.orientation.unlock();
    } catch (error) {
      console.warn("Orientation unlock failed", error);
    }
  }
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

{
  let xhr = new XMLHttpRequest();
  xhr.open("POST", "/questions/state", true);
  xhr.onload = (e) => {
    console.log(xhr.status + ": " + xhr.responseText);
    if (xhr.status === 200) {
      let groupQuestionState = JSON.parse(xhr.responseText);
      localStorage.setItem(
        "answerIsShown",
        groupQuestionState.questionState.answerIsShown
      );
      localStorage.setItem(
        "solutionIsShown",
        groupQuestionState.questionState.solutionIsShown
      );
      if (groupQuestionState.questionState.solutionIsShown) {
        document.getElementById("solution").style.display = "block";
        document.getElementById("buttonSend").disabled = true;
      }
      if (groupQuestionState.questionState.answerIsShown) {
        document.getElementById("buttonSend").disabled = true;
      }
      updatePictureFullscreenButton();
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
    //document.getElementById("hallo").innerHTML += event.data;
    if (event.data === "show_solution") {
      document.getElementById("solution").style.display = "block";
      document.getElementById("buttonSend").disabled = true;
      localStorage.setItem("solutionIsShown", true);
    } else if (event.data === "show_answers") {
      document.getElementById("buttonSend").disabled = true;
      localStorage.setItem("answerIsShown", true);
    } else if (event.data === "question") {
      location.reload(true);
    }
  };
} else {
  alert("Whoops! Your browser doesn't receive server-sent events.");
}
/*
1. Anzeige öffnen 
2. EventStream dingens erstellen
3. Wenn zb Lösung angezeigt werden soll, event an eventstream geben
 */
