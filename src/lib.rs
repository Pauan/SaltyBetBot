#![feature(is_sorted, unsize)]

pub mod regexp;
mod macros;

use core::marker::Unsize;
use std::cmp::Ordering;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::rc::Rc;
use std::cell::Cell;
use std::future::Future;
use serde::Serialize;
use serde_derive::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use discard::{Discard, DiscardOnDrop};
use algorithm::record::{Tier, Mode, Winner, Record};
use futures_core::stream::Stream;
use futures_util::stream::StreamExt;
use futures_channel::mpsc::{UnboundedReceiver, unbounded};
use futures_signals::signal::Mutable;
use dominator::{Dom, html};
use lazy_static::lazy_static;
use gloo_timers::callback::Timeout;
use wasm_bindgen::{JsValue, JsCast};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use js_sys::{Error, Promise, Date, Function};
use web_sys::{window, Window, Document, Node, Element, HtmlElement, HtmlInputElement, NodeList};


#[wasm_bindgen(inline_js = "
    export function send_message_raw(message) {
        return new Promise(function (resolve, reject) {
            chrome.runtime.sendMessage(null, message, null, function (x) {
                if (chrome.runtime.lastError != null) {
                    reject(new Error(chrome.runtime.lastError.message));

                } else {
                    resolve(x);
                }
            });
        });
    }

    export function chrome_on_message() {
        return chrome.runtime.onMessage;
    }

    export function chrome_port_connect(name) {
        return chrome.runtime.connect(null, { name: name });
    }

    export function get_extension_url(url) {
        return chrome.runtime.getURL(url);
    }

    // TODO add to js_sys
    export function format_float(f) {
        return f.toLocaleString(\"en-US\", {
            style: \"currency\",
            currency: \"USD\",
            minimumFractionDigits: 0
        });
    }

    // TODO add to js_sys
    export function decimal(f) {
        return f.toLocaleString(\"en-US\", {
            style: \"decimal\",
            maximumFractionDigits: 2
        });
    }
")]
extern "C" {
    fn send_message_raw(message: &str) -> Promise;

    fn chrome_on_message() -> Event;

    fn chrome_port_connect(name: &str) -> RawPort;

    pub fn get_extension_url(url: &str) -> String;

    fn format_float(f: f64) -> String;
    pub fn decimal(f: f64) -> String;
}


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
    ReloadPage,
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
        // TODO replace all \u{a0} with spaces ?
        static ref PARSE_NEWLINES: regexp::RegExp = regexp::RegExp::new(r"(?:^[ \u{a0}\n\r]+)|(?:[\n\r]+)|(?:[ \u{a0}\n\r]+$)");
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
            Timeout::new(100, move || wait_until_defined(get, done)).forget();
        },
    }
}


pub fn get_text_content(node: &Node) -> Option<String> {
    node.text_content()
        .map(|x| remove_newlines(&x))
        .map(|x| collapse_whitespace(&x))
}


pub fn to_input_element(node: Element) -> Option<HtmlInputElement> {
    // TODO better error handling
    node.dyn_into().ok()
}

pub fn get_value(node: &HtmlInputElement) -> String {
    let value = node.value();
    let value = remove_newlines(&value);
    collapse_whitespace(&value)
}


thread_local! {
    pub static WINDOW: Window = window().unwrap_throw();
    pub static DOCUMENT: Document = WINDOW.with(|x| x.document().unwrap_throw());
}


pub fn click(node: &HtmlElement) {
    node.click();
}


pub fn query(input: &str) -> Option<Element> {
    DOCUMENT.with(|x| x.query_selector(input).unwrap_throw())
}

pub fn query_all(input: &str) -> NodeList {
    DOCUMENT.with(|x| x.query_selector_all(input).unwrap_throw())
}


pub fn spawn<A>(future: A) where A: Future<Output = Result<(), JsValue>> + 'static {
    spawn_local(async move {
        // TODO replace with a wasm-bindgen-futures API
        if let Err(value) = future.await {
            wasm_bindgen::throw_val(value);
        }
    })
}


pub fn send_message<A, B>(message: &A) -> impl Future<Output = Result<B, JsValue>>
    where A: Serialize,
          B: DeserializeOwned {

    let message: String = serde_json::to_string(message).unwrap_throw();

    async move {
        let reply: String = JsFuture::from(send_message_raw(&message)).await?.as_string().unwrap_throw();

        Ok(serde_json::from_str(&reply).unwrap_throw())
    }
}


pub fn send_message_result<A, B>(message: &A) -> impl Future<Output = Result<B, JsValue>>
    where A: Serialize,
          Result<B, String>: DeserializeOwned {

    let future = send_message(message);

    async move {
        let reply: Result<B, String> = future.await?;

        // TODO don't convert to JsValue
        reply.map_err(|e| Error::new(&e).into())
    }
}


pub fn on_message<A, B, F>(mut f: F) -> DiscardOnDrop<Listener<dyn FnMut(String, JsValue, Function) -> bool>>
    where A: DeserializeOwned + 'static,
          B: Future<Output = String> + 'static,
          F: FnMut(A) -> B + 'static {

    let (sender, receiver) = unbounded::<(String, Function)>();

    spawn_local(async move {
        receiver.for_each(move |(message, reply)| {
            let message = serde_json::from_str(&message).unwrap_throw();

            let future = f(message);

            async move {
                let result = future.await;

                // TODO make this more efficient ?
                match reply.call1(&JsValue::UNDEFINED, &JsValue::from(result)) {
                    Ok(value) => {
                        assert!(value.is_undefined());
                    },
                    Err(e) => {
                        let e: Error = e.dyn_into().unwrap_throw();

                        // TODO incredibly hacky, but needed because Chrome is stupid and gives errors that cannot be avoided
                        if e.message() != "Attempting to use a disconnected port object" {
                            wasm_bindgen::throw_val(e.into());
                        }
                    },
                }
            }
        }).await;
    });

    Listener::new(chrome_on_message(), move |message: String, _sender: JsValue, reply: Function| -> bool {
        sender.unbounded_send((message, reply)).unwrap_throw();
        // TODO somehow only return true when needed ?
        true
    })
}


#[inline]
pub fn serialize_result<A>(value: Result<A, JsValue>) -> Result<A, String> {
    value.map_err(|err| {
        web_sys::console::error_1(&err);

        err.dyn_into::<Error>()
            .unwrap_throw()
            .message()
            .into()
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
        serde_json::to_string(&$value).unwrap_throw()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    RecordsNew,
    RecordsSlice(u32, u32, u32),
    RecordsDrop(u32),

    InsertRecords(Vec<Record>),
    DeleteAllRecords,
    ServerLog(String),
}

const CHUNK_SIZE: u32 = 10000;

pub async fn records_get_all() -> Result<Vec<Record>, JsValue> {
    let mut records = vec![];

    let mut index = 0;

    let id: u32 = send_message_result(&Message::RecordsNew).await?;

    loop {
        let chunk: Option<Vec<Record>> = send_message(&Message::RecordsSlice(id, index, index + CHUNK_SIZE)).await?;

        if let Some(mut chunk) = chunk {
            records.append(&mut chunk);
            index += CHUNK_SIZE;

        } else {
            break;
        }
    }

    send_message(&Message::RecordsDrop(id)).await?;

    Ok(records)
}

pub async fn records_insert(records: Vec<Record>) -> Result<(), JsValue> {
    // TODO more idiomatic check
    if records.len() > 0 {
        for chunk in records.chunks(CHUNK_SIZE as usize) {
            // TODO can this be made more efficient ?
            send_message_result(&Message::InsertRecords(chunk.into_iter().cloned().collect())).await?;
        }
    }

    Ok(())
}

pub async fn records_delete_all() -> Result<(), JsValue> {
    send_message_result(&Message::DeleteAllRecords).await
}

pub fn server_log(message: String) {
    spawn(send_message(&Message::ServerLog(message)))
}


pub fn find_first_index<A, F>(slice: &[A], mut f: F) -> usize where F: FnMut(&A) -> Ordering {
    slice.binary_search_by(|value| {
        match f(value) {
            Ordering::Equal => Ordering::Greater,
            a => a,
        }
    }).unwrap_err()
}

pub fn find_last_index<A, F>(slice: &[A], mut f: F) -> usize where F: FnMut(&A) -> Ordering {
    slice.binary_search_by(|value| {
        match f(value) {
            Ordering::Equal => Ordering::Less,
            a => a,
        }
    }).unwrap_err()
}


pub fn sorted_record_index(old_records: &[Record], new_record: &Record) -> Result<(), usize> {
    let start_date = new_record.date - MAX_MATCH_TIME_LIMIT;
    let end_date = new_record.date + MAX_MATCH_TIME_LIMIT;

    let index = find_first_index(&old_records, |x| x.date.partial_cmp(&start_date).unwrap_throw());

    let mut found = false;

    for old_record in &old_records[index..] {
        assert!(old_record.date >= start_date);

        if old_record.date <= end_date {
            if old_record.is_duplicate(&new_record) {
                found = true;
                break;
            }

        } else {
            break;
        }
    }

    if found {
        // TODO return the index of the duplicate ?
        Ok(())

    } else {
        let new_index = find_last_index(&old_records, |x| Record::sort_date(x, &new_record));
        Err(new_index)
    }
}


pub fn get_added_records(mut old_records: Vec<Record>, new_records: Vec<Record>) -> Vec<Record> {
    assert!(old_records.is_sorted_by(|x, y| Some(Record::sort_date(x, y))));

    let mut added_records = vec![];

    // TODO this can be implemented more efficiently (linear rather than quadratic)
    for new_record in new_records {
        if let Err(index) = sorted_record_index(&old_records, &new_record) {
            old_records.insert(index, new_record.clone());
            added_records.push(new_record);
        }
    }

    added_records
}


#[inline]
pub fn performance_now() -> f64 {
    WINDOW.with(|x| x.performance().unwrap_throw().now())
}


#[inline]
pub fn current_date_pretty() -> String {
    Date::new_0().to_utc_string().into()
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


/*pub struct Debouncer {
    value: Value,
}

impl Debouncer {
    pub fn new<F>(time: u32, f: F) -> Self where F: FnOnce() + 'static {
        Self {
            value: js!(
                var done = false;
                var callback = @{Once(f)};
                var timer;

                function reset() {
                    if (!done) {
                        clearTimeout(timer);

                        timer = setTimeout(function () {
                            done = true;
                            callback();
                        }, @{time});
                    }
                }

                function drop() {
                    done = true;
                    clearTimeout(timer);
                    callback.drop();
                }

                reset();

                return {
                    reset: reset,
                    drop: drop
                };
            )
        }
    }

    pub fn reset(&self) {
        js! { @(no_return)
            @{&self.value}.reset();
        }
    }
}

impl Drop for Debouncer {
    fn drop(&mut self) {
        js! { @(no_return)
            @{&self.value}.drop();
        }
    }
}*/


pub fn reload_page() {
    WINDOW.with(|x| x.location().reload().unwrap_throw())
}


#[wasm_bindgen]
extern "C" {
    pub type Tab;
}


/*impl Tab {
    // TODO is i32 correct ?
    #[inline]
    pub fn id(&self) -> i32 {
        js!( return @{self}.id; ).try_into().unwrap_throw()
    }
}*/


#[wasm_bindgen]
extern "C" {
    #[derive(Debug)]
    pub type Event;

    #[wasm_bindgen(method, js_name = addListener)]
    pub fn add_listener(this: &Event, callback: &Function);

    #[wasm_bindgen(method, js_name = removeListener)]
    pub fn remove_listener(this: &Event, callback: &Function);
}


#[derive(Debug)]
pub struct Listener<A> where A: ?Sized {
    event: Event,
    closure: ManuallyDrop<Closure<A>>,
}

impl<A> Listener<A> where A: ?Sized {
    fn new_raw(event: Event, closure: Closure<A>) -> DiscardOnDrop<Self> {
        event.add_listener(closure.as_ref().unchecked_ref());

        DiscardOnDrop::new(Self {
            event,
            closure: ManuallyDrop::new(closure),
        })
    }
}

impl<A> Listener<A> where A: ?Sized + wasm_bindgen::closure::WasmClosure {
    pub fn new<F>(event: Event, f: F) -> DiscardOnDrop<Self> where F: Unsize<A> + 'static {
        let closure = Closure::new(f);
        Self::new_raw(event, closure)
    }

    /*pub fn unchecked_once(event: Event, f: A) -> DiscardOnDrop<Self> {
        Self::new_raw(event, Closure::once(f))
    }*/
}

impl<A> Discard for Listener<A> where A: ?Sized {
    fn discard(self) {
        let closure = ManuallyDrop::into_inner(self.closure);
        self.event.remove_listener(closure.as_ref().unchecked_ref());
    }
}


#[wasm_bindgen]
extern "C" {
    type Sender;
}


#[wasm_bindgen]
extern "C" {
    #[derive(Debug)]
    type RawPort;

    #[wasm_bindgen(method, js_name = postMessage)]
    fn post_message(this: &RawPort, message: &str);

    #[wasm_bindgen(method, getter)]
    fn name(this: &RawPort) -> String;

    #[wasm_bindgen(method, getter)]
    fn sender(this: &RawPort) -> Option<Sender>;

    #[wasm_bindgen(method)]
    fn disconnect(this: &RawPort);

    #[wasm_bindgen(method, getter, js_name = onMessage)]
    fn on_message(this: &RawPort) -> Event;

    #[wasm_bindgen(method, getter, js_name = onDisconnect)]
    fn on_disconnect(this: &RawPort) -> Event;
}


#[derive(Debug)]
struct PortState {
    port: RawPort,
    // TODO figure out a way to get rid of this second Rc
    disconnected: Rc<Cell<bool>>,
    listener: DiscardOnDrop<Listener<dyn FnMut()>>,
}

impl PortState {
    // TODO trigger existing onDisconnect listeners
    fn disconnect(&self) {
        self.port.disconnect();
        self.disconnected.set(true);
    }
}

impl Drop for PortState {
    fn drop(&mut self) {
        // TODO is this a good idea ?
        self.disconnect();
    }
}


pub struct PortOnMessage {
    _on_disconnect: Option<DiscardOnDrop<Listener<dyn FnMut()>>>,
}


#[derive(Clone, Debug)]
pub struct Port {
    state: Rc<PortState>,
}

impl Port {
    fn new(port: RawPort) -> Self {
        let disconnected = Rc::new(Cell::new(false));

        let listener = {
            let disconnected = disconnected.clone();

            Listener::new(port.on_disconnect(), move || {
                disconnected.set(true);
            })
        };

        Self {
            state: Rc::new(PortState {
                port,
                disconnected,
                listener,
            }),
        }
    }

    #[inline]
    pub fn disconnect(&self) {
        self.state.disconnect();
    }

    // TODO lazy initialization ?
    // TODO verify that dropping/cleanup/disconnect is handled correctly
    pub fn messages<A>(&self) -> impl Stream<Item = A> where A: DeserializeOwned + 'static {
        struct PortMessages<A> {
            receiver: UnboundedReceiver<A>,
            _listener: PortOnMessage,
        }

        impl<A> Stream for PortMessages<A> {
            type Item = A;

            #[inline]
            fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
                self.receiver.poll_next_unpin(cx)
            }
        }

        let (sender, receiver) = unbounded();

        PortMessages {
            receiver,
            _listener: self.on_message(move |message| {
                sender.unbounded_send(message).unwrap_throw();
            }),
        }
    }

    #[inline]
    pub fn on_message<A, F>(&self, mut f: F) -> PortOnMessage
        where A: DeserializeOwned,
              F: FnMut(A) + 'static {

        if self.state.disconnected.get() {
            PortOnMessage {
                _on_disconnect: None,
            }

        } else {
            // TODO improve/check all of this
            let on_message = Listener::new::<dyn FnMut(String, JsValue)>(self.state.port.on_message(), move |message: String, _port: JsValue| {
                f(serde_json::from_str(&message).unwrap_throw());
            });

            PortOnMessage {
                _on_disconnect: Some(Listener::new_raw(self.state.port.on_disconnect(), Closure::once(move || {
                    drop(on_message);
                }))),
            }
        }

        /*Listener::new(js!(
            var self = @{self};
            var callback = @{f};

            function stop() {
                self.port.onMessage.removeListener(callback);
                self.port.onDisconnect.removeListener(stop);
                callback.drop();
            }

            // TODO error checking
            self.port.onMessage.addListener(callback);
            // TODO reconnect when it is disconnected ?
            self.port.onDisconnect.addListener(stop);
            return stop;
        ))*/
    }

    /*#[inline]
    pub fn on_disconnect<A>(&self, f: A) -> DiscardOnDrop<Listener>
        where A: FnOnce() + 'static {

        Listener::new(js!(
            var self = @{self};
            var callback = @{Once(f)};

            function onDisconnect() {
                callback();
            }

            if (self.disconnected) {
                onDisconnect();
                return function () {};

            } else {
                // TODO error checking
                self.port.onDisconnect.addListener(onDisconnect);

                return function () {
                    self.port.onDisconnect.removeListener(onDisconnect);
                    callback.drop();
                };
            }
        ))
    }*/

    // TODO handle errors ?
    // TODO return whether the message was sent or not ?
    #[inline]
    fn send_message_raw(&self, message: &str) {
        if !self.state.disconnected.get() {
            self.state.port.post_message(message);
        }
    }

    #[inline]
    pub fn send_message<A>(&self, message: &A) where A: Serialize {
        self.send_message_raw(&serde_json::to_string(message).unwrap_throw());
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerPort(Port);

impl ServerPort {
    // TODO maybe return Option<String> ?
    #[inline]
    pub fn name(&self) -> String {
        self.0.state.port.name()
    }

    // TODO make new MessageSender type ?
    #[inline]
    pub fn tab(&self) -> Option<Tab> {
        self.0.state.port.sender().tab()
    }

    /*#[inline]
    pub fn on_connect<F>(mut f: F) -> DiscardOnDrop<Listener> where F: FnMut(Self) + 'static {
        let f = move |port: Value| {
            f(ServerPort(Port::new(port)));
        };

        Listener::new(js!(
            var callback = @{f};

            chrome.runtime.onConnect.addListener(callback);

            return function () {
                chrome.runtime.onConnect.removeListener(callback);
                callback.drop();
            };
        ))
    }*/
}

impl std::ops::Deref for ServerPort {
    type Target = Port;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientPort(Port);

impl ClientPort {
    // TODO return DiscardOnDrop<Self> which calls self.disconnect()
    #[inline]
    pub fn connect(name: &str) -> Self {
        // TODO error checking
        ClientPort(Port::new(chrome_port_connect(name)))
    }
}

impl std::ops::Deref for ClientPort {
    type Target = Port;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


pub fn round_to_hour(date: f64) -> f64 {
    let date = Date::new(&JsValue::from(date));
    date.set_utc_minutes(0);
    date.set_utc_seconds(0);
    date.set_utc_milliseconds(0);
    date.get_time()
}

pub fn subtract_days(date: f64, days: u32) -> f64 {
    let date = Date::new(&JsValue::from(date));
    date.set_utc_date(date.get_utc_date() - days);
    date.get_time()
}

pub fn add_days(date: f64, days: u32) -> f64 {
    let date = Date::new(&JsValue::from(date));
    date.set_utc_date(date.get_utc_date() + days);
    date.get_time()
}


pub fn percentage(p: f64) -> String {
    // Rounds to 2 digits
    // https://stackoverflow.com/a/28656825/449477
    format!("{:.2}%", p * 100.0)
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
    visible: Mutable<bool>,
}

impl Loading {
    pub fn new() -> Self {
        Self {
            visible: Mutable::new(true),
        }
    }

    pub fn render(&self) -> Dom {
        html!("div", {
            .style_signal("display", self.visible.signal_ref(|visible| {
                if *visible {
                    "flex"

                } else {
                    "none"
                }
            }))

            .style("cursor", "default")
            .style("position", "fixed")
            .style("left", "0px")
            .style("top", "0px")
            .style("width", "100%")
            .style("height", "100%")
            .style("z-index", "2147483647") // Highest Z-index
            .style("background-color", "hsla(0, 0%, 0%, 0.50)")
            .style("color", "white")
            .style("font-weight", "bold")
            .style("font-size", "30px")
            .style("letter-spacing", "15px")
            .style("text-shadow", "2px 2px 10px black, 0px 0px 5px black")
            .style("flex-direction", "row")
            .style("align-items", "center")
            .style("justify-content", "center")

            .text("LOADING")
        })
    }

    pub fn show(&self) {
        self.visible.set_neq(true);
    }

    pub fn hide(&self) {
        self.visible.set_neq(false);
    }
}
