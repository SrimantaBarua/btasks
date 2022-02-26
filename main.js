const { app, BrowserWindow } = require("electron");

const createWindow = () => {
    const window = new BrowserWindow({
        width: 1000,
        height: 800,
    });
    window.loadFile("index.html");
};

app.whenReady().then(() => {
    createWindow();
})
