if(typeof(EventSource)!=="undefined") {
	//create an object, passing it the name and location of the server side script
	var eSource = new EventSource("http://localhost:8000/display/events");
	//detect message receipt
	eSource.onmessage = function(event) {
		//write the received data to the page
		document.getElementById("hallo").innerHTML += event.data;
	};
}
else {
	document.getElementById("hallo").innerHTML="Whoops! Your browser doesn't receive server-sent events.";
}
/*
1. Anzeige öffnen 
2. EventStream dingens erstellen
3. Wenn zb Lösung angezeigt werden soll, event an eventstream geben


 */