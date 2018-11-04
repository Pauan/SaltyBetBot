#![recursion_limit="256"]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;
#[macro_use]
extern crate dominator;

use salty_bet_bot::{records_get_all, records_insert_many, records_delete_all, deserialize_records, serialize_records, Loading, MAX_MATCH_TIME_LIMIT};
use algorithm::record::Record;
use dominator::events::{ClickEvent, ChangeEvent};
use stdweb::{Reference, Once};
use stdweb::web::{document, INode};
use stdweb::unstable::TryInto;


// TODO return Option<File>
fn get_file(event: &ChangeEvent) -> Option<Reference> {
    js!(
        var files = @{event}.target.files;

        if (files.length === 1) {
            return files[0];

        } else {
            return null;
        }
    ).try_into().unwrap()
}

// TODO accept File
fn read_file<P, D>(file: Reference, on_progress: P, on_done: D)
    where P: FnMut(u32, u32) + 'static,
          D: FnOnce(String) + 'static {
    js! { @(no_return)
        var on_progress = @{on_progress};
        var on_done = @{Once(on_done)};

        var reader = new FileReader();

        reader.onprogress = function (e) {
            on_progress(e.loaded, e.total);
        };

        // TODO handle errors
        reader.onload = function (e) {
            on_progress.drop();
            on_done(e.target.result);
        };

        reader.readAsText(@{file});
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

    stylesheet!("*", {
        .style("box-sizing", "border-box")
        .style("margin", "0px")
        .style("padding", "0px")
        .style("font-size", "13px")
    });

    stylesheet!("html, body", {
        .style("width", "100%")
        .style("height", "100%")
    });

    stylesheet!("body", {
        .style("min-width", "500px")
        .style("min-height", "300px")
    });

    lazy_static! {
        static ref CLASS_ROOT: String = class! {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("align-items", "center")
            .style("justify-content", "center")
            .style("padding", "15px")
            .style("width", "100%")
            .style("height", "100%")
        };

        static ref CLASS_HR: String = class! {
            .style("width", "100%")
            .style("border", "none")
            .style("border-top", "1px solid gainsboro")
            .style("margin", "10px")
        };

        static ref CLASS_ROW: String = class! {
            .style("display", "flex")
            .style("align-items", "center")
        };

        static ref CLASS_BUTTON: String = class! {
            .style("padding", "5px 10px")
            .style("margin", "5px")
            .style("border-radius", "3px")
            .style("border", "1px solid gainsboro")
            .style("background-color", "hsl(0, 0%, 99%)")
            .style("box-shadow", "1px 1px 1px hsla(0, 0%, 0%, 0.25)")
        };

        static ref CLASS_DELETE_BUTTON: String = class! {
            .style("background-color", "hsl(0, 100%, 90%)")
            .style("font-weight", "bold")
            .style("margin-left", "10px")
        };
    }

    dominator::append_dom(&dominator::body(),
        html!("div", {
            .class(&*CLASS_ROOT)
            .children(&mut [
                html!("h1", {
                    .text("Match records")
                }),
                html!("div", {
                    .class(&*CLASS_ROW)
                    .children(&mut [
                        html!("input", {
                            .attribute("id", "import-input")
                            .attribute("type", "file")
                            .style("display", "none")
                            .event(clone!(loading => move |e: ChangeEvent| {
                                if let Some(file) = get_file(&e) {
                                    log!("Loading file");

                                    loading.show();

                                    let on_progress = |loaded, total| {
                                        log!("Loaded {} / {}", loaded, total);
                                    };

                                    let on_loaded = clone!(loading => move |new_records: String| {
                                        log!("File loaded, deserializing records");

                                        let mut new_records = deserialize_records(&new_records);

                                        log!("{} records deserialized, now retrieving current records", new_records.len());

                                        records_get_all(clone!(loading => move |mut old_records| {
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

                                                    // TODO are the bet_amount, illuminati_bettors, and normal_bettors reliable enough to be used ?
                                                    // TODO compare left and right directly, rather than using the fields ?
                                                    // TODO move this into Record ?
                                                    if old_record.left.name                == new_record.left.name &&
                                                       old_record.left.bet_amount          == new_record.left.bet_amount &&
                                                       old_record.left.win_streak          == new_record.left.win_streak &&
                                                       old_record.left.illuminati_bettors  == new_record.left.illuminati_bettors &&
                                                       old_record.left.normal_bettors      == new_record.left.normal_bettors &&
                                                       old_record.right.name               == new_record.right.name &&
                                                       old_record.right.bet_amount         == new_record.right.bet_amount &&
                                                       old_record.right.win_streak         == new_record.right.win_streak &&
                                                       old_record.right.illuminati_bettors == new_record.right.illuminati_bettors &&
                                                       old_record.right.normal_bettors     == new_record.right.normal_bettors &&
                                                       old_record.winner                   == new_record.winner &&
                                                       old_record.tier                     == new_record.tier &&
                                                       old_record.mode                     == new_record.mode {
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
                                        }));
                                    });

                                    read_file(file, on_progress, on_loaded);
                                }
                            }))
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .text("Import")
                            .event(|_: ClickEvent| {
                                // TODO gross
                                click("import-input");
                            })
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .text("Export")
                            .event(clone!(loading => move |_: ClickEvent| {
                                log!("Getting records");

                                loading.show();

                                records_get_all(clone!(loading => move |records| {
                                    log!("Got records, now serializing");

                                    let records = serialize_records(records);

                                    log!("Serialization complete, now downloading");

                                    download(&format!("SaltyBet Records ({}).json", current_date()), &records);

                                    loading.hide();

                                    log!("Download complete");
                                }));
                            }))
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .class(&*CLASS_DELETE_BUTTON)
                            .text("DELETE")
                            .event(clone!(loading => move |_: ClickEvent| {
                                if confirm("This will PERMANENTLY delete ALL of the match records!\n\nYou should export the match records before doing this.\n\nAre you sure that you want to delete the match records?") {
                                    log!("Deleting match records");

                                    loading.show();

                                    records_delete_all(clone!(loading => move || {
                                        loading.hide();
                                        log!("Match records deleted");
                                    }));
                                }
                            }))
                        }),
                    ])
                }),
                html!("hr", {
                    .class(&*CLASS_HR)
                }),
                html!("h1", {
                    .text("Statistics")
                }),
                html!("div", {
                    .class(&*CLASS_ROW)
                    .children(&mut [
                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .text("Open chart page")
                            .event(|_: ClickEvent| {
                                open_tab("chart.html");
                            })
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .text("Open records page")
                            .event(|_: ClickEvent| {
                                open_tab("records.html");
                            })
                        }),
                    ])
                }),
            ])
        })
    );

    stdweb::event_loop();
}
