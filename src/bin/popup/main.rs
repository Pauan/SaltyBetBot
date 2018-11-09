#![recursion_limit="256"]
#![feature(async_await, await_macro, futures_api)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate salty_bet_bot;
#[macro_use]
extern crate dominator;

use salty_bet_bot::{records_get_all, records_insert, records_delete_all, deserialize_records, serialize_records, get_added_records, Loading, set_panic_hook};
use dominator::events::{ClickEvent, ChangeEvent};
use stdweb::{Reference, PromiseFuture, spawn_local, unwrap_future};
use stdweb::web::{document, INode, Blob};
use stdweb::web::error::Error;
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
fn read_file<P>(file: Reference, on_progress: P) -> PromiseFuture<String> where P: FnMut(u32, u32) + 'static {
    js!(
        return new Promise(function (resolve, reject) {
            var on_progress = @{on_progress};

            var reader = new FileReader();

            reader.onprogress = function (e) {
                on_progress(e.loaded, e.total);
            };

            // TODO handle errors
            reader.onload = function (e) {
                on_progress.drop();
                resolve(e.target.result);
            };

            reader.readAsText(@{file});
        });
    ).try_into().unwrap()
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

fn str_to_blob(contents: &str) -> Blob {
    js!( return new Blob([@{contents}], { type: "application/json" }); ).try_into().unwrap()
}

fn download(filename: &str, blob: &Blob) -> PromiseFuture<()> {
    js!(
        return new Promise(function (resolve, reject) {
            var url = URL.createObjectURL(@{blob});

            // TODO error handling
            chrome.downloads.download({
                url: url,
                filename: @{filename},
                saveAs: true,
                conflictAction: "prompt"
            }, function () {
                URL.revokeObjectURL(url);
                resolve();
            });
        });
    ).try_into().unwrap()
}

// TODO return PromiseFuture<()>
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

    set_panic_hook();

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
                                async fn future(loading: Loading, e: ChangeEvent) -> Result<(), Error> {
                                    if let Some(file) = get_file(&e) {
                                        log!("Starting import");

                                        loading.show();

                                        let on_progress = |loaded, total| {
                                            log!("Loaded {} / {}", loaded, total);
                                        };

                                        let new_records = time!("Loading file", { await!(read_file(file, on_progress))? });

                                        let new_records = time!("Deserializing records", { deserialize_records(&new_records) });

                                        log!("{} records deserialized", new_records.len());

                                        let old_records = time!("Retrieving current records", { await!(records_get_all())? });

                                        log!("{} records retrieved", old_records.len());

                                        let added_records = time!("Merging records", { get_added_records(old_records, new_records) });

                                        let len = added_records.len();

                                        time!("Inserting new records", { await!(records_insert(added_records))? });

                                        loading.hide();

                                        log!("Inserted {} new records", len);
                                    }

                                    Ok(())
                                }

                                spawn_local(unwrap_future(future(loading.clone(), e)));
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
                                async fn future(loading: Loading) -> Result<(), Error> {
                                    log!("Starting export");

                                    loading.show();

                                    let records = time!("Getting records", { await!(records_get_all())? });

                                    let records = time!("Serializing records", { serialize_records(records) });

                                    let blob = time!("Converting into Blob", { str_to_blob(&records) });

                                    time!("Downloading", {
                                        await!(download(&format!("SaltyBet Records ({}).json", current_date()), &blob))?;
                                    });

                                    loading.hide();

                                    Ok(())
                                }

                                spawn_local(unwrap_future(future(loading.clone())));
                            }))
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .class(&*CLASS_DELETE_BUTTON)
                            .text("DELETE")
                            .event(clone!(loading => move |_: ClickEvent| {
                                if confirm("This will PERMANENTLY delete ALL of the match records!\n\nYou should export the match records before doing this.\n\nAre you sure that you want to delete the match records?") {
                                    async fn future(loading: Loading) -> Result<(), Error> {
                                        log!("Starting deletion");

                                        loading.show();

                                        time!("Deleting all records", { await!(records_delete_all())? });

                                        loading.hide();

                                        Ok(())
                                    }

                                    spawn_local(unwrap_future(future(loading.clone())));
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
