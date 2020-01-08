#![feature(is_sorted)]

pub mod indexeddb;
pub mod regexp;
pub mod api;
mod macros;

use std::marker::PhantomData;
use std::cmp::Ordering;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::task::{Poll, Context};
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::future::Future;
use serde::Serialize;
use serde::de::DeserializeOwned;
use discard::{Discard, DiscardOnDrop};
use futures_core::stream::Stream;
use futures_util::stream::StreamExt;
use futures_channel::oneshot;
use futures_channel::mpsc::{UnboundedReceiver, unbounded};
use futures_signals::signal::Mutable;
use dominator::{Dom, html};
use gloo_timers::callback::Timeout;
use wasm_bindgen::{JsValue, JsCast};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use js_sys::{Error, Promise, Date, Function};
use web_sys::{window, Window, Document, Node, Element, HtmlElement, HtmlInputElement, NodeList, FileReader, Blob, ProgressEvent};


#[wasm_bindgen(inline_js = "
    export function send_message_raw(message) {
        return new Promise(function (resolve, reject) {
            chrome.runtime.sendMessage(null, message, null, function (x) {
                var error = chrome.runtime.lastError;

                if (error != null) {
                    reject(new Error(error.message));

                } else {
                    resolve(x);
                }
            });
        });
    }

    export function chrome_on_message() {
        return chrome.runtime.onMessage;
    }

    export function chrome_on_connect() {
        return chrome.runtime.onConnect;
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

    export function set_utc_date(date, days) {
        date.setUTCDate(days);
    }
")]
extern "C" {
    fn send_message_raw(message: &str) -> Promise;

    fn chrome_on_message() -> Event;
    fn chrome_on_connect() -> Event;

    fn chrome_port_connect(name: &str) -> RawPort;

    pub fn get_extension_url(url: &str) -> String;

    fn format_float(f: f64) -> String;

    pub fn decimal(f: f64) -> String;

    fn set_utc_date(date: &Date, days: f64);
}


pub fn poll_receiver<A>(receiver: &mut oneshot::Receiver<A>, cx: &mut Context) -> Poll<A> {
    Pin::new(receiver).poll(cx).map(|x| {
        // TODO better error handling
        match x {
            Ok(x) => x,
            Err(_) => unreachable!(),
        }
    })
}


#[derive(Debug)]
pub struct MultiSender<A> {
    sender: Rc<RefCell<Option<oneshot::Sender<A>>>>,
}

impl<A> MultiSender<A> {
    pub fn new(sender: oneshot::Sender<A>) -> Self {
        Self {
            sender: Rc::new(RefCell::new(Some(sender))),
        }
    }

    pub fn send(&self, value: A) {
        let _ = self.sender.borrow_mut()
            .take()
            .unwrap()
            .send(value);
    }
}

impl<A> Clone for MultiSender<A> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct ReadProgress {
    pub is_size_known: bool,
    pub loaded: u64,
    pub total: u64,
}

struct ReadFile {
    reader: FileReader,
    receiver: oneshot::Receiver<Result<String, JsValue>>,
    _onprogress: Closure<dyn FnMut(&ProgressEvent)>,
    _onabort: Closure<dyn FnMut(&JsValue)>,
    _onerror: Closure<dyn FnMut(&JsValue)>,
    _onload: Closure<dyn FnMut(&JsValue)>,
}

impl Future for ReadFile {
    type Output = Result<String, JsValue>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        poll_receiver(&mut self.receiver, cx)
    }
}

impl Drop for ReadFile {
    // TODO test whether this triggers the abort event or not
    #[inline]
    fn drop(&mut self) {
        self.reader.abort();
    }
}

pub fn read_file<P>(blob: &Blob, mut on_progress: P) -> impl Future<Output = Result<String, JsValue>>
    where P: FnMut(ReadProgress) + 'static {

    let (sender, receiver) = oneshot::channel();

    let sender = MultiSender::new(sender);

    let reader = FileReader::new().unwrap();

    let onprogress = closure!(move |event: &ProgressEvent| {
        on_progress(ReadProgress {
            is_size_known: event.length_computable(),
            // TODO are these conversions safe ?
            loaded: event.loaded() as u64,
            total: event.total() as u64,
        });
    });

    let onabort = {
        let sender = sender.clone();

        Closure::once(move |_event: &JsValue| {
            sender.send(Err(Error::new("read_file was aborted").into()));
        })
    };

    let onerror = {
        let reader = reader.clone();
        let sender = sender.clone();

        Closure::once(move |_event: &JsValue| {
            sender.send(Err(reader.error().unwrap().into()));
        })
    };

    let onload = {
        let reader = reader.clone();

        Closure::once(move |_event: &JsValue| {
            sender.send(Ok(reader.result().unwrap().as_string().unwrap()));
        })
    };

    reader.set_onprogress(Some(onprogress.as_ref().unchecked_ref()));
    reader.set_onabort(Some(onabort.as_ref().unchecked_ref()));
    reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    reader.set_onload(Some(onload.as_ref().unchecked_ref()));

    reader.read_as_text(blob).unwrap();

    ReadFile {
        reader,
        receiver,
        _onprogress: onprogress,
        _onabort: onabort,
        _onerror: onerror,
        _onload: onload,
    }
}


// TODO make this more efficient
pub fn parse_f64(input: &str) -> Option<f64> {
    thread_local! {
        static PARSE_F64_REGEX: regexp::RegExp = regexp::RegExp::new(r",");
    }

    match PARSE_F64_REGEX.with(|re| re.replace(input, "")).parse::<f64>() {
        Ok(a) => Some(a),
        // TODO better error handling
        Err(_) => None,
    }
}


// TODO make this more efficient
pub fn remove_newlines(input: &str) -> String {
    thread_local! {
        // TODO replace all \u{a0} with spaces ?
        static PARSE_NEWLINES: regexp::RegExp = regexp::RegExp::new(r"(?:^[ \u{a0}\n\r]+)|(?:[\n\r]+)|(?:[ \u{a0}\n\r]+$)");
    }

    PARSE_NEWLINES.with(|re| re.replace(input, ""))
}


// TODO make this more efficient
pub fn collapse_whitespace(input: &str) -> String {
    thread_local! {
        static PARSE_WHITESPACE: regexp::RegExp = regexp::RegExp::new(r" {2,}");
    }

    PARSE_WHITESPACE.with(|re| re.replace(input, " "))
}


pub fn parse_name(input: &str) -> Option<String> {
    thread_local! {
        static REGEXP: regexp::RegExp = regexp::RegExp::new(r"^(.+) \[-?[0-9,]+\] #[0-9,]+$");
    }

    REGEXP.with(|re| re.first_match(input)).and_then(|mut captures| captures[1].take())
}


pub fn parse_money(input: &str) -> Option<f64> {
    thread_local! {
        static MONEY_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^[ \n\r]*\$([0-9,]+)[ \n\r]*$"
        );
    }

    MONEY_REGEX.with(|re| re.first_match(input))
        .and_then(|captures|
            captures[1].as_ref()
                .and_then(|x| parse_f64(x)))
}


pub fn wait_until_defined<A, B, C>(mut get: A, done: B)
    where A: FnMut() -> Option<C> + 'static,
          B: FnOnce(C) + 'static {
    match get() {
        Some(a) => done(a),
        None => {
            // TODO does this forget leak memory ?
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
    pub static WINDOW: Window = window().unwrap();
    pub static DOCUMENT: Document = WINDOW.with(|x| x.document().unwrap());
}


pub fn click(node: &HtmlElement) {
    node.click();
}


pub fn query(input: &str) -> Option<Element> {
    DOCUMENT.with(|x| x.query_selector(input).unwrap())
}

pub fn query_all(input: &str) -> NodeList {
    DOCUMENT.with(|x| x.query_selector_all(input).unwrap())
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

    let message: String = serde_json::to_string(message).unwrap();

    // TODO move this inside of the async ?
    let fut = JsFuture::from(send_message_raw(&message));

    async move {
        let reply: String = fut.await?.as_string().unwrap();

        Ok(serde_json::from_str(&reply).unwrap())
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


pub struct Messages<A> {
    // TODO use dyn FnMut(String, &JsValue, Function) -> bool
    _listener: DiscardOnDrop<Listener<dyn FnMut(String, JsValue, Function) -> bool>>,
    receiver: UnboundedReceiver<(String, Function)>,
    _value: PhantomData<A>,
}

impl<A> Unpin for Messages<A> {}

impl<A> Stream for Messages<A> where A: DeserializeOwned {
    type Item = Message<A>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx).map(|option| {
            option.map(|(value, reply)| {
                let value = serde_json::from_str(&value).unwrap();
                Message { value, reply }
            })
        })
    }
}


#[derive(Debug)]
pub struct Message<A> {
    value: A,
    reply: Function,
}

impl<A> Message<A> {
    pub fn with<B, F>(self, f: F) -> impl Future<Output = ()>
        where B: Future<Output = String>,
              F: FnOnce(A) -> B {

        let Message { value, reply } = self;

        let fut = f(value);

        async move {
            let message = fut.await;

            // TODO make this more efficient ?
            match reply.call1(&JsValue::UNDEFINED, &JsValue::from(message)) {
                Ok(value) => {
                    assert!(value.is_undefined());
                },
                Err(e) => {
                    let e: Error = e.dyn_into().unwrap();

                    // TODO incredibly hacky, but needed because Chrome is stupid and gives errors that cannot be avoided
                    if e.message() != "Attempting to use a disconnected port object" {
                        wasm_bindgen::throw_val(e.into());
                    }
                },
            }
        }
    }
}


pub fn messages<A>() -> Messages<A> where A: DeserializeOwned {
    let (sender, receiver) = unbounded::<(String, Function)>();

    Messages {
        _listener: Listener::new(chrome_on_message(), closure!(move |value: String, _sender: JsValue, reply: Function| -> bool {
            sender.unbounded_send((value, reply)).unwrap();
            // TODO somehow only return true when needed ?
            true
        })),
        receiver,
        _value: PhantomData,
    }
}


#[inline]
pub fn serialize_result<A>(value: Result<A, JsValue>) -> Result<A, String> {
    value.map_err(|err| {
        web_sys::console::error_1(&err);

        err.dyn_into::<Error>()
            .unwrap()
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
        serde_json::to_string(&$value).unwrap()
    }
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


#[inline]
pub fn performance_now() -> f64 {
    WINDOW.with(|x| x.performance().unwrap().now())
}


#[inline]
pub fn current_date_pretty() -> String {
    Date::new_0().to_utc_string().into()
}


#[inline]
pub fn console_log(message: String) {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from(message));
}

#[inline]
pub fn console_error(message: String) {
    web_sys::console::error_1(&wasm_bindgen::JsValue::from(message));
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


// TODO move into gloo
pub struct Debouncer {
    time: u32,
    timer: i32,
    done: Rc<Cell<bool>>,
    closure: Closure<dyn FnMut()>,
}

impl Debouncer {
    fn clear_timeout(&self) {
        WINDOW.with(|window| {
            window.clear_timeout_with_handle(self.timer);
        })
    }

    fn set_timeout(time: u32, closure: &Closure<dyn FnMut()>) -> i32 {
        WINDOW.with(|window| {
            // TODO better i32 conversion
            window.set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), time as i32).unwrap()
        })
    }

    pub fn new<F>(time: u32, f: F) -> Self where F: FnOnce() + 'static {
        let done = Rc::new(Cell::new(false));

        let closure = {
            let done = done.clone();

            Closure::once(move || {
                done.set(true);
                f();
            })
        };

        let timer = Self::set_timeout(time, &closure);

        Self {
            time,
            timer,
            done,
            closure,
        }
    }

    pub fn reset(&mut self) {
        if !self.done.get() {
            self.clear_timeout();
            self.timer = Self::set_timeout(self.time, &self.closure);
        }
    }
}

impl Drop for Debouncer {
    fn drop(&mut self) {
        self.done.set(true);
        self.clear_timeout();
    }
}


pub fn reload_page() {
    WINDOW.with(|x| x.location().reload().unwrap())
}


pub fn export_function<A>(name: &str, f: Closure<A>) where A: wasm_bindgen::closure::WasmClosure + ?Sized {
    WINDOW.with(|window| js_sys::Reflect::set(&window, &JsValue::from(name), f.as_ref())).unwrap();
    f.forget();
}


/*impl Tab {
    // TODO is i32 correct ?
    #[inline]
    pub fn id(&self) -> i32 {
        js!( return @{self}.id; ).try_into().unwrap()
    }
}*/


#[macro_export]
macro_rules! closure {
    (move || -> $ret:ty $body:block) => {
        wasm_bindgen::closure::Closure::wrap(std::boxed::Box::new(move || -> $ret { $body }) as std::boxed::Box<dyn FnMut() -> $ret>)
    };
    (move |$($arg:ident: $type:ty),*| -> $ret:ty $body:block) => {
        wasm_bindgen::closure::Closure::wrap(std::boxed::Box::new(move |$($arg: $type),*| -> $ret { $body }) as std::boxed::Box<dyn FnMut($($type),*) -> $ret>)
    };
    (move || $body:block) => {
        $crate::closure!(move || -> () $body)
    };
    (move |$($arg:ident: $type:ty),*| $body:block) => {
        $crate::closure!(move |$($arg: $type),*| -> () $body);
    };
}


#[wasm_bindgen]
extern "C" {
    #[derive(Debug)]
    pub type Event;

    #[wasm_bindgen(method, js_name = addListener)]
    pub fn add_listener(this: &Event, callback: &Function);

    #[wasm_bindgen(method, js_name = removeListener)]
    pub fn remove_listener(this: &Event, callback: &Function);
}


pub struct Listener<A> where A: ?Sized {
    event: Event,
    closure: ManuallyDrop<Closure<A>>,
}

// TODO use derive instead
impl<A> std::fmt::Debug for Listener<A> where A: ?Sized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Listener")
            .field("event", &self.event)
            .field("closure", &self.closure)
            .finish()
    }
}

impl<A> Listener<A> where A: ?Sized {
    pub fn new(event: Event, closure: Closure<A>) -> DiscardOnDrop<Self> {
        event.add_listener(closure.as_ref().unchecked_ref());

        DiscardOnDrop::new(Self {
            event,
            closure: ManuallyDrop::new(closure),
        })
    }
}

impl<A> Discard for Listener<A> where A: ?Sized {
    fn discard(self) {
        let closure = ManuallyDrop::into_inner(self.closure);
        self.event.remove_listener(closure.as_ref().unchecked_ref());
    }
}


#[wasm_bindgen]
extern "C" {
    pub type Tab;
}


#[wasm_bindgen]
extern "C" {
    type Sender;

    #[wasm_bindgen(method, getter)]
    fn tab(this: &Sender) -> Option<Tab>;
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
    fn sender(this: &RawPort) -> Sender;

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
    fn new(port: RawPort) -> Self {
        let disconnected = Rc::new(Cell::new(false));

        let listener = {
            let disconnected = disconnected.clone();

            // TODO cleanup the Listener when the closure is called ?
            Listener::new(port.on_disconnect(), Closure::once(move || {
                disconnected.set(true);
            }))
        };

        Self {
            port,
            disconnected,
            listener,
        }
    }

    // TODO trigger existing onDisconnect listeners
    fn disconnect(&self) {
        self.disconnected.set(true);
        self.port.disconnect();
    }
}


pub struct PortOnDisconnect {
    listener: Option<Listener<dyn FnMut()>>,
}

impl Discard for PortOnDisconnect {
    fn discard(self) {
        if let Some(listener) = self.listener {
            listener.discard();
        }
    }
}


pub struct PortOnMessage {
    on_disconnect: PortOnDisconnect,
}

impl Discard for PortOnMessage {
    fn discard(self) {
        self.on_disconnect.discard();
    }
}


#[derive(Clone, Debug)]
pub struct Port {
    state: Rc<PortState>,
}

impl Port {
    fn new(port: RawPort) -> Self {
        Self {
            state: Rc::new(PortState::new(port)),
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
            _listener: DiscardOnDrop<PortOnMessage>,
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
                sender.unbounded_send(message).unwrap();
            }),
        }
    }

    #[inline]
    pub fn on_message<A, F>(&self, mut f: F) -> DiscardOnDrop<PortOnMessage>
        where A: DeserializeOwned,
              F: FnMut(A) + 'static {

        // TODO error checking
        // TODO use |message: &str, _port: &JsValue|
        let on_message = Listener::new(self.state.port.on_message(), closure!(move |message: String, _port: JsValue| {
            f(serde_json::from_str(&message).unwrap());
        }));

        DiscardOnDrop::new(PortOnMessage {
            // TODO reconnect when it is disconnected ?
            on_disconnect: DiscardOnDrop::leak(self.on_disconnect(move || {
                drop(on_message);
            })),
        })
    }

    #[inline]
    pub fn on_disconnect<A>(&self, f: A) -> DiscardOnDrop<PortOnDisconnect>
        where A: FnOnce() + 'static {

        if self.state.disconnected.get() {
            f();

            DiscardOnDrop::new(PortOnDisconnect {
                listener: None,
            })

        } else {
            // TODO cleanup the Listener when the closure is called ?
            // TODO error checking
            DiscardOnDrop::new(PortOnDisconnect {
                listener: Some(DiscardOnDrop::leak(Listener::new(self.state.port.on_disconnect(), Closure::once(f)))),
            })
        }
    }

    // TODO return whether the message was sent or not ?
    #[inline]
    fn send_message_raw(&self, message: &str) {
        if !self.state.disconnected.get() {
            // TODO use try/catch to catch errors ?
            self.state.port.post_message(message);
        }
    }

    #[inline]
    pub fn send_message<A>(&self, message: &A) where A: Serialize {
        self.send_message_raw(&serde_json::to_string(message).unwrap());
    }
}

impl PartialEq for Port {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for Port {}


pub struct ServerPortOnConnect {
    listener: Listener<dyn FnMut(RawPort)>,
}

impl Discard for ServerPortOnConnect {
    fn discard(self) {
        self.listener.discard();
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

    #[inline]
    pub fn on_connect<F>(mut f: F) -> DiscardOnDrop<ServerPortOnConnect> where F: FnMut(Self) + 'static {
        let listener = Listener::new(chrome_on_connect(), closure!(move |port: RawPort| {
            f(ServerPort(Port::new(port)));
        }));

        DiscardOnDrop::new(ServerPortOnConnect {
            listener: DiscardOnDrop::leak(listener),
        })
    }
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


#[derive(Debug, Clone)]
pub struct NodeListIter {
    list: NodeList,
    range: std::ops::Range<u32>,
}

impl NodeListIter {
    pub fn new(list: NodeList) -> Self {
        Self {
            range: 0..list.length(),
            list,
        }
    }
}

impl std::iter::Iterator for NodeListIter {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.range.next()?;
        Some(self.list.get(index).unwrap())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl std::iter::DoubleEndedIterator for NodeListIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        let index = self.range.next_back()?;
        Some(self.list.get(index).unwrap())
    }
}

impl std::iter::FusedIterator for NodeListIter {}

impl std::iter::ExactSizeIterator for NodeListIter {}


pub struct MutationObserver {
    observer: web_sys::MutationObserver,
    closure: ManuallyDrop<Closure<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>>,
}

impl Discard for MutationObserver {
    fn discard(self) {
        let _ = ManuallyDrop::into_inner(self.closure);
        self.observer.disconnect();
    }
}

impl MutationObserver {
    pub fn new<F>(mut f: F) -> DiscardOnDrop<Self> where F: FnMut(Vec<web_sys::MutationRecord>) + 'static {
        let closure = closure!(move |records: js_sys::Array, _observer: web_sys::MutationObserver| {
            f(records.iter().map(|x| x.dyn_into().unwrap()).collect());
        });

        let observer = web_sys::MutationObserver::new(closure.as_ref().unchecked_ref()).unwrap();

        DiscardOnDrop::new(Self {
            observer,
            closure: ManuallyDrop::new(closure),
        })
    }

    pub fn observe(&self, target: &Node, options: &web_sys::MutationObserverInit) {
        self.observer.observe_with_options(target, options).unwrap();
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
    // TODO https://github.com/rustwasm/wasm-bindgen/pull/1684
    set_utc_date(&date, (date.get_utc_date() as f64) - (days as f64));
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
