const { readFileSync } = require("fs");
const path = require("path");

module.exports = function (dirname) {
    let wsUrl;
    try {
        wsUrl = readFileSync(path.join(dirname, "websocket.url"), "utf-8");
    } catch (_) {
    }

    return {
        wsUrl,
    }
};
