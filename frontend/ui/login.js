function sendGroupName() {
    let name = document.getElementById("group_name_input").value;

    if (name.length === 0) {
        alert("Der Gruppenname darf nicht leer sein!")
        return
    }

    let xhr = new XMLHttpRequest();
    
    xhr.onload = function () {
        console.log(xhr.response)
        console.log("???????sdffd")
        localStorage.setItem("group_name", name)
        console.log("???????????")
        window.location.href = "/"
    }
    xhr.open("POST", "/groups/new")
    xhr.setRequestHeader('Content-Type', 'text/plain');
    xhr.send(name)
    

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