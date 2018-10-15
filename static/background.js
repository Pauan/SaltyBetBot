var salty_ports = [];
var twitch_ports = [];

function remove_saltybet(port) {
    var index = salty_ports.indexOf(port);

    if (index !== -1) {
        salty_ports.splice(index, 1);

        if (salty_ports.length === 0) {
            run_promise(remove_twitch_tabs());
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

function run_promise(promise) {
    promise.catch(function (e) {
        console.error(e);
        throw e;
    });
}

function get_twitch_tabs() {
    return new Promise(function (resolve, reject) {
        chrome.tabs.query({
            url: "https://www.twitch.tv/embed/saltybet/chat?darkpopout"
        }, function (tabs) {
            if (chrome.runtime.lastError != null) {
                reject(chrome.runtime.lastError);

            } else {
                resolve(tabs);
            }
        });
    });
}

function remove_twitch_tabs() {
    return get_twitch_tabs()
        .then(function (tabs) {
            if (tabs.length > 0) {
                return remove_tabs(tabs.map(function (tab) { return tab.id; }));
            }
        });
}

function create_twitch_tab() {
    return new Promise(function (resolve, reject) {
        chrome.tabs.create({
            url: "https://www.twitch.tv/embed/saltybet/chat?darkpopout",
            active: false
        }, function (tab) {
            if (chrome.runtime.lastError != null) {
                reject(chrome.runtime.lastError);

            } else {
                resolve();
            }
        });
    });
}

function remove_tabs(ids) {
    return new Promise(function (resolve, reject) {
        chrome.tabs.remove(ids, function () {
            if (chrome.runtime.lastError != null) {
                reject(chrome.runtime.lastError);

            } else {
                resolve();
            }
        });
    });
}

function get_db(name, version, upgrade_needed) {
    return new Promise(function (resolve, reject) {
        var request = indexedDB.open(name, version);

        request.onupgradeneeded = function (event) {
            // TODO test this with oldVersion and newVersion
            upgrade_needed(event.target.result, event.oldVersion, event.newVersion);
        };

        request.onsuccess = function (event) {
            resolve(event.target.result);
        };

        request.onblocked = function () {
            reject(new Error("Database is blocked"));
        };

        request.onerror = function (event) {
            // TODO is this correct ?
            reject(event);
        };
    });
}

function get_all_records(db) {
    return new Promise(function (resolve, reject) {
        var request = db.transaction("records", "readonly").objectStore("records").getAll();

        request.onsuccess = function (event) {
            resolve(event.target.result);
        };

        request.onerror = function (event) {
            // TODO is this correct ?
            reject(event);
        };
    });
}

function insert_records(db, values) {
    return new Promise(function (resolve, reject) {
        var transaction = db.transaction("records", "readwrite");

        transaction.oncomplete = function () {
            resolve();
        };

        transaction.onerror = function (event) {
            // TODO is this correct ?
            reject(event);
        };

        var store = transaction.objectStore("records");

        values.forEach(function (value) {
            store.add(value);
        });
    });
}

function delete_all_records(db) {
    return new Promise(function (resolve, reject) {
        var transaction = db.transaction("records", "readwrite");

        transaction.oncomplete = function () {
            resolve();
        };

        transaction.onerror = function (event) {
            // TODO is this correct ?
            reject(event);
        };

        var store = transaction.objectStore("records");

        store.clear();
    });
}

run_promise(
    get_db("", 2, function (db) {
        db.createObjectStore("records", { autoIncrement: true });
    })
    .then(function (db) {
        // This is necessary because Chrome doesn't allow content scripts to use the tabs API
        chrome.runtime.onMessage.addListener(function (message, _sender, reply) {
            if (message.type === "records:get-all") {
                run_promise(
                    get_all_records(db)
                        .then(function (records) {
                            reply(records);
                        })
                );

                return true;

            } else if (message.type === "records:insert") {
                run_promise(
                    insert_records(db, message.value)
                        .then(function () {
                            reply(null);
                        })
                );

                return true;

            } else if (message.type === "records:delete-all") {
                run_promise(
                    delete_all_records(db)
                        .then(function () {
                            reply(null);
                        })
                );

                return true;

            } else if (message.type === "tabs:open-twitch-chat") {
                if (twitch_ports.length === 0) {
                    run_promise(
                        remove_twitch_tabs()
                            .then(function () {
                                return create_twitch_tab();
                            })
                            .then(function () {
                                reply(null);
                            })
                    );

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
                    run_promise(remove_tabs([port.sender.tab.id]));

                } else {
                    salty_ports.push(port);

                    // TODO error checking
                    port.onMessage.addListener(send_twitch_chat);

                    // TODO error checking
                    port.onDisconnect.addListener(remove_saltybet);
                }

            } else if (port.name === "twitch_chat") {
                if (twitch_ports.length > 0) {
                    run_promise(remove_tabs([port.sender.tab.id]));

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
    })
);
