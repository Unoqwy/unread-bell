const { readFileSync } = require("fs");
const path = require("path");
const net = require("net");

function checkSocket(host, port) {
    return new Promise((resolve, reject) => {
        const socket = new net.Socket();
        socket.on("connect", () => {
            socket.destroy();
            resolve();
        });
        socket.on("error", () => {
            socket.destroy();
            reject();
        });
        socket.connect(port, host);
    });
}

module.exports = function (dirname) {
    let wsUrl;
    try {
        wsUrl = readFileSync(path.join(dirname, "websocket.url"), "utf-8");
    } catch (_) {}

    return {
        wsUrl,
        lib: {
            checkSocket,
        },
    };
};
