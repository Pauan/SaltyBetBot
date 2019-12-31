use algorithm::record::{deserialize_records, serialize_records};
use salty_bet_bot::{spawn, records_get_all, records_insert, records_delete_all, get_added_records, Loading, log, time, DOCUMENT, WINDOW, read_file, ReadProgress};
use dominator::{stylesheet, events, class, html, clone, with_node};
use lazy_static::lazy_static;
use js_sys::Promise;
use web_sys::{HtmlInputElement, HtmlElement, Blob, File};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;


#[wasm_bindgen(inline_js = "
    export function open_tab(url) {
        // TODO error handling
        chrome.tabs.create({
            url: chrome.runtime.getURL(url)
        });
    }

    export function current_date() {
        return new Date().toISOString().replace(new RegExp(\"\\\\:\", \"g\"), \"_\");
    }

    export function download(filename, blob) {
        return new Promise(function (resolve, reject) {
            var url = URL.createObjectURL(blob);

            // TODO error handling
            chrome.downloads.download({
                url: url,
                filename: filename,
                saveAs: true,
                conflictAction: \"prompt\"
            }, function () {
                URL.revokeObjectURL(url);
                resolve();
            });
        });
    }

    export function str_to_blob(contents) {
        return new Blob([contents], { type: \"application/json\" });
    }
")]
extern "C" {
    // TODO return Promise
    fn open_tab(url: &str);

    fn current_date() -> String;

    fn download(filename: &str, blob: &Blob) -> Promise;

    fn str_to_blob(contents: &str) -> Blob;
}


fn get_file(node: &HtmlInputElement) -> Option<File> {
    let files = node.files().unwrap_throw();

    if files.length() == 1 {
        Some(files.get(0).unwrap_throw())

    } else {
        None
    }
}


fn click(id: &str) {
    DOCUMENT.with(|document| {
        document.get_element_by_id(id)
            .unwrap_throw()
            .dyn_into::<HtmlElement>()
            .unwrap_throw()
            .click()
    })
}

fn confirm(message: &str) -> bool {
    WINDOW.with(|window| {
        window.confirm_with_message(message).unwrap_throw()
    })
}


#[wasm_bindgen(start)]
pub fn main_js() {
    console_error_panic_hook::set_once();


    let loading = Loading::new();

    loading.hide();

    dominator::append_dom(&dominator::body(), loading.render());

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
                        html!("input" => HtmlInputElement, {
                            .attribute("id", "import-input")
                            .attribute("type", "file")
                            .style("display", "none")
                            .with_node!(element => {
                                .event(clone!(loading => move |_: events::Change| {
                                    spawn(clone!(element, loading => async move {
                                        if let Some(file) = get_file(&element) {
                                            // If we don't reset the value then the button will stop working after 1 click
                                            element.set_value("");

                                            log!("Starting import");

                                            loading.show();

                                            let on_progress = |progress: ReadProgress| {
                                                log!("Loaded {} / {}", progress.loaded, progress.total);
                                            };

                                            let new_records = time!("Loading file", { read_file(&file, on_progress).await? });

                                            //time!("Deserializing JSON.parse", { js!( return JSON.parse(@{&new_records}); ) });

                                            let new_records = time!("Deserializing records", { deserialize_records(&new_records) });

                                            log!("{} records deserialized", new_records.len());

                                            let old_records = time!("Retrieving current records", { records_get_all().await? });

                                            log!("{} records retrieved", old_records.len());

                                            let added_records = time!("Merging records", { get_added_records(old_records, new_records) });

                                            let len = added_records.len();

                                            time!("Inserting new records", { records_insert(added_records).await? });

                                            loading.hide();

                                            log!("Inserted {} new records", len);
                                        }

                                        Ok(())
                                    }));
                                }))
                            })
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .text("Import")
                            .event(|_: events::Click| {
                                // TODO gross
                                click("import-input");
                            })
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .text("Export")
                            .event(clone!(loading => move |_: events::Click| {
                                spawn(clone!(loading => async move {
                                    log!("Starting export");

                                    loading.show();

                                    let records = time!("Getting records", { records_get_all().await? });

                                    let records = time!("Serializing records", { serialize_records(&records) });

                                    let blob = time!("Converting into Blob", { str_to_blob(&records) });

                                    time!("Downloading", {
                                        let value = JsFuture::from(download(&format!("SaltyBet Records ({}).json", current_date()), &blob)).await?;
                                        assert!(value.is_undefined());
                                    });

                                    loading.hide();

                                    Ok(())
                                }));
                            }))
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .class(&*CLASS_DELETE_BUTTON)
                            .text("DELETE")
                            .event(clone!(loading => move |_: events::Click| {
                                if confirm("This will PERMANENTLY delete ALL of the match records!\n\nYou should export the match records before doing this.\n\nAre you sure that you want to delete the match records?") {
                                    spawn(clone!(loading => async move {
                                        log!("Starting deletion");

                                        loading.show();

                                        time!("Deleting all records", { records_delete_all().await? });

                                        loading.hide();

                                        Ok(())
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
                            .event(|_: events::Click| {
                                open_tab("chart.html");
                            })
                        }),

                        html!("button", {
                            .class(&*CLASS_BUTTON)
                            .text("Open records page")
                            .event(|_: events::Click| {
                                open_tab("records.html");
                            })
                        }),
                    ])
                }),
            ])
        })
    );
}
