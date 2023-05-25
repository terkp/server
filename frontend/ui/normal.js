let current_answer = undefined;
let letters = ["A", "B", "C", "D"];

function setAnswer(answer) {
  current_answer = answer;
  document.querySelectorAll("#answerContainer button").forEach((b) => {
    b.disabled = false;
  });
  document.getElementById("button" + letters[answer]).disabled = true;
  if (
    localStorage.getItem("answerIsShown") === "false" &&
    localStorage.getItem("solutionIsShown") === "false"
  ) {
    document.getElementById("buttonSend").disabled = false;
  }
}

function sendAnswer() {
  if (current_answer === undefined) {
    return;
  }
  var name = localStorage.getItem("group_name");
  var data = JSON.stringify({
    name: name,
    type: "normal",
    answer: current_answer.toString(),
  });
  let xhr = new XMLHttpRequest();
  xhr.open("POST", "/groups/set_answer", true);
  xhr.setRequestHeader("Content-Type", "application/json");
  xhr.onload = (e) => {
    // If the group is not found, remove it from local storage and ask the user to login again
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
        letters[current_answer];
      document.getElementById("answer").style.display = "block";
    }
  };
  xhr.send(data);
}
