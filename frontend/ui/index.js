console.log("current group: " + localStorage.getItem("group_name"));

let isLoginPage = window.location.href.endsWith("/login");

if (localStorage.getItem("group_name") === null && !isLoginPage) {
  window.location.href = "/login";
}

if (localStorage.getItem("group_name") !== null) {
  document.getElementById("currentGroup").innerHTML =
    "Deine Gruppe: " + localStorage.getItem("group_name");
}

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
  document.getElementById("hallo").innerHTML =
    "Whoops! Your browser doesn't receive server-sent events.";
}
/*
1. Anzeige öffnen 
2. EventStream dingens erstellen
3. Wenn zb Lösung angezeigt werden soll, event an eventstream geben
 */
