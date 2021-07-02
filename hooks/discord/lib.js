/** Target unread-bell named pipe */
const IPC_FIFO = "/tmp/unread-bell-discord.pipe";

const { createWriteStream } = require("fs");

let pipeWriteStream;

function init() {
    if (pipeWriteStream !== undefined && !pipeWriteStream.destroyed) {
        return true;
    }

    try {
        pipeWriteStream = createWriteStream(IPC_FIFO);
        pipeWriteStream.on("error", function (e) {
            console.error("[unread-bell] An error occured while sending a packet!", e);
        });
    } catch (e) {
        if (pipeWriteStream !== undefined) {
            pipeWriteStream = undefined;
        }
        console.error("[unread-bell] Unable to initialize a connection to '" + IPC_FIFO + "'!", e);
        return false;
    }
    return true;
}

function sendPacket(packet) {
    if (pipeWriteStream === undefined) {
        console.error("[unread-bell] Cannot send packet because the writestream is not initialized.");
        return;
    }
    
    if (!init()) {
        return;
    }

    if (typeof packet !== "string" && (typeof packet !== "object" || packet.type === undefined)) {
        console.warn("[unread-bell] Cannot send this invalid packet.", packet);
        return;
    }
    if (typeof packet === "object") {
        packet = b64(packet);
    }

    pipeWriteStream.write(packet + "\n");
}

function b64(packet) {
    return btoa(JSON.stringify(packet));
}

module.exports = { sendPacket, init, b64 };
