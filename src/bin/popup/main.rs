#![recursion_limit="256"]

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;

use salty_bet_bot::{records_get_all, records_insert_many, records_delete_all, deserialize_records, serialize_records, Loading, MAX_MATCH_TIME_LIMIT};
use algorithm::record::Record;
use stdweb::web::{document, INode};
use stdweb::unstable::TryInto;


fn on_click<F>(id: &str, f: F) where F: FnMut() + 'static {
    js! { @(no_return)
        document.getElementById(@{id}).addEventListener("click", function () {
            @{f}();
        }, true);
    }
}

fn on_import_file<S, P, D>(id: &str, on_start: S, on_progress: P, on_done: D)
    where S: FnMut() + 'static,
          P: FnMut(u32, u32) + 'static,
          D: FnMut(String) + 'static {
    js! { @(no_return)
        var on_start = @{on_start};
        var on_progress = @{on_progress};
        var on_done = @{on_done};

        document.getElementById(@{id}).addEventListener("change", function (event) {
            var files = event.target.files;

            if (files.length === 1) {
                var reader = new FileReader();

                reader.onprogress = function (e) {
                    on_progress(e.loaded, e.total);
                };

                // TODO handle errors
                reader.onload = function (e) {
                    on_done(e.target.result);
                };

                on_start();

                reader.readAsText(files[0]);
            }
        }, true);
    }
}

fn click(id: &str) {
    js! { @(no_return)
        document.getElementById(@{id}).click();
    }
}

fn confirm(message: &str) -> bool {
    js!( return confirm(@{message}); ).try_into().unwrap()
}

fn current_date() -> String {
    js!( return new Date().toISOString().replace(new RegExp("\\:", "g"), "_"); ).try_into().unwrap()
}

fn download(filename: &str, contents: &str) {
    js! { @(no_return)
        var blob = new Blob([@{contents}], { type: "application/json" });
        var url = URL.createObjectURL(blob);

        // TODO error handling
        chrome.downloads.download({
            url: url,
            filename: @{filename},
            saveAs: true,
            conflictAction: "prompt"
        }, function () {
            URL.revokeObjectURL(url);
        });
    }
}

fn open_tab(url: &str) {
    js! { @(no_return)
        // TODO error handling
        chrome.tabs.create({
            url: chrome.runtime.getURL(@{url})
        });
    }
}

fn main() {
    stdweb::initialize();

    let loading = Loading::new();

    loading.hide();

    document().body().unwrap().append_child(loading.element());

    {
        let on_start = {
            let loading = loading.clone();

            move || {
                log!("Loading file");
                loading.show();
            }
        };

        let on_progress = |loaded, total| {
            log!("Loaded {} / {}", loaded, total);
        };

        let on_loaded = {
            let loading = loading.clone();

            move |new_records: String| {
                log!("File loaded, deserializing records");

                let mut new_records = deserialize_records(&new_records);

                log!("{} records deserialized, now retrieving current records", new_records.len());

                let loading = loading.clone();

                records_get_all(move |mut old_records| {
                    log!("{} records retrieved, now sorting", old_records.len());

                    old_records.sort_by(Record::sort_date);
                    new_records.sort_by(Record::sort_date);

                    log!("Sorting complete, now merging");

                    let mut added_records = vec![];

                    // TODO this can be implemented more efficiently (linear rather than quadratic)
                    for new_record in new_records {
                        let start_date = new_record.date - MAX_MATCH_TIME_LIMIT;
                        let end_date = new_record.date + MAX_MATCH_TIME_LIMIT;

                        let index = match old_records.binary_search_by(|x| x.date.partial_cmp(&start_date).unwrap()) {
                            Ok(a) => a,
                            Err(a) => a,
                        };

                        let mut found = false;

                        for old_record in &old_records[index..] {
                            if old_record.date > end_date {
                                break;
                            }

                            // TODO is the bet_amount reliable enough to be used ?
                            if old_record.left.name        == new_record.left.name &&
                               old_record.left.win_streak  == new_record.left.win_streak &&
                               old_record.right.name       == new_record.right.name &&
                               old_record.right.win_streak == new_record.right.win_streak &&
                               old_record.winner           == new_record.winner &&
                               old_record.tier             == new_record.tier &&
                               old_record.mode             == new_record.mode {
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            added_records.push(new_record);
                        }
                    }

                    log!("Merging complete, inserting new records");

                    let len = added_records.len();

                    records_insert_many(&added_records, move || {
                        loading.hide();

                        log!("Inserted {} new records", len);
                    });
                });
            }
        };

        on_import_file("import-input", on_start, on_progress, on_loaded);
    }

    on_click("import-button", move || {
        click("import-input");
    });

    {
        let loading = loading.clone();

        on_click("export-button", move || {
            log!("Getting records");

            loading.show();

            let loading = loading.clone();

            records_get_all(move |records| {
                log!("Got records, now serializing");

                let records = serialize_records(records);

                log!("Serialization complete, now downloading");

                download(&format!("SaltyBet Records ({}).json", current_date()), &records);

                loading.hide();

                log!("Download complete");
            });
        });
    }

    {
        let loading = loading.clone();

        on_click("delete-button", move || {
            if confirm("This will PERMANENTLY delete ALL of the match records!\n\nYou should export the match records before doing this.\n\nAre you sure that you want to delete the match records?") {
                log!("Deleting match records");

                loading.show();

                let loading = loading.clone();

                records_delete_all(move || {
                    loading.hide();
                    log!("Match records deleted");
                });
            }
        });
    }

    on_click("open-chart", || {
        open_tab("chart.html");
    });

    on_click("open-records", || {
        open_tab("records.html");
    });

    stdweb::event_loop();
}
