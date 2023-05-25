let letters = ["A", "B", "C", "D"];

var editCSS = document.createElement("style");
editCSS.innerHTML = ".answerText {display: none;}";
document.body.appendChild(editCSS);
updateGroups();

if (typeof EventSource !== "undefined") {
  //create an object, passing it the name and location of the server side script
  var eSource = new EventSource("/events");
  //detect message receipt
  eSource.onmessage = function (event) {
    //write the received data to the page
    if (event.data === "show_solution") {
      document.getElementById("solution").style.display = "block";
    } else if (event.data === "show_answers") {
      editCSS.innerHTML = ".groupAnswerText {display: block;}";
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
  let answerString = "";
  if (answer.Sort != undefined) {
    for (sortedAnswer of answer.Sort) {
      answerString += letters[sortedAnswer];
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
    for (var groupName in answers) {
      let answer = answers[groupName];
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
