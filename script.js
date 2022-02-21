function send_request(path, data, method, callback) {
    var url = "http://localhost:12345" + path;
    var xhr = new XMLHttpRequest();
    xhr.open(method, url, true);
    xhr.onreadystatechange = function() {
        if (xhr.readyState === XMLHttpRequest.DONE && xhr.status === 200) {
            callback(JSON.parse(xhr.responseText));
        }
    }
    if (data) {
        xhr.send(JSON.stringify(data));
    } else {
        xhr.send();
    }
}

function get_request(path, data, callback) {
    send_request(path, data, "GET", callback);
}

function post_request(path, data, callback) {
    send_request(path, data, "POST", callback);
}

function list_projects() {
    get_request("/", null, (data) => {
        console.log(JSON.stringify(data));
    })
}

document.addEventListener("DOMContentLoaded", () => {
})
