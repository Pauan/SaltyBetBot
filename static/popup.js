function download(filename, contents) {
    var blob = new Blob([contents], { type: "application/json" });
    var url = URL.createObjectURL(blob);

    var a = document.createElement("a");
    a.download = filename;
    a.href = url;
    a.click();

    URL.revokeObjectURL(url);
}

document.getElementById("export-button").addEventListener("click", function () {
    chrome.storage.local.get("matches", function (obj) {
        download("SaltyBet Records (" + new Date().toISOString() + ").json", obj.matches);
    });
}, true);
