(() => {
    /** Time in millis after which a sync packet will be forced sent even though no changes have been detected */
    const SYNC_PACKET_FORCE_AFTER = 120_000;

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
        const fn = Object.values(webpackModules)
            .map(mod => mod.exports)
            .filter(mod => mod !== undefined)
            .map(mod => (typeof mod.default === "object" ? mod.default : mod))
            .find(mod => mod[functionName] !== undefined && typeof mod[functionName] === "function")[functionName];
        if (fn === undefined) {
            console.error("[unread-bell] Could not find function '" + functionName + "'!");
        }
        return fn;
    }

    const getUnreadPrivateChannelIds = getFunction("getUnreadPrivateChannelIds");
    const getUnreadGuilds = getFunction("getUnreadGuilds");
    const getMentionCounts = getFunction("getMentionCounts");

    let lastUpdatePayload, lastSynced;

    function checkNotifications() {
        const payload = {
            privateMessages: getUnreadPrivateChannelIds(),
            unreadGuilds: Object.keys(getUnreadGuilds()),
            unreadGuildMentions: Object.fromEntries(
                Object.entries(getMentionCounts()).filter(([guild, count]) => guild && count > 0)
            ),
        };

        const payloadB64 = UnreadBellLib.b64(payload);
        const now = new Date();

        let forced = false;
        if (payloadB64 !== lastUpdatePayload || (forced = now - lastSynced >= SYNC_PACKET_FORCE_AFTER)) {
            UnreadBellLib.sendPacket({
                type: "UPDATE",
                payload: payload,
                forced: forced,
            });

            lastUpdatePayload = payloadB64;
            lastSynced = now;
        }
    }

    window.UnreadBell = {
        checkNotifications,
        runningIntervals: [],
        _debug: {
            getFunction,
        },
    };

    if (UnreadBellLib.init()) {
        setTimeout(function () {
            checkNotifications();
            window.UnreadBell.runningIntervals.push(setInterval(checkNotifications, 1000));
        }, 2500);
    }
})();
