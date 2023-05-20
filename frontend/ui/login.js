function sendGroupName() {
    event.preventDefault();
    let name = document.getElementById("group_name_input").value;

    if (name.length === 0) {
        alert("Der Gruppenname darf nicht leer sein!")
        return
    }


    let xhr = new XMLHttpRequest();

    xhr.onreadystatechange = function () {
        if (xhr.readyState === XMLHttpRequest.DONE) {
            if (xhr.status >= 200 && xhr.status < 300) {
                // Success response
                console.log(xhr.response);
                localStorage.setItem("group_name", name);
            } else {
                // Error or redirect response
                console.log(xhr.status);
                console.log(xhr.statusText);
                localStorage.setItem("group_name", name);
                // Handle the error or redirect here
            }
            window.location.href = "/";
        }
    };

    xhr.open("POST", "/groups/new", true);
    xhr.setRequestHeader('Content-Type', 'text/plain');
    xhr.send(name);
    

    /*
     // Make the HTTP request
     fetch('/groups/new', {
         method: 'POST',
         headers: {
             'Content-Type': 'text/plain',
         },
         body: name,
     })
         .then(response => {
             localStorage.setItem("group_name", name)
             if (response.ok) {
                 // Redirect the user to a different page
                 window.location.href = '/';
             } else {
 
                 console.error('Request failed with status:', response.status);
             }
         })
 */
}