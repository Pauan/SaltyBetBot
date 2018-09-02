function download(filename, contents) {
    var blob = new Blob([contents], { type: "application/json" });
    var url = URL.createObjectURL(blob);

    chrome.downloads.download({
        url: url,
        filename: filename,
        saveAs: true,
        conflictAction: "prompt"
    }, function () {
        URL.revokeObjectURL(url);
    });
}

// TODO replace this with Rust code
document.getElementById("export-button").addEventListener("click", function () {
    const request = indexedDB.open("", 1);

    request.onsuccess = function (event) {
        const db = event.target.result;

        db.transaction("records", "readonly").objectStore("records").getAll().onsuccess = function (event) {
            const json = "[" + event.target.result.join(",") + "]";
            console.log(json);
            download("SaltyBet Records (" + new Date().toISOString().replace(/\:/g, "_") + ").json", json);
        };
    };
}, true);

document.getElementById("open-chart").addEventListener("click", function () {
    // TODO error handling
    chrome.tabs.create({
        url: chrome.runtime.getURL("chart.html")
    });
}, true);

document.getElementById("open-records").addEventListener("click", function () {
    // TODO error handling
    chrome.tabs.create({
        url: chrome.runtime.getURL("records.html")
    });
}, true);
