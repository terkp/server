let loginRequestInFlight = false;

function sendGroupName() {
  if (loginRequestInFlight) {
    return;
  }
  const name = document.getElementById("groupNameInput").value.trim();

  if (name.length === 0) {
    alert("Der Gruppenname darf nicht leer sein!");
    return;
  }

  const button = document.getElementById("buttonSend");
  loginRequestInFlight = true;
  button.disabled = true;

  Quiz.fetchWithTimeout(
    "/groups/new",
    {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: name,
    },
    8000
  )
    .then((result) => {
      if (result.ok) {
        // The group was created, or it already existed and is joined.
        // The server answers with the name as it stores it; that is the one
        // to remember, so that later lookups find the group.
        localStorage.setItem("group_name", JSON.parse(result.text).name);
        window.location.href = "/";
        return;
      }
      if (result.status === 400 && !result.text.startsWith("<")) {
        // The server's reason why the name cannot be used.
        alert(result.text);
      } else {
        alert(
          "Die Anmeldung hat nicht geklappt (Fehler " +
            result.status +
            "). Bitte versuche es noch einmal."
        );
      }
    })
    .catch(() => {
      // Do not store the name: the group does not exist on the server.
      alert(
        "Keine Verbindung zum Server. Bitte prüfe das WLAN und versuche es noch einmal."
      );
    })
    .then(() => {
      loginRequestInFlight = false;
      button.disabled = false;
    });
}
