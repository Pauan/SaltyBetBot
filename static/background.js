var salty_ports = [];
var twitch_ports = [];
var twitch_tabs = [];

function remove_saltybet(port) {
    var index = salty_ports.indexOf(port);

    if (index === -1) {
        throw new Error("Not found");
    }

    salty_ports.splice(index, 1);

    if (salty_ports.length === 0) {
        // TODO handle error messages
        chrome.tabs.remove(twitch_tabs);
        twitch_tabs.length = 0;
    }
}

function remove_twitch_chat(port) {
    var index = twitch_ports.indexOf(port);

    if (index === -1) {
        throw new Error("Not found");
    }

    twitch_ports.splice(index, 1);
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
    // TODO error checking
    chrome.tabs.create({
        url: "https://www.twitch.tv/saltybet/chat",
        active: false
    }, function (tab) {
        // TODO update the id when the chrome.tabs.onReplaced event fires
        // TODO remove the id when the Twitch tab is closed / disconnected
        twitch_tabs.push(tab.id);
        reply({});
    });

    return true;
});

// This is necessary because Chrome doesn't allow content scripts to directly communicate with other content scripts
chrome.runtime.onConnect.addListener(function (port) {
    if (port.name === "saltybet") {
        salty_ports.push(port);

        // TODO error checking
        port.onMessage.addListener(send_twitch_chat);

        // TODO error checking
        port.onDisconnect.addListener(remove_saltybet);

    } else if (port.name === "twitch_chat") {
        twitch_ports.push(port);

        // TODO error checking
        port.onMessage.addListener(send_saltybet);

        // TODO error checking
        port.onDisconnect.addListener(remove_twitch_chat);

    } else {
        throw new Error("Invalid port: " + port.name);
    }
});

console.log("Background page started");
