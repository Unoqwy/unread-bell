((wsUrl, lib) => {
    /** Websocket URL to connect to if preload does not give one */
    const DEFAULT_WS_URL = "ws://127.0.0.1:3631";
    /** Delay in millis to try to [re]connect to the Websocket once it gets closed */
    const RECONNECT_AFTER = 5_000;

    if (wsUrl === undefined) {
        wsUrl = DEFAULT_WS_URL;
    }

    /* Webpack Injection */

    let webpackModules;
    const injectId = Math.random().toString(36).substring(2);
    const injected = {
        [injectId]: function (_, _, i) {
            const cleanedUp = Object.values(i)
                .filter(val => typeof val === "object" && injectId in val)
                .every(val => delete val[injectId]);
            if (!cleanedUp) {
                console.error("[unread-bell] Could not clean up webpack injection.");
            }
            webpackModules = i.c;
        },
    };
    webpackJsonp.push([[], injected, [[injectId]]]);

    if (webpackModules === undefined || typeof webpackModules !== "object") {
        console.error("[unread-bell] Failed to fetch webpack modules, unread messages won't be reported!");
        return;
    }

    /**
     * Gets a reference to an internal Discord function.
     * @param {string} functionName - The name of the function as stored in its webpack's module
     */
    function getFunction(functionName) {
        const fnMod = Object.values(webpackModules)
            .map(mod => mod.exports)
            .filter(mod => mod !== undefined)
            .map(mod => (typeof mod.default === "object" ? mod.default : mod))
            .find(mod => mod[functionName] !== undefined && typeof mod[functionName] === "function");
        if (fnMod === undefined) {
            console.error("[unread-bell] Could not find function '" + functionName + "'!");
        }
        return fnMod[functionName];
    }

    /* Websocket connection */

    const wsUrlObj = new URL(wsUrl);
    let ws;

    function wsInit() {
        if (ws?.readyState === WebSocket.OPEN) {
            return;
        }

        if (lib !== undefined) {
            lib.checkSocket(wsUrlObj.hostname, wsUrlObj.port)
                .then(() => __wsInit())
                .catch(() => {
                    setTimeout(function () {
                        wsInit();
                    }, RECONNECT_AFTER);
                });
        } else {
            __wsInit();
        }
    }

    function __wsInit() {
        ws = new WebSocket(wsUrl);
        // TODO: optional authentification

        ws.onopen = function () {
            console.log("[unread-bell] Connection to unread-bell daemon was successful.");
            if (lastUpdatePayloadB64 !== undefined) {
                checkNotifications(true);
            }
        };

        let wasError = false;
        ws.onerror = function () {
            return (wasError = true);
        };

        ws.onclose = function () {
            if (!wasError) {
                console.warn(
                    "[unread-bell] Connection to unread-bell daemon was closed. Retrying in " + RECONNECT_AFTER + "ms."
                );
            }
            setTimeout(function () {
                wsInit();
            }, RECONNECT_AFTER);
        };
    }

    function sendPacket(packet) {
        if (ws?.readyState !== WebSocket.OPEN) {
            return;
        }
        ws.send(b64(packet));
    }

    function b64(obj) {
        return btoa(JSON.stringify(obj));
    }

    /* Main */

    const getGuild = getFunction("getGuild");
    const getChannel = getFunction("getChannel");
    const getUnreadPrivateChannelIds = getFunction("getUnreadPrivateChannelIds");
    const getMentionCount = getFunction("getMentionCount");
    const getUnreadGuilds = getFunction("getUnreadGuilds");
    const getGuildUnreadCount = getFunction("getGuildUnreadCount");
    const getMentionCounts = getFunction("getMentionCounts");

    function getNotificationsPayload() {
        const dms = {},
            groups = {},
            guilds = {};
        getUnreadPrivateChannelIds().forEach(privateChannelId => {
            const privateChannel = getChannel(privateChannelId);
            if (privateChannel.type === 1) {
                const recipient = privateChannel.rawRecipients[0];
                if (!recipient) {
                    return;
                }
                dms[recipient.id] = {
                    channelId: privateChannelId,
                    unreadCount: getMentionCount(privateChannelId),
                    lastMessageId: privateChannel.lastMessageId,
                    username: recipient.username,
                    discriminator: recipient.discriminator,
                };
            } else if (privateChannel.type === 3) {
                groups[privateChannelId] = {
                    unreadCount: getMentionCount(privateChannelId),
                    lastMessageId: privateChannel.lastMessageId,
                    name: privateChannel.name,
                    users: privateChannel.recipients,
                };
            }
        });

        const mentionCounts = getMentionCounts();
        Object.keys(getUnreadGuilds()).forEach(guildId => {
            const guild = getGuild(guildId);
            guilds[guildId] = {
                unreadCount: getGuildUnreadCount(guildId),
                mentionCount: mentionCounts[guildId],
                name: guild.name,
            };
        });
        return { dms, groups, guilds };
    }

    let lastUpdatePayloadB64;

    function checkNotifications(revive = false) {
        const payload = getNotificationsPayload();
        const payloadB64 = b64(payload);
        if (payloadB64 !== lastUpdatePayloadB64 || revive) {
            sendPacket({
                type: "Update",
                payload: payload,
                revive: revive,
            });

            lastUpdatePayloadB64 = payloadB64;
        }
    }

    window.UnreadBell = {
        getNotificationsPayload,
        runningIntervals: [],
        _debug: {
            getFunction,
            scope: function () {
                return {
                    wsUrl,
                    ws,
                    wsInit,
                    __wsInit,
                    lastUpdatePayloadB64,
                    checkNotifications,
                    sendPacket,
                };
            },
        },
    };

    wsInit();
    setTimeout(function () {
        checkNotifications();
        window.UnreadBell.runningIntervals.push(setInterval(checkNotifications, 1000));
    }, 2500);
})(window.UnreadBellPreload?.wsUrl, window.UnreadBellPreload?.lib);
