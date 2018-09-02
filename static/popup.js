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

document.getElementById("export-button").addEventListener("click", function () {
    chrome.runtime.sendMessage(null, { type: "records:get-all" }, function (records) {
        const json = JSON.stringify(records.map(JSON.parse));
        download("SaltyBet Records (" + new Date().toISOString().replace(/\:/g, "_") + ").json", json);
    });
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
