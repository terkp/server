// The leaderboard page of the display. The server renders the scores into the
// page; this script keeps them current and switches back to the question
// when the leaderboard is turned off.

// Same order as the server: highest score first, equal scores by name.
function renderScores(groups) {
  let names = Object.keys(groups).sort((a, b) => {
    if (groups[a].score !== groups[b].score) {
      return groups[b].score - groups[a].score;
    }
    return a < b ? -1 : a > b ? 1 : 0;
  });

  let scoresElement = document.getElementById("groupScores");
  scoresElement.textContent = "";
  for (const name of names) {
    let row = document.createElement("div");
    row.className = "groupScore";

    let nameElement = document.createElement("span");
    nameElement.className = "groupName";
    nameElement.textContent = name;
    row.appendChild(nameElement);
    row.appendChild(document.createTextNode(": "));

    let scoreElement = document.createElement("span");
    scoreElement.className = "score";
    scoreElement.textContent = groups[name].score;
    row.appendChild(scoreElement);

    scoresElement.appendChild(row);
  }
}

// Each of these fetches from the server and applies the result; see
// Quiz.createSync for how overlapping and failed requests are handled.
// "groups" events arrive in bursts (one per answer); the scores are redrawn
// in place instead of reloading the page for each of them.
const updateScores = Quiz.createSync(Quiz.fetchGroups, renderScores, true);
const resyncState = Quiz.createSync(
  () => Quiz.fetchState(""),
  (state) => {
    if (!state.leaderboardShown) {
      Quiz.navigate("/display");
    }
  }
);

function refreshEverything() {
  resyncState();
  updateScores();
}

Quiz.connectEvents((data) => {
  if (data === "groups") {
    updateScores();
  } else if (data === "toggle_leaderboard" || data === "resync") {
    refreshEverything();
  }
  // Everything is fetched whenever the connection opens, which includes the
  // first time, so there is no separate fetch when the page loads.
}, refreshEverything);

// Safety net for everything that can make the page miss an event, and for a
// server whose event stream cannot be reached at all.
setInterval(refreshEverything, 12000);
