#![recursion_limit="256"]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate stdweb_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate algorithm;

pub mod regexp;
mod macros;

use algorithm::record::{Tier, Mode, Winner, Record};
use algorithm::types::BetStrategy;
use stdweb::{Value, Once};
use stdweb::web::{document, set_timeout, INode, Element, NodeList};
use stdweb::web::html_element::InputElement;
use stdweb::unstable::{TryInto};
use stdweb::traits::*;


// 50 minutes
// TODO is this high enough ?
pub const MAX_MATCH_TIME_LIMIT: f64 = 1000.0 * 60.0 * 50.0;


pub fn matchmaking_strategy() -> BetStrategy {
    serde_json::from_str(&include_str!("../strategies/2018-08-20T12.29.43 (matchmaking)")).unwrap()
}


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


pub fn create_tab<A>(done: A)
    where A: FnOnce() + 'static {

    js! { @(no_return)
        // TODO error handling
        chrome.runtime.sendMessage(null, { type: "tabs:open-twitch-chat" }, null, function () {
            @{Once(done)}();
        });
    }
}

pub fn records_get_all<A>(done: A)
    where A: FnOnce(Vec<Record>) + 'static {

    let done = move |records: Vec<String>| done(records.into_iter().map(|x| Record::deserialize(&x)).collect());

    js! { @(no_return)
        // TODO error handling
        chrome.runtime.sendMessage(null, { type: "records:get-all" }, null, function (value) {
            @{Once(done)}(value);
        });
    }
}

pub fn records_insert<A>(record: &Record, done: A) where A: FnOnce() + 'static {
    js! { @(no_return)
        // TODO error handling
        chrome.runtime.sendMessage(null, {
            type: "records:insert",
            value: [@{record.serialize()}]
        }, null, function () {
            @{Once(done)}();
        });
    }
}

pub fn records_insert_many<A>(records: &[Record], done: A) where A: FnOnce() + 'static {
    // TODO more idiomatic check
    if records.len() > 0 {
        let records: Vec<String> = records.into_iter().map(Record::serialize).collect();

        js! { @(no_return)
            // TODO error handling
            chrome.runtime.sendMessage(null, {
                type: "records:insert",
                value: @{records}
            }, null, function () {
                @{Once(done)}();
            });
        }

    } else {
        done();
    }
}

pub fn records_delete_all<A>(done: A) where A: FnOnce() + 'static {
    js! { @(no_return)
        // TODO error handling
        chrome.runtime.sendMessage(null, { type: "records:delete-all" }, null, function () {
            @{Once(done)}();
        });
    }
}

pub fn serialize_records(records: Vec<Record>) -> String {
    serde_json::to_string_pretty(&records).unwrap()
}

pub fn deserialize_records(records: &str) -> Vec<Record> {
    serde_json::from_str(records).unwrap()
}


#[must_use]
pub struct Listener<'a> {
    callback: Value,
    stop: Value,
    phantom: std::marker::PhantomData<&'a Value>,
}

impl<'a> Drop for Listener<'a> {
    #[inline]
    fn drop(&mut self) {
        js! { @(no_return)
            @{&self.stop}();
            @{&self.callback}.drop();
        }
    }
}


/*pub fn get_storage<A>(key: &str, f: A)
    where A: FnOnce(Option<String>) + 'static {
    js! { @(no_return)
        // TODO error handling
        chrome.storage.local.get(@{key}, function (items) {
            @{Once(f)}(items[@{key}]);
        });
    }
}*/


// TODO verify that this sets things in the correct order if called multiple times
/*pub fn set_storage(key: &str, value: &str) {
    js! { @(no_return)
        var obj = {};
        obj[@{key}] = @{value};
        // TODO error handling
        chrome.storage.local.set(obj);
    }
}*/


/*pub fn delete_storage<A>(key: &str, f: A)
    where A: FnOnce() + 'static {
    js! { @(no_return)
        // TODO error handling
        chrome.storage.local.remove(@{key}, function () {
            @{Once(f)}();
        });
    }
}*/


#[inline]
pub fn performance_now() -> f64 {
    js!( return performance.now(); ).try_into().unwrap()
}


pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(move |info| {
        stdweb::print_error_panic(info.to_string());
    }));
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


pub struct Port(Value);

// TODO handle onDisconnect
impl Port {
    #[inline]
    pub fn new(name: &str) -> Self {
        // TODO error checking
        Port(js! ( return chrome.runtime.connect(null, { name: @{name} }); ))
    }

    pub fn listen<'a, A>(&'a self, f: A) -> Listener<'a>
        where A: FnMut(String) + 'static {

        let callback = js!( return @{f}; );

        Listener {
            stop: js! {
                function listener(message) {
                    @{&callback}(message);
                }

                // TODO error checking
                @{&self.0}.onMessage.addListener(listener);

                return function () {
                    @{&self.0}.onMessage.removeListener(listener);
                };
            },
            callback: callback,
            phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn send_message(&self, message: &str) {
        js! { @(no_return)
            @{&self.0}.postMessage(@{message});
        }
    }
}

impl Drop for Port {
    #[inline]
    fn drop(&mut self) {
        js! { @(no_return)
            @{&self.0}.disconnect();
        }
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
