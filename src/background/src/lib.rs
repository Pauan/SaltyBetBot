#![feature(try_blocks)]

use algorithm::record::{Record, deserialize_records};
use discard::DiscardOnDrop;
use std::rc::Rc;
use std::cell::RefCell;
use std::future::Future;
use salty_bet_bot::{api, spawn, Tab, ServerPort, messages, log, time, reply, reply_result, PortOnMessage, PortOnDisconnect, console_log};
use salty_bet_bot::indexeddb::{Db, Write, TableOptions};
use salty_bet_bot::indexeddb;
use futures_util::stream::StreamExt;
use futures_util::future::{try_join, try_join_all};
use futures_signals::signal::{Mutable, SignalExt};
use js_sys::{Promise, Array};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::prelude::*;


#[wasm_bindgen(inline_js = "
    // TODO cancellation
    export function fetch_(url) {
        // TODO cache ?
        // TODO integrity ?
        return fetch(chrome.runtime.getURL(url), {
            credentials: \"same-origin\",
            mode: \"same-origin\"
        // TODO check HTTP status codes ?
        }).then(function (response) {
            return response.text();
        });
    }

    export function remove_tabs(tabs) {
        // TODO move this into Rust ?
        var ids = tabs.map(function (tab) { return tab.id; });

        return new Promise(function (resolve, reject) {
            chrome.tabs.remove(ids, function () {
                if (chrome.runtime.lastError != null) {
                    reject(new Error(chrome.runtime.lastError.message));

                } else {
                    resolve();
                }
            });
        });
    }
")]
extern "C" {
    // TODO replace with gloo
    fn fetch_(url: &str) -> Promise;

    fn remove_tabs(tabs: &Array) -> Promise;
}


/*async fn remove_twitch_tabs() -> Result<(), Error> {
    let tabs = await!(get_twitch_tabs())?;

    if tabs.len() > 0 {
        await!(remove_tabs(&tabs))?;
    }

    Ok(())
}*/


pub fn get_all_records(db: &Db) -> impl Future<Output = Result<Vec<(u32, Record)>, JsValue>> {
    db.read(&["records2"], move |tx| {
        async move {
            let records = tx.get_all("records2").await?;

            let mut records: Vec<(u32, Record)> = records.into_iter()
                .map(|x| {
                    let key = x.key().as_f64().unwrap();

                    // TODO is this check robust ?
                    let key = if key.round() == key {
                        key as u32

                    } else {
                        panic!("Invalid db record key: {:?}", key);
                    };

                    (
                        key,
                        Record::deserialize(&x.value().as_string().unwrap()),
                    )
                })
                .collect();

            records.sort_by(|x, y| Record::sort_date(&x.1, &y.1));

            Ok(records)
        }
    })
}

pub fn remove_records<'a>(db: &'a Db, record_keys: &[u32]) -> impl Future<Output = Result<(), JsValue>> + 'a {
    let keys: Vec<JsValue> = record_keys.into_iter().map(|x| JsValue::from(*x)).collect();

    async move {
        if keys.len() > 0 {
            db.write(&["records2"], move |tx| {
                async move {
                    tx.remove_many("records2", keys);
                    Ok(())
                }
            }).await?;
        }

        Ok(())
    }
}

pub fn delete_all_records(db: &Db) -> impl Future<Output = Result<(), JsValue>> {
    db.write(&["records2"], move |tx| {
        async move {
            tx.clear("records2");
            Ok(())
        }
    })
}

fn convert_records(records: &[Record]) -> Vec<indexeddb::Record> {
    records.into_iter().map(|x| {
        let value = JsValue::from(Record::serialize(x));
        indexeddb::Record::new(None, &value)
    }).collect()
}

pub fn insert_records<'a>(db: &'a Db, records: &[Record]) -> impl Future<Output = Result<(), JsValue>> + 'a {
    // TODO avoid doing anything if the len is 0 ?
    let records = convert_records(records);

    async move {
        if records.len() > 0 {
            db.write(&["records2"], move |tx| {
                async move {
                    tx.insert_many("records2", records);
                    Ok(())
                }
            }).await?;
        }

        Ok(())
    }
}


fn remove_value<A, F>(vec: &mut Vec<A>, f: F) -> bool where F: FnMut(&A) -> bool {
    if let Some(index) = vec.iter().position(f) {
        vec.swap_remove(index);
        true

    } else {
        false
    }
}


pub struct Remote;

// TODO use u32, usize, or isize ?
impl Remote {
    #[inline]
    fn new<A>(value: A) -> u32 {
        Box::into_raw(Box::new(value)) as u32
    }

    #[inline]
    fn with<A, B, F>(pointer: u32, f: F) -> B where F: FnOnce(&mut A) -> B {
        let mut value: Box<A> = unsafe { Box::from_raw(pointer as *mut A) };

        let output = f(&mut value);

        Box::into_raw(value);

        output
    }

    #[inline]
    fn drop<A>(pointer: u32) {
        drop(unsafe { Box::from_raw(pointer as *mut A) });
    }
}


async fn get_static_records(files: &[&'static str]) -> Result<Vec<Record>, JsValue> {
    let records = time!("Retrieving default records", {
        let mut output = vec![];

        let files = files.into_iter().map(|file| JsFuture::from(fetch_(file)));

        for file in try_join_all(files).await? {
            let mut records = deserialize_records(&file.as_string().unwrap());
            output.append(&mut records);
        }

        output
    });

    log!("Retrieved {} default records", records.len());

    Ok(records)
}


fn listen_to_ports() {
    struct SaltyBet {
        port: ServerPort,
        _on_disconnect: DiscardOnDrop<PortOnDisconnect>,
    }

    impl Drop for SaltyBet {
        #[inline]
        fn drop(&mut self) {
            self.port.disconnect();
        }
    }


    struct TwitchChat {
        port: ServerPort,
        _on_message: DiscardOnDrop<PortOnMessage>,
        _on_disconnect: DiscardOnDrop<PortOnDisconnect>,
    }

    impl Drop for TwitchChat {
        #[inline]
        fn drop(&mut self) {
            self.port.disconnect();
        }
    }


    struct State {
        salty_bet_ports: Vec<SaltyBet>,
        twitch_chat_ports: Vec<TwitchChat>,
    }

    let state = Rc::new(RefCell::new(State {
        salty_bet_ports: vec![],
        twitch_chat_ports: vec![],
    }));


    // This is necessary because Chrome doesn't allow content scripts to directly communicate with other content scripts
    // TODO auto-reload the tabs if they haven't sent a message in a while
    DiscardOnDrop::leak(ServerPort::on_connect(move |port| {
        match port.name().as_str() {
            "saltybet" => {
                let mut lock = state.borrow_mut();

                let tabs: Vec<Tab> = lock.salty_bet_ports.drain(..).map(|x| x.port.tab().unwrap()).collect();

                let on_disconnect = port.on_disconnect({
                    let state = state.clone();
                    let port = port.clone();

                    move || {
                        let mut lock = state.borrow_mut();

                        assert!(remove_value(&mut lock.salty_bet_ports, |x| x.port == port));
                    }
                });

                lock.salty_bet_ports.push(SaltyBet {
                    port,
                    _on_disconnect: on_disconnect,
                });

                if tabs.len() > 0 {
                    let fut = JsFuture::from(remove_tabs(&tabs.into_iter().collect()));

                    spawn(async move {
                        let value = fut.await?;
                        assert!(value.is_undefined());
                        Ok(())
                    });
                }
            },

            "twitch_chat" => {
                let mut lock = state.borrow_mut();

                lock.twitch_chat_ports.clear();

                let on_message = {
                    let state = state.clone();

                    port.on_message(move |message: Vec<api::WaifuMessage>| {
                        let lock = state.borrow();

                        assert!(lock.salty_bet_ports.len() <= 1);
                        assert_eq!(lock.twitch_chat_ports.len(), 1);

                        for x in lock.salty_bet_ports.iter() {
                            x.port.send_message(&message);
                        }
                    })
                };

                let on_disconnect = port.on_disconnect({
                    let state = state.clone();
                    let port = port.clone();

                    move || {
                        let mut lock = state.borrow_mut();

                        assert!(remove_value(&mut lock.twitch_chat_ports, |x| x.port == port));
                    }
                });

                lock.twitch_chat_ports.push(TwitchChat {
                    port,
                    _on_message: on_message,
                    _on_disconnect: on_disconnect,
                });
            },

            name => {
                panic!("Invalid port name: {}", name);
            },
        }
    }));
}


#[wasm_bindgen(start)]
pub async fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();


    log!("Initializing...");


    listen_to_ports();


    // This starts listening to messages immediately, but they're only polled later
    let messages = messages::<api::Message>();


    let db = time!("Initializing database", {
        Rc::new(Db::open("", 3, |tx, old, _new| {
            async move {
                tx.create_table("records2", &TableOptions {
                    auto_increment: true,
                });

                if let Some(2) = old {
                    time!("Migrating old records", {
                        struct DeletedInfo {
                            records: Vec<Record>,
                            deleted: usize,
                        }

                        fn delete_duplicate_records_raw(tx: &Write) -> impl Future<Output = Result<DeletedInfo, JsValue>> {
                            tx.fold("records", DeletedInfo {
                                records: vec![],
                                deleted: 0,
                            }, move |mut state, cursor| {
                                let record = Record::deserialize(&cursor.value().as_string().unwrap());

                                match api::sorted_record_index(&state.records, &record) {
                                    Ok(_) => {
                                        state.deleted += 1;
                                        cursor.delete();
                                    },
                                    Err(index) => {
                                        state.records.insert(index, record);
                                    },
                                }

                                state
                            })
                        }

                        let old_records = tx.get_all("records2").await?;

                        assert_eq!(old_records.len(), 0);

                        let info = delete_duplicate_records_raw(&tx).await?;

                        log!("Deleted {} records", info.deleted);

                        let records = convert_records(&info.records);

                        let len = records.len();

                        if len > 0 {
                            tx.insert_many("records2", records);
                        }

                        log!("Migrated {} records", len);

                        tx.delete_table("records");
                    });
                }

                Ok(())
            }
        }).await?)
    });

    let loaded = Mutable::new(false);


    {
        let db = db.clone();
        let loaded = loaded.clone();

        spawn(async move {
            messages.for_each(move |message| {
                // TODO is there a better way of doing this ?
                let done = loaded.signal().wait_for(true);

                let db = db.clone();

                message.with(move |message| {
                    async move {
                        done.await;

                        match message {
                            api::Message::RecordsNew => reply_result!({
                                // TODO this shouldn't pause the message queue
                                let records: Vec<Record> = {
                                    let records = get_all_records(&db).await?;

                                    records.into_iter().map(|x| x.1).collect()
                                };

                                Remote::new(records)
                            }),
                            api::Message::RecordsSlice(id, from, to) => Remote::with(id, |records: &mut Vec<Record>| {
                                let from = from as usize;
                                let to = to as usize;

                                reply!({
                                    let len = records.len();

                                    if from >= len {
                                        None

                                    } else {
                                        Some(&records[from..(to.min(len))])
                                    }
                                })
                            }),
                            api::Message::RecordsDrop(id) => reply!({
                                Remote::drop::<Vec<Record>>(id);
                            }),

                            api::Message::InsertRecords(records) => reply_result!({
                                insert_records(&db, records.as_slice()).await?;
                            }),

                            api::Message::DeleteAllRecords => reply_result!({
                                delete_all_records(&db).await?;
                            }),

                            api::Message::ServerLog(message) => reply!({
                                console_log(message);
                            }),
                        }
                    }
                })
            }).await;

            Ok(())
        });
    }


    {
        let old_records = async {
            let old_records = time!("Retrieving current records", { get_all_records(&db).await? });

            log!("Retrieved {} current records", old_records.len());

            Ok(old_records)
        };

        let new_records = get_static_records(&[
            "records/SaltyBet Records 0.json",
            "records/SaltyBet Records 1.json",
            "records/SaltyBet Records 2.json",
            "records/SaltyBet Records 3.json",
            "records/SaltyBet Records 4.json",
            "records/SaltyBet Records 5.json",
        ]);

        let (old_records, new_records) = try_join(old_records, new_records).await?;

        // TODO make this more efficient somehow ?
        let (old_records, deleted_records) = api::partition_records(old_records);
        let added_records = api::get_added_records(old_records, new_records);

        try_join(
            async {
                if !deleted_records.is_empty() {
                    time!("Deleting duplicate records", {
                        remove_records(&db, &deleted_records).await?;
                    });

                    log!("Deleted {} records", deleted_records.len());
                }

                Ok(()) as Result<(), JsValue>
            },

            async {
                if !added_records.is_empty() {
                    time!("Inserting default records", {
                        insert_records(&db, added_records.as_slice()).await?;
                    });

                    log!("Inserted {} records", added_records.len());
                }

                Ok(()) as Result<(), JsValue>
            },
        ).await?;

        loaded.set(true);
    }


    log!("Background page started");

    Ok(())
}
