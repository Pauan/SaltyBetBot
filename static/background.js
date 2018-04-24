var salty_ports = [];
var twitch_ports = [];

function noop() {}

function remove_twitch_tabs(f) {
    // TODO handle error messages
    chrome.tabs.query({
        url: "https://www.twitch.tv/embed/saltybet/chat?darkpopout"
    }, function (tabs) {
        if (tabs.length) {
            var mapped = tabs.map(function (tab) { return tab.id; });

            // TODO handle error messages
            chrome.tabs.remove(mapped, function () {
                f();
            });

        } else {
            f();
        }
    });
}

function remove_saltybet(port) {
    var index = salty_ports.indexOf(port);

    if (index !== -1) {
        salty_ports.splice(index, 1);

        if (salty_ports.length === 0) {
            remove_twitch_tabs(noop);
        }
    }
}

function remove_twitch_chat(port) {
    var index = twitch_ports.indexOf(port);

    if (index !== -1) {
        twitch_ports.splice(index, 1);
    }
}

function send_saltybet(message) {
    salty_ports.forEach(function (port) {
        port.postMessage(message);
    });
}

function send_twitch_chat(message) {
    twitch_ports.forEach(function (port) {
        port.postMessage(message);
    });
}

// This is necessary because Chrome doesn't allow content scripts to use the tabs API
chrome.runtime.onMessage.addListener(function (message, _sender, reply) {
    if (twitch_ports.length === 0) {
        var pending = 2;

        function done() {
            --pending;

            if (pending === 0) {
                reply({});
            }
        }

        remove_twitch_tabs(done);

        // TODO error checking
        chrome.tabs.create({
            url: "https://www.twitch.tv/embed/saltybet/chat?darkpopout",
            active: false
        }, function (tab) {
            done();
        });

        return true;

    } else {
        reply({});
    }
});

// This is necessary because Chrome doesn't allow content scripts to directly communicate with other content scripts
// TODO auto-reload the tabs if they haven't sent a message in a while
chrome.runtime.onConnect.addListener(function (port) {
    if (port.name === "saltybet") {
        if (salty_ports.length > 0) {
            // TODO handle error messages
            chrome.tabs.remove(port.sender.tab.id);

        } else {
            salty_ports.push(port);

            // TODO error checking
            port.onMessage.addListener(send_twitch_chat);

            // TODO error checking
            port.onDisconnect.addListener(remove_saltybet);
        }

    } else if (port.name === "twitch_chat") {
        if (twitch_ports.length > 0) {
            // TODO handle error messages
            chrome.tabs.remove(port.sender.tab.id);

        } else {
            twitch_ports.push(port);

            // TODO error checking
            port.onMessage.addListener(send_saltybet);

            // TODO error checking
            port.onDisconnect.addListener(remove_twitch_chat);
        }

    } else {
        throw new Error("Invalid port: " + port.name);
    }
});

console.log("Background page started");
