#![recursion_limit="256"]
#![feature(try_blocks)]

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate stdweb_derive;
#[macro_use]
extern crate salty_bet_bot;
#[macro_use]
extern crate dominator;

use algorithm::record::{Record, deserialize_records};
use discard::DiscardOnDrop;
use std::rc::Rc;
use std::cell::RefCell;
use std::future::Future;
use salty_bet_bot::{set_panic_hook, spawn, sorted_record_index, get_added_records, Message, Tab, Port, Listener, on_message, WaifuMessage};
use stdweb::{PromiseFuture, Reference, Once};
use stdweb::web::error::Error;
use stdweb::unstable::TryInto;
use futures_util::try_future::{try_join, try_join_all};


// TODO cancellation
fn fetch(url: &str) -> PromiseFuture<String> {
    js!(
        // TODO cache ?
        // TODO integrity ?
        return fetch(chrome.runtime.getURL(@{url}), {
            credentials: "same-origin",
            mode: "same-origin"
        // TODO check HTTP status codes ?
        }).then(function (response) {
            return response.text();
        });
    ).try_into().unwrap()
}

/*fn get_twitch_tabs() -> PromiseFuture<Vec<Tab>> {
    js!(
        return new Promise(function (resolve, reject) {
            chrome.tabs.query({
                url: "https://www.twitch.tv/embed/saltybet/chat?darkpopout"
            }, function (tabs) {
                if (chrome.runtime.lastError != null) {
                    reject(new Error(chrome.runtime.lastError.message));

                } else {
                    resolve(tabs);
                }
            });
        });
    ).try_into().unwrap()
}*/

fn remove_tabs(tabs: &[Tab]) -> PromiseFuture<()> {
    js!(
        // TODO move this into Rust ?
        var ids = @{tabs}.map(function (tab) { return tab.id; });

        return new Promise(function (resolve, reject) {
            chrome.tabs.remove(ids, function () {
                if (chrome.runtime.lastError != null) {
                    reject(new Error(chrome.runtime.lastError.message));

                } else {
                    resolve();
                }
            });
        });
    ).try_into().unwrap()
}

/*async fn remove_twitch_tabs() -> Result<(), Error> {
    let tabs = await!(get_twitch_tabs())?;

    if tabs.len() > 0 {
        await!(remove_tabs(&tabs))?;
    }

    Ok(())
}*/


#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "IDBCursorWithValue")]
pub struct Cursor(Reference);

impl Cursor {
    pub fn value(&self) -> String {
        js!( return @{self}.value; ).try_into().unwrap()
    }

    pub fn delete(&self) {
        js! { @(no_return)
            @{self}.delete();
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "IDBDatabase")]
pub struct Db(Reference);

impl Db {
    // TODO this should actually be u64
    pub fn new<F>(version: u32, upgrade_needed: F) -> PromiseFuture<Self> where F: FnOnce(Db, u32, Option<u32>) + 'static {
        js!(
            var upgrade_needed = @{Once(upgrade_needed)};

            return new Promise(function (resolve, reject) {
                var request = indexedDB.open("", @{version});

                request.onupgradeneeded = function (event) {
                    // TODO test this with oldVersion and newVersion
                    upgrade_needed(event.target.result, event.oldVersion, event.newVersion);
                };

                request.onsuccess = function (event) {
                    upgrade_needed.drop();
                    resolve(event.target.result);
                };

                request.onblocked = function () {
                    upgrade_needed.drop();
                    reject(new Error("Database is blocked"));
                };

                request.onerror = function (event) {
                    upgrade_needed.drop();
                    // TODO is this correct ?
                    reject(event);
                };
            });
        ).try_into().unwrap()
    }

    pub fn migrate(&self) {
        js! { @(no_return)
            @{self}.createObjectStore("records", { autoIncrement: true });
        }
    }

    pub fn for_each<F>(&self, f: F) -> PromiseFuture<()> where F: FnMut(Cursor) + 'static {
        js!(
            return new Promise(function (resolve, reject) {
                var callback = @{f};

                var transaction = @{self}.transaction("records", "readwrite");

                transaction.oncomplete = function () {
                    callback.drop();
                    resolve();
                };

                transaction.onerror = function (event) {
                    callback.drop();
                    // TODO is this correct ?
                    reject(event);
                };

                var request = transaction.objectStore("records").openCursor();

                request.onsuccess = function (event) {
                    var cursor = event.target.result;

                    if (cursor) {
                        callback(cursor);
                        cursor.continue();
                    }
                };
            });
        ).try_into().unwrap()
    }

    fn get_all_records_raw(&self) -> PromiseFuture<Vec<String>> {
        js!(
            return new Promise(function (resolve, reject) {
                var request = @{self}.transaction("records", "readonly").objectStore("records").getAll();

                request.onsuccess = function (event) {
                    resolve(event.target.result);
                };

                request.onerror = function (event) {
                    // TODO is this correct ?
                    reject(event);
                };
            });
        ).try_into().unwrap()
    }

    pub async fn get_all_records(&self) -> Result<Vec<Record>, Error> {
        let records = self.get_all_records_raw().await?;
        let mut records: Vec<Record> = records.into_iter().map(|x| Record::deserialize(&x)).collect();
        records.sort_by(Record::sort_date);
        Ok(records)
    }

    fn insert_records_raw(&self, records: Vec<String>) -> PromiseFuture<()> {
        js!(
            return new Promise(function (resolve, reject) {
                var transaction = @{self}.transaction("records", "readwrite");

                transaction.oncomplete = function () {
                    resolve();
                };

                transaction.onerror = function (event) {
                    // TODO is this correct ?
                    reject(event);
                };

                var store = transaction.objectStore("records");

                @{records}.forEach(function (value) {
                    store.add(value);
                });
            });
        ).try_into().unwrap()
    }

    // TODO is this '_ correct ?
    pub fn insert_records(&self, records: &[Record]) -> impl Future<Output = Result<(), Error>> + '_ {
        // TODO avoid doing anything if the len is 0 ?
        let records: Vec<String> = records.into_iter().map(Record::serialize).collect();

        async move {
            if records.len() > 0 {
                self.insert_records_raw(records).await?;
            }

            Ok(())
        }
    }

    pub fn delete_all_records(&self) -> PromiseFuture<()> {
        js!(
            return new Promise(function (resolve, reject) {
                var transaction = @{self}.transaction("records", "readwrite");

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
        ).try_into().unwrap()
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


async fn get_static_records(files: &[&'static str]) -> Result<Vec<Record>, Error> {
    let records = time!("Retrieving default records", {
        let mut output = vec![];

        let files = files.into_iter().map(|file| fetch(file));

        for file in try_join_all(files).await? {
            let mut records = deserialize_records(&file);
            output.append(&mut records);
        }

        output
    });

    log!("Retrieved {} default records", records.len());

    Ok(records)
}


async fn delete_duplicate_records(db: &Db) -> Result<Vec<Record>, Error> {
    let state = time!("Deleting duplicate records", {
        struct State {
            records: Vec<Record>,
            deleted: usize,
        }

        let state = Rc::new(RefCell::new(State {
            records: vec![],
            deleted: 0,
        }));

        db.for_each(clone!(state => move |cursor| {
            let mut state = state.borrow_mut();

            let record = Record::deserialize(&cursor.value());

            match sorted_record_index(&state.records, &record) {
                Ok(_) => {
                    state.deleted += 1;
                    cursor.delete();
                },
                Err(index) => {
                    state.records.insert(index, record);
                },
            }
        })).await?;

        match Rc::try_unwrap(state) {
            Ok(state) => state.into_inner(),
            Err(_) => unreachable!(),
        }
    });

    log!("Deleted {} records", state.deleted);
    log!("Retrieved {} current records", state.records.len());

    Ok(state.records)
}


async fn main_future() -> Result<(), Error> {
    set_panic_hook();

    log!("Initializing...");

    let db = time!("Initializing database", {
        Db::new(2, |db, _old, _new| {
            db.migrate();
        }).await?
    });

    {
        let old_records = delete_duplicate_records(&db);

        let new_records = get_static_records(&[
            "records/SaltyBet Records 0.json",
            "records/SaltyBet Records 1.json",
            "records/SaltyBet Records 2.json",
            "records/SaltyBet Records 3.json",
        ]);

        let (old_records, new_records) = try_join(old_records, new_records).await?;

        let added_records = time!("Inserting default records", {
            let added_records = get_added_records(old_records, new_records);
            db.insert_records(added_records.as_slice()).await?;
            added_records
        });

        log!("Inserted {} records", added_records.len());
    }

    // This is necessary because Chrome doesn't allow content scripts to use the tabs API
    DiscardOnDrop::leak(on_message(move |message| {
        clone!(db => async move {
            match message {
                Message::RecordsNew => reply_result!({
                    let records = db.get_all_records().await?;
                    Remote::new(records)
                }),
                Message::RecordsSlice(id, from, to) => Remote::with(id, |records: &mut Vec<Record>| {
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
                Message::RecordsDrop(id) => reply!({
                    Remote::drop::<Vec<Record>>(id);
                }),

                Message::InsertRecords(records) => reply_result!({
                    db.insert_records(records.as_slice()).await?;
                }),
                Message::DeleteAllRecords => reply_result!({
                    db.delete_all_records().await?;
                }),
                Message::ServerLog(message) => reply!({
                    console!(log, message);
                }),
            }
        })
    }));


    struct SaltyBet {
        port: Port,
        _on_disconnect: DiscardOnDrop<Listener>,
    }

    impl Drop for SaltyBet {
        #[inline]
        fn drop(&mut self) {
            self.port.disconnect();
        }
    }


    struct TwitchChat {
        port: Port,
        _on_message: DiscardOnDrop<Listener>,
        _on_disconnect: DiscardOnDrop<Listener>,
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
    DiscardOnDrop::leak(Port::on_connect(move |port| {
        match port.name().as_str() {
            "saltybet" => spawn(clone!(state => async move {
                {
                    let mut lock = state.borrow_mut();

                    let tabs: Vec<Tab> = lock.salty_bet_ports.drain(..).map(|x| x.port.tab().unwrap()).collect();

                    let future = async move {
                        if tabs.len() > 0 {
                            remove_tabs(&tabs).await?;
                        }

                        Ok(())
                    };

                    let on_disconnect = port.on_disconnect(clone!(state => move |port| {
                        let mut lock = state.borrow_mut();

                        assert!(remove_value(&mut lock.salty_bet_ports, |x| x.port == port));
                    }));

                    lock.salty_bet_ports.push(SaltyBet {
                        port,
                        _on_disconnect: on_disconnect,
                    });

                    future
                }.await
            })),

            "twitch_chat" => spawn(clone!(state => async move {
                let mut lock = state.borrow_mut();

                lock.twitch_chat_ports.clear();

                let on_message = port.on_message(clone!(state => move |message: Vec<WaifuMessage>| {
                    let lock = state.borrow();

                    assert!(lock.salty_bet_ports.len() <= 1);
                    assert_eq!(lock.twitch_chat_ports.len(), 1);

                    for x in lock.salty_bet_ports.iter() {
                        x.port.send_message(&message);
                    }
                }));

                let on_disconnect = port.on_disconnect(clone!(state => move |port| {
                    let mut lock = state.borrow_mut();

                    assert!(remove_value(&mut lock.twitch_chat_ports, |x| x.port == port));
                }));

                lock.twitch_chat_ports.push(TwitchChat {
                    port,
                    _on_message: on_message,
                    _on_disconnect: on_disconnect,
                });

                Ok(())
            })),

            name => {
                panic!("Invalid port name: {}", name);
            },
        }
    }));

    log!("Background page started");

    Ok(())
}

fn main() {
    stdweb::initialize();

    spawn(main_future());

    stdweb::event_loop();
}
