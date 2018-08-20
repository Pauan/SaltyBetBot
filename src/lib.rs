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

use algorithm::record::{Tier, Mode, Winner};
use algorithm::types::BetStrategy;
use stdweb::{Value, Once};
use stdweb::web::{document, set_timeout, INode, Element, NodeList};
use stdweb::web::html_element::InputElement;
use stdweb::unstable::{TryInto};
use stdweb::traits::*;


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
    ModeSwitch { date: f64 },
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
        chrome.runtime.sendMessage(null, {}, null, function () {
            @{Once(done)}();
        });
    }
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


pub fn get_storage<A>(key: &str, f: A)
    where A: FnOnce(Option<String>) + 'static {
    js! { @(no_return)
        // TODO error handling
        chrome.storage.local.get(@{key}, function (items) {
            @{Once(f)}(items[@{key}]);
        });
    }
}


// TODO verify that this sets things in the correct order if called multiple times
pub fn set_storage(key: &str, value: &str) {
    js! { @(no_return)
        var obj = {};
        obj[@{key}] = @{value};
        // TODO error handling
        chrome.storage.local.set(obj);
    }
}


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


pub fn subtract_days(previous_days: u32) -> f64 {
    js!(
        var date = new Date();
        date.setUTCDate(date.getUTCDate() - @{previous_days});
        return date.getTime();
    ).try_into().unwrap()
}
