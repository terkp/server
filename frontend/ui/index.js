console.log("das wird geladen")
console.log(window.location.hostname)
console.log(localStorage.getItem("group_name"))
console.log(window.location.href)

let isLoginPage = window.location.href.endsWith("/login");

if (localStorage.getItem("group_name") === null && !isLoginPage) {
    window.location.href = "/login"
}

if (localStorage.getItem("group_name") !== null) {
    document.getElementById("current_group").innerHTML = "Deine Gruppe: " + localStorage.getItem("group_name")
}

if (typeof (EventSource) !== "undefined") {
    //create an object, passing it the name and location of the server side script
    var eSource = new EventSource("/events");
    //detect message receipt
    eSource.onmessage = function (event) {
        //write the received data to the page
        //document.getElementById("hallo").innerHTML += event.data;
        if (event.data === "show_solution") {
            document.getElementById("solution").style.display = "block"
            document.getElementById("buttonSend").disabled = true;
        } else if (event.data === "show_answers") {
            document.getElementById("buttonSend").disabled = true;
        } else if (event.data === "question") {
            location.reload(true)
        }
    }
} else {
    document.getElementById("hallo").innerHTML = "Whoops! Your browser doesn't receive server-sent events.";
}
/*
1. Anzeige öffnen 
2. EventStream dingens erstellen
3. Wenn zb Lösung angezeigt werden soll, event an eventstream geben
 */
