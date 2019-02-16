#![recursion_limit="256"]
#![feature(async_await, await_macro, futures_api, arbitrary_self_types)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate stdweb_derive;
#[macro_use]
extern crate serde_derive;

pub mod regexp;
mod macros;

use core::cmp::Ordering;
use std::pin::Pin;
use std::task::{Poll, LocalWaker};
use std::future::Future;
use serde::Serialize;
use serde::de::DeserializeOwned;
use discard::{Discard, DiscardOnDrop};
use algorithm::record::{Tier, Mode, Winner, Record};
use futures_core::stream::Stream;
use futures_util::stream::StreamExt;
use futures_channel::mpsc::{UnboundedReceiver, unbounded};
use stdweb::{Reference, Value, Once, PromiseFuture, spawn_local, unwrap_future};
use stdweb::web::{document, set_timeout, INode, Element, NodeList};
use stdweb::web::error::Error;
use stdweb::web::html_element::InputElement;
use stdweb::unstable::TryInto;
use stdweb::traits::*;


// 50 minutes
// TODO is this high enough ?
pub const MAX_MATCH_TIME_LIMIT: f64 = 1000.0 * 60.0 * 50.0;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuBetsOpen {
    pub left: String,
    pub right: String,
    pub tier: Tier,
    pub mode: Mode,
    pub date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuBetsClosedInfo {
    pub name: String,
    pub win_streak: f64,
    pub bet_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuBetsClosed {
    pub left: WaifuBetsClosedInfo,
    pub right: WaifuBetsClosedInfo,
    pub date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuWinner {
    pub name: String,
    pub side: Winner,
    pub date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaifuMessage {
    BetsOpen(WaifuBetsOpen),
    BetsClosed(WaifuBetsClosed),
    Winner(WaifuWinner),
    ModeSwitch { date: f64, is_exhibition: bool },
}


// TODO make this more efficient
pub fn parse_f64(input: &str) -> Option<f64> {
    lazy_static! {
        static ref PARSE_F64_REGEX: regexp::RegExp = regexp::RegExp::new(r",");
    }

    match PARSE_F64_REGEX.replace(input, "").parse::<f64>() {
        Ok(a) => Some(a),
        // TODO better error handling
        Err(_) => None,
    }
}


// TODO make this more efficient
pub fn remove_newlines(input: &str) -> String {
    lazy_static! {
        static ref PARSE_NEWLINES: regexp::RegExp = regexp::RegExp::new(r"(?:^[ \n\r]+)|(?:[\n\r]+)|(?:[ \n\r]+$)");
    }

    PARSE_NEWLINES.replace(input, "")
}


// TODO make this more efficient
pub fn collapse_whitespace(input: &str) -> String {
    lazy_static! {
        static ref PARSE_WHITESPACE: regexp::RegExp = regexp::RegExp::new(r" {2,}");
    }

    PARSE_WHITESPACE.replace(input, " ")
}


pub fn parse_name(input: &str) -> Option<String> {
    lazy_static! {
        static ref REGEXP: regexp::RegExp = regexp::RegExp::new(r"^(.+) \[-?[0-9,]+\] #[0-9,]+$");
    }

    REGEXP.first_match(input).and_then(|mut captures| captures[1].take())
}


pub fn parse_money(input: &str) -> Option<f64> {
    lazy_static! {
        static ref MONEY_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^[ \n\r]*\$([0-9,]+)[ \n\r]*$"
        );
    }

    MONEY_REGEX.first_match(input).and_then(|captures|
        captures[1].as_ref().and_then(|x| parse_f64(x)))
}


pub fn wait_until_defined<A, B, C>(mut get: A, done: B)
    where A: FnMut() -> Option<C> + 'static,
          B: FnOnce(C) + 'static {
    match get() {
        Some(a) => done(a),
        None => {
            set_timeout(|| wait_until_defined(get, done), 100);
        },
    }
}


pub fn get_text_content<A: INode>(node: A) -> Option<String> {
    node.text_content()
        .map(|x| remove_newlines(&x))
        .map(|x| collapse_whitespace(&x))
}


pub fn to_input_element(node: Element) -> Option<InputElement> {
    // TODO better error handling
    node.try_into().ok()
}

pub fn get_value(node: &InputElement) -> String {
    let value = node.raw_value();
    let value = remove_newlines(&value);
    collapse_whitespace(&value)
}


// TODO move this into stdweb
pub fn click(node: &InputElement) {
    js! { @(no_return)
        @{node}.click();
    }
}


pub fn query(input: &str) -> Option<Element> {
    document().query_selector(input).unwrap()
}

pub fn query_all(input: &str) -> NodeList {
    document().query_selector_all(input).unwrap()
}


pub fn spawn<A>(future: A) where A: Future<Output = Result<(), Error>> + 'static {
    spawn_local(unwrap_future(future))
}


#[inline]
fn send_message_raw(message: &str) -> PromiseFuture<String> {
    js!(
        return new Promise(function (resolve, reject) {
            chrome.runtime.sendMessage(null, @{message}, null, function (x) {
                if (chrome.runtime.lastError != null) {
                    reject(new Error(chrome.runtime.lastError.message));

                } else {
                    resolve(x);
                }
            });
        });
    ).try_into().unwrap()
}

pub fn send_message<A, B>(message: &A) -> impl Future<Output = Result<B, Error>>
    where A: Serialize,
          B: DeserializeOwned {

    let message: String = serde_json::to_string(message).unwrap();

    async move {
        let reply: String = await!(send_message_raw(&message))?;

        Ok(serde_json::from_str(&reply).unwrap())
    }
}

pub fn send_message_result<A, B>(message: &A) -> impl Future<Output = Result<B, Error>>
    where A: Serialize,
          Result<B, String>: DeserializeOwned {

    let future = send_message(message);

    async move {
        let reply: Result<B, String> = await!(future)?;

        reply.map_err(|e| {
            // TODO replace with stdweb
            js!( return new Error(@{e}); ).try_into().unwrap()
        })
    }
}

pub fn on_message<A, B, F>(mut f: F) -> DiscardOnDrop<Listener>
    where A: DeserializeOwned,
          B: Future<Output = String> + 'static,
          F: FnMut(A) -> B + 'static {

    let callback = move |message: String, reply: Reference| {
        let future = f(serde_json::from_str(&message).unwrap());

        spawn_local(async {
            let result = await!(future);

            // TODO make this more efficient ?
            js! { @(no_return)
                try {
                    @{reply}(@{result});

                } catch (e) {
                    // TODO incredibly hacky, but needed because Chrome is stupid and gives errors that cannot be avoided
                    if (e.message !== "Attempting to use a disconnected port object") {
                        throw e;
                    }
                }
            }
        });
    };

    Listener::new(js!(
        var callback = @{callback};

        function listener(message, _sender, reply) {
            callback(message, reply);
            // TODO somehow only return true when needed ?
            return true;
        }

        // TODO handle errors
        chrome.runtime.onMessage.addListener(listener);

        return function () {
            chrome.runtime.onMessage.removeListener(listener);
            callback.drop();
        };
    ))
}

#[inline]
pub fn serialize_result<A>(value: Result<A, Error>) -> Result<A, String> {
    value.map_err(|err| {
        console!(error, &err);
        // TODO use stdweb method
        js!( return @{err}.message; ).try_into().unwrap()
    })
}

#[macro_export]
macro_rules! reply_result {
    ($value:block) => {{
        $crate::reply!({ $crate::serialize_result(try { $value }) })
    }}
}

#[macro_export]
macro_rules! reply {
    ($value:block) => {
        serde_json::to_string(&$value).unwrap()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    GetAllRecords,
    InsertRecords(Vec<Record>),
    DeleteAllRecords,
    OpenTwitchChat,
    ServerLog(String),
}

pub async fn create_tab() -> Result<(), Error> {
    await!(send_message_result(&Message::OpenTwitchChat))
}

pub async fn records_get_all() -> Result<Vec<Record>, Error> {
    await!(send_message_result(&Message::GetAllRecords))
}

pub async fn records_insert(records: Vec<Record>) -> Result<(), Error> {
    // TODO more idiomatic check
    if records.len() > 0 {
        await!(send_message_result(&Message::InsertRecords(records)))

    } else {
        Ok(())
    }
}

pub async fn records_delete_all() -> Result<(), Error> {
    await!(send_message_result(&Message::DeleteAllRecords))
}

pub fn server_log(message: String) {
    spawn(send_message(&Message::ServerLog(message)))
}


pub fn serialize_records(records: Vec<Record>) -> String {
    serde_json::to_string_pretty(&records).unwrap()
}

pub fn deserialize_records(records: &str) -> Vec<Record> {
    serde_json::from_str(records).unwrap()
}


pub fn find_starting_index<A, F>(slice: &[A], mut f: F) -> usize where F: FnMut(&A) -> Ordering {
    slice.binary_search_by(|value| {
        match f(value) {
            Ordering::Equal => Ordering::Greater,
            a => a,
        }
    }).unwrap_err()
}


pub fn get_added_records(mut old_records: Vec<Record>, mut new_records: Vec<Record>) -> Vec<Record> {
    old_records.sort_by(Record::sort_date);
    new_records.sort_by(Record::sort_date);

    let mut added_records = vec![];

    // TODO this can be implemented more efficiently (linear rather than quadratic)
    for new_record in new_records {
        let start_date = new_record.date - MAX_MATCH_TIME_LIMIT;
        let end_date = new_record.date + MAX_MATCH_TIME_LIMIT;

        let index = find_starting_index(&old_records, |x| x.date.partial_cmp(&start_date).unwrap());

        let mut found = false;

        for old_record in &old_records[index..] {
            assert!(old_record.date >= start_date);

            if old_record.date <= end_date {
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

            } else {
                break;
            }
        }

        if !found {
            added_records.push(new_record);
        }
    }

    added_records
}


pub struct Listener {
    stop: Value,
}

impl Listener {
    #[inline]
    pub fn new(stop: Value) -> DiscardOnDrop<Self> {
        DiscardOnDrop::new(Self { stop })
    }
}

impl Discard for Listener {
    #[inline]
    fn discard(self) {
        js! { @(no_return)
            @{&self.stop}();
        }
    }
}


#[inline]
pub fn performance_now() -> f64 {
    js!( return performance.now(); ).try_into().unwrap()
}


pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(move |info| {
        stdweb::print_error_panic(info.to_string());
    }));
}


#[inline]
pub fn current_date_pretty() -> String {
    js!( return new Date().toISOString(); ).try_into().unwrap()
}


/*pub struct IndexedDBSchema(Value);

impl IndexedDBSchema {
    pub fn create_object_store(&self, name: &str) {
        js! { @(no_return)
            @{&self.0}.createObjectStore(@{name}, { autoIncrement: true });
        }
    }
}


pub struct IndexedDBWrite(Value);

impl IndexedDBWrite {
    // TODO handle errors
    pub fn insert(&self, store: &str, value: &str) {
        js! { @(no_return)
            @{&self.0}.objectStore(@{store}).add(@{value});
        }
    }

    // TODO handle errors
    pub fn get_all<F>(&self, store: &str, f: F) where F: FnOnce(Vec<String>) + 'static {
        js! { @(no_return)
            @{&self.0}.objectStore(@{store}).getAll().onsuccess = function (event) {
                @{Once(f)}(event.target.result);
            };
        }
    }

    // TODO return a listener handle
    pub fn on_complete<F>(&self, f: F) where F: FnOnce() + 'static {
        js! { @(no_return)
            @{&self.0}.addEventListener("complete", function () {
                @{Once(f)}();
            }, true);
        }
    }
}


pub struct IndexedDB(Value);

impl IndexedDB {
    // TODO use promises or futures or whatever
    // TODO handle errors
    pub fn open<M, D>(name: &str, version: u32, make_schema: M, done: D)
        where M: FnOnce(u32, IndexedDBSchema) + 'static,
              D: FnOnce(Self) + 'static {

        let make_schema = move |old: u32, value: Value| make_schema(old, IndexedDBSchema(value));

        let done = move |value: Value| done(IndexedDB(value));

        js! { @(no_return)
            var make_schema = @{Once(make_schema)};
            var request = indexedDB.open(@{name}, @{version});

            request.onupgradeneeded = function (event) {
                make_schema(event.oldVersion, event.target.result);
            };

            request.onsuccess = function (event) {
                make_schema.drop();
                @{Once(done)}(event.target.result);
            };
        }
    }

    pub fn transaction_write(&self, stores: &[&str]) -> IndexedDBWrite {
        IndexedDBWrite(js!( return @{&self.0}.transaction(@{stores}, "readwrite"); ))
    }
}*/


pub fn get_extension_url(url: &str) -> String {
    js!( return chrome.runtime.getURL(@{url}); ).try_into().unwrap()
}


#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "Object")]
pub struct Tab(Reference);

/*impl Tab {
    // TODO is i32 correct ?
    #[inline]
    pub fn id(&self) -> i32 {
        js!( return @{self}.id; ).try_into().unwrap()
    }
}*/


#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "Object")]
pub struct Port(Reference);

// TODO disconnect(&self) but it needs to remove the listeners
impl Port {
    // TODO return DiscardOnDrop<Self> which calls self.disconnect()
    #[inline]
    pub fn connect(name: &str) -> Self {
        // TODO error checking
        js!( return chrome.runtime.connect(null, { name: @{name} }); ).try_into().unwrap()
    }

    #[inline]
    pub fn on_connect<F>(f: F) -> DiscardOnDrop<Listener> where F: FnMut(Self) + 'static {
        Listener::new(js!(
            var callback = @{f};

            chrome.runtime.onConnect.addListener(callback);

            return function () {
                chrome.runtime.onConnect.removeListener(callback);
                callback.drop();
            };
        ))
    }

    // TODO maybe return Option<String> ?
    #[inline]
    pub fn name(&self) -> String {
        js!( return @{self}.name; ).try_into().unwrap()
    }

    // TODO make new MessageSender type ?
    #[inline]
    pub fn tab(&self) -> Option<Tab> {
        js!( return @{self}.sender.tab; ).try_into().unwrap()
    }

    // TODO lazy initialization ?
    // TODO verify that dropping/cleanup/disconnect is handled correctly
    pub fn messages<A>(&self) -> impl Stream<Item = A> where A: DeserializeOwned + 'static {
        struct PortMessages<A> {
            receiver: UnboundedReceiver<A>,
            _listener: DiscardOnDrop<Listener>,
        }

        impl<A> Stream for PortMessages<A> {
            type Item = A;

            #[inline]
            fn poll_next(mut self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Option<Self::Item>> {
                self.receiver.poll_next_unpin(lw)
            }
        }

        let (sender, receiver) = unbounded();

        PortMessages {
            receiver,
            _listener: self.on_message(move |message, _port| {
                sender.unbounded_send(serde_json::from_str(&message).unwrap()).unwrap();
            }),
        }
    }

    #[inline]
    fn on_message<A>(&self, f: A) -> DiscardOnDrop<Listener>
        where A: FnMut(String, Self) + 'static {

        Listener::new(js!(
            var self = @{self};
            var callback = @{f};

            function stop() {
                self.onMessage.removeListener(callback);
                self.onDisconnect.removeListener(stop);
                callback.drop();
            }

            // TODO error checking
            self.onMessage.addListener(callback);
            // TODO reconnect when it is disconnected ?
            // TODO should this use onDisconnect ?
            self.onDisconnect.addListener(stop);

            return stop;
        ))
    }

    #[inline]
    pub fn on_disconnect<A>(&self, f: A) -> DiscardOnDrop<Listener>
        where A: FnOnce(Self) + 'static {

        Listener::new(js!(
            var self = @{self};
            var callback = @{Once(f)};

            // TODO error checking
            self.onDisconnect.addListener(callback);

            return function () {
                self.onDisconnect.removeListener(callback);
                callback.drop();
            };
        ))
    }

    // TODO handle onDisconnect ?
    // TODO handle errors ?
    #[inline]
    fn send_message_raw(&self, message: &str) {
        js! { @(no_return)
            @{self}.postMessage(@{message});
        }
    }

    #[inline]
    pub fn send_message<A>(&self, message: &A) where A: Serialize {
        self.send_message_raw(&serde_json::to_string(message).unwrap());
    }
}


pub fn subtract_days(date: f64, days: u32) -> f64 {
    js!(
        var date = new Date(@{date});
        date.setUTCDate(date.getUTCDate() - @{days});
        return date.getTime();
    ).try_into().unwrap()
}

pub fn add_days(date: f64, days: u32) -> f64 {
    js!(
        var date = new Date(@{date});
        date.setUTCDate(date.getUTCDate() + @{days});
        return date.getTime();
    ).try_into().unwrap()
}


pub fn percentage(p: f64) -> String {
    // Rounds to 2 digits
    // https://stackoverflow.com/a/28656825/449477
    format!("{:.2}%", p * 100.0)
}

fn format_float(f: f64) -> String {
    js!(
        return @{f}.toLocaleString("en-US", {
            style: "currency",
            currency: "USD",
            minimumFractionDigits: 0
        });
    ).try_into().unwrap()
}

pub fn decimal(f: f64) -> String {
    js!(
        return @{f}.toLocaleString("en-US", {
            style: "decimal",
            maximumFractionDigits: 2
        });
    ).try_into().unwrap()
}

pub fn money(m: f64) -> String {
    if m < 0.0 {
        format!("-{}", format_float(-m))

    } else {
        format_float(m)
    }
}

pub fn display_odds(odds: f64) -> String {
    if odds == 1.0 {
        "1 : 1".to_string()

    } else if odds < 1.0 {
        format!("{} : 1", decimal(1.0 / odds))

    } else {
        format!("1 : {}", decimal(odds))
    }
}



#[derive(Debug, Clone)]
pub struct Loading {
    element: Element,
}

impl Loading {
    pub fn new() -> Self {
        let element = document().create_element("div").unwrap();

        js! { @(no_return)
            var node = @{&element};
            node.textContent = "LOADING";
            node.style.cursor = "default";
            node.style.position = "fixed";
            node.style.left = "0px";
            node.style.top = "0px";
            node.style.width = "100%";
            node.style.height = "100%";
            node.style.zIndex = "2147483647"; // Highest Z-index
            node.style.backgroundColor = "hsla(0, 0%, 0%, 0.50)";
            node.style.color = "white";
            node.style.fontWeight = "bold";
            node.style.fontSize = "30px";
            node.style.letterSpacing = "15px";
            node.style.textShadow = "2px 2px 10px black, 0px 0px 5px black";
            node.style.display = "flex";
            node.style.flexDirection = "row";
            node.style.alignItems = "center";
            node.style.justifyContent = "center";
        }

        Self { element }
    }

    pub fn element(&self) -> &Element {
        &self.element
    }

    pub fn show(&self) {
        js! { @(no_return)
            @{&self.element}.style.display = "flex";
        }
    }

    pub fn hide(&self) {
        js! { @(no_return)
            @{&self.element}.style.display = "none";
        }
    }
}
