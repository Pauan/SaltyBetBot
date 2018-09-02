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

function migrate_records(db, done) {
    chrome.storage.local.get("matches", function (items) {
        const value = items["matches"];

        if (value == null) {
            done();

        } else {
            console.log("Migrating old records");

            const records = JSON.parse(value);

            // TODO handle errors
            const transaction = db.transaction("records", "readwrite");

            const store = transaction.objectStore("records");

            records.forEach(function (record) {
                store.add(JSON.stringify(record));
            });

            transaction.oncomplete = function () {
                chrome.storage.local.remove("matches", function () {
                    console.log("Finished migrating old records");
                    done();
                });
            };
        }
    });
}

// TODO handle errors
function get_db(done) {
    const request = indexedDB.open("", 2);

    request.onupgradeneeded = function (event) {
        const db = event.target.result;

        db.createObjectStore("records", { autoIncrement: true });
    };

    request.onsuccess = function (event) {
        done(event.target.result);
    };
}

get_db(function (db) {
    migrate_records(db, function () {
        // This is necessary because Chrome doesn't allow content scripts to use the tabs API
        chrome.runtime.onMessage.addListener(function (message, _sender, reply) {
            if (message.type === "records:get-all") {
                // TODO handle errors
                db.transaction("records", "readonly").objectStore("records").getAll().onsuccess = function (event) {
                    reply(event.target.result);
                };

                return true;

            } else if (message.type === "records:insert") {
                // TODO handle errors
                const transaction = db.transaction("records", "readwrite");

                transaction.objectStore("records").add(message.value);

                transaction.oncomplete = function () {
                    reply(null);
                };

                return true;

            } else if (message.type === "tabs:open-twitch-chat") {
                if (twitch_ports.length === 0) {
                    var pending = 2;

                    function done() {
                        --pending;

                        if (pending === 0) {
                            reply(null);
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
                    reply(null);
                }

            } else {
                throw new Error("Invalid message type: " + message.type);
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
    });
});
