// WAIFU4u: Bets are OPEN for FOO vs BAR! (B Tier) (matchmaking) www.saltybet.com
// WAIFU4u: Bets are locked. FOO (6) - $515,396, BAR (1) - $896,035
// WAIFU4u: FOO wins! Payouts to Team Red. 77 more matches until the next tournament!

// WAIFU4u: Bets are OPEN for FOO vs BAR! (B Tier) tournament bracket: http://www.saltybet.com/shaker?bracket=1
// WAIFU4u: Bets are locked. FOO (-5) - $141,061, BAR (3) - $638,656
// WAIFU4u: BAR wins! Payouts to Team Blue. 13 characters are left in the bracket!

// SaltyBet: Tournament will start shortly. Thanks for watching!  wtfSALTY
// WAIFU4u: BAR wins! Payouts to Team Blue. 16 characters are left in the bracket!

// SaltyBet: Exhibitions will start shortly. Thanks for watching!  wtfSALTY
// SaltyBet: wtfSalt Congrats tournament winner! stuker (+$1,219,553)
// WAIFU4u: BAR wins! Payouts to Team Blue. 25 exhibition matches left!

// WAIFU4u: Bets are OPEN for FOO vs BAR! (Requested by WormPHD) (exhibitions) www.saltybet.com
// WAIFU4u: Bets are OPEN for FOO vs BAR! (X / X Tier) (Requested by Yeno) (exhibitions) www.saltybet.com
// WAIFU4u: Bets are locked. FOO- $723,823, BAR- $60,903
// WAIFU4u: FOO wins! Payouts to Team Red. 24 exhibition matches left!

// Bets are OPEN for Team DoraTheEmployer vs Team NoSwiping! (Requested by NinaYamada) (exhibitions) www.saltybet.com

// WAIFU4u: "wtfSalt ♫ "

use std::iter::Iterator;
use std::rc::Rc;
use std::cell::RefCell;
use discard::DiscardOnDrop;
use gloo_timers::callback::Timeout;
use salty_bet_bot::{parse_f64, wait_until_defined, ClientPort, get_text_content, query, query_all, regexp, reload_page, Debouncer, server_log, log, NodeListIter, DOCUMENT, MutationObserver};
use salty_bet_bot::api::{WaifuMessage, WaifuBetsOpen, WaifuBetsClosed, WaifuBetsClosedInfo, WaifuWinner};
use algorithm::record::{Tier, Mode, Winner};
use js_sys::Date;
use web_sys::{Node, Element, HtmlImageElement, MutationRecord, MutationObserverInit};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;


// 15 minutes
const REFRESH_TIME: u32 = 1000 * 60 * 15;


fn parse_tier(input: &Option<String>) -> Option<Tier> {
    match input.as_deref() {
        // TODO is this correct ?
        None => Some(Tier::None),
        Some("NEW") => Some(Tier::New),
        Some("None") => Some(Tier::None),
        Some("X") => Some(Tier::X),
        Some("S") => Some(Tier::S),
        Some("A") => Some(Tier::A),
        Some("B") => Some(Tier::B),
        Some("P") => Some(Tier::P),
        _ => None,
    }
}

fn parse_mode(input: &str) -> Option<Mode> {
    match input {
        "(matchmaking) www.saltybet.com" => Some(Mode::Matchmaking),
        "tournament bracket: http://www.saltybet.com/shaker?bracket=1" => Some(Mode::Tournament),
        "(exhibitions) www.saltybet.com" => Some(Mode::Exhibitions),
        _ => None,
    }
}

fn parse_bets_open(input: &str, date: f64) -> Option<WaifuMessage> {
    thread_local! {
        static BET_OPEN_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^Bets are OPEN for (.+) vs (.+?) *!(?: \((NEW|None|[XSABP])(?: / (?:NEW|None|[XSABP]))? Tier\))? (?:\(Requested by .+? *\) )?((?:\(matchmaking\) www\.saltybet\.com)|(?:tournament bracket: http://www\.saltybet\.com/shaker\?bracket=1)|(?:\(exhibitions\) www\.saltybet\.com))$"
        );
    }

    BET_OPEN_REGEX.with(|re| re.first_match(input)).and_then(|mut captures|
        captures[1].take().and_then(|left|
        captures[2].take().and_then(|right|
        parse_tier(&captures[3]).and_then(|tier|
        captures[4].as_ref().and_then(|x| parse_mode(x)).map(|mode|
            WaifuMessage::BetsOpen(WaifuBetsOpen { left, right, tier, mode, date }))))))
}


fn parse_bets_closed(input: &str, date: f64) -> Option<WaifuMessage> {
    thread_local! {
        static BETS_CLOSED_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^Bets are locked\. (.+) \((-?[0-9,]+)\) - \$([0-9,]+), (.+) \((-?[0-9,]+)\) - \$([0-9,]+)$"
        );
    }

    /*let capture = BETS_CLOSED_REGEX.captures(input)?;
    let left_name        = capture.get(1)?;
    let left_win_streak  = capture.get(2)?;
    let left_bet_amount  = capture.get(3)?;
    let right_name       = capture.get(4)?;
    let right_win_streak = capture.get(5)?;
    let right_bet_amount = capture.get(6)?;

    Some(WaifuMessage::BetsClosed(WaifuBetsClosed {
        left: WaifuBetsClosedInfo {
            name: to_string(left_name),
            win_streak: parse_f64(left_win_streak.as_str()),
            bet_amount: parse_f64(left_bet_amount.as_str()),
        },
        right: WaifuBetsClosedInfo {
            name: to_string(right_name),
            win_streak: parse_f64(right_win_streak.as_str()),
            bet_amount: parse_f64(right_bet_amount.as_str()),
        },
        date: date
    }))*/

    BETS_CLOSED_REGEX.with(|re| re.first_match(input)).and_then(|mut captures|
        captures[1].take().and_then(|left_name|
        captures[2].as_ref().and_then(|x| parse_f64(x)).and_then(|left_win_streak|
        captures[3].as_ref().and_then(|x| parse_f64(x)).and_then(|left_bet_amount|
        captures[4].take().and_then(|right_name|
        captures[5].as_ref().and_then(|x| parse_f64(x)).and_then(|right_win_streak|
        captures[6].as_ref().and_then(|x| parse_f64(x)).map(|right_bet_amount|
            WaifuMessage::BetsClosed(WaifuBetsClosed {
                left: WaifuBetsClosedInfo {
                    name: left_name,
                    win_streak: left_win_streak,
                    bet_amount: left_bet_amount,
                },
                right: WaifuBetsClosedInfo {
                    name: right_name,
                    win_streak: right_win_streak,
                    bet_amount: right_bet_amount,
                },
                date: date
            }))))))))
}


fn parse_side(input: &str) -> Option<Winner> {
    match input {
        "Red" => Some(Winner::Left),
        "Blue" => Some(Winner::Right),
        _ => None,
    }
}

fn parse_winner(input: &str, date: f64) -> Option<WaifuMessage> {
    thread_local! {
        static WINNER_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^(.+) wins! Payouts to Team (Red|Blue)\. "
        );
    }

    WINNER_REGEX.with(|re| re.first_match(input)).and_then(|mut captures|
        captures[1].take().and_then(|name|
        captures[2].as_ref().and_then(|x| parse_side(x)).map(|side|
            WaifuMessage::Winner(WaifuWinner { name, side, date }))))
}


fn parse_mode_switch(input: &str, date: f64) -> Option<WaifuMessage> {
    thread_local! {
        static MODE_SWITCH_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^(Tournament|Exhibitions|Matchmaking) will start shortly\. Thanks for watching! wtfSALTY$"
        );
    }

    MODE_SWITCH_REGEX.with(|re| re.first_match(input)).and_then(|mut captures|
        captures[1].take().map(|mode| {
            let is_exhibition = mode == "Exhibitions";
            WaifuMessage::ModeSwitch { date, is_exhibition }
        }))
}


fn check_unknown_message(input: &str) -> Option<WaifuMessage> {
    thread_local! {
        static UNKNOWN_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"(?:^wtfSalt ♫ )|(?:^(?:NEW|None|[XSABP])(?: / (?:NEW|None|[XSABP]))? Tier$)|(?:^Current stage: )|(?:^(?:.+) by(?: .+?)? *, (?:.+) by(?: .+)?$)|(?:^Current odds: [0-9\.]+:[0-9\.]+$)|(?:^The current game mode is: (?:matchmaking|tournament|exhibitions)\. [0-9]+ (?:more matches until the next tournament|characters are left in the bracket|exhibition matches left)!$)|(?:^Download WAIFU Wars at www\.waifuwars\.com! https://clips\.twitch\.tv/UninterestedHumbleCiderWoofer)|(?:^Current pot total: \$[0-9]+$)|(?:^The current tournament bracket can be found at: http://www\.saltybet\.com/shaker\?bracket=1$)|(?:^wtfVeku Note: .*\(from (?:.*?) *\)$)|(?:^wtfSALTY (?:.+) is fighting to stay in [SAB] Tier!$)|(?:^wtfSALTY New Waifu Wars bounties available! Winner: (?:.+) \(wave [0-9,]+\)! Play for free at http://www\.waifuwars\.com$)|(?:^wtfSalt Congrats tournament winner! (?:.+) \(\+\$[0-9,]+\)$)|(?:^The current game mode is: (?:tournament|exhibitions)\. FINAL ROUND! Stay tuned for exhibitions after the tournament!$)|(?:^Bets are locked\. (?:.+?) *- \$[0-9,]+, (?:.+?) *- \$[0-9,]+$)|(?:^(?:.+) vs (?:.+) was requested by (?:.+?) *\. OMGScoots$)|(?:^Palettes of previous match: [0-9]+(?: / [0-9]+)?, [0-9]+(?: / [0-9]+)?$)|(?:^The current game mode is: (?:matchmaking|exhibitions)\. Matchmaking mode will be activated after the next exhibition match!$)|(?:^The current game mode is: tournament\. Tournament mode will be activated after the next match!$)|(?:^wtfSALTY (?:.+) has been demoted!$)|(?:^(?:.+) vs (?:.+) was requested by RNG\. Kappa$)|(?:^The current game mode is: matchmaking\. Tournament mode will be activated after the next match!$)|(?:^wtfSALTY (?:.+) is fighting for a promotion from [ABP] to [SAB] Tier!$)|(?:^wtfSALTY (?:.+) has been promoted!$)|(?:^[0-9,]+ characters are left in NEW tier: http://www\.saltybet\.com/stats\?playerstats=1&new=1$)|(?:^Join the official Salty Bet Illuminati Discord! https://discord\.gg/saltybet$)|(?:^https://www\.waifuwars\.com$)"
        );
    }

    if !UNKNOWN_REGEX.with(|re| re.is_match(input)) {
        server_log!("Unknown message: {:#?}", input);
    }

    None
}

fn parse_message(input: &str, date: f64) -> Option<WaifuMessage> {
    thread_local! {
        static NAME_REGEX: regexp::RegExp = regexp::RegExp::new(r"^([^:]+): *(.*)$");
    }

    NAME_REGEX.with(|re| re.first_match(input)).and_then(|captures|
        captures[1].as_ref().and_then(|name| {
            if name == "WAIFU4u" || name == "SaltyBet" {
                captures[2].as_ref().and_then(|message| {
                    parse_bets_open(message, date).or_else(||
                    parse_bets_closed(message, date).or_else(||
                    parse_winner(message, date).or_else(||
                    parse_mode_switch(message, date).or_else(||
                    check_unknown_message(message)))))
                })

            } else {
                None
            }
        }))
}


fn get_waifu_message(node: Node, date: f64) -> Option<WaifuMessage> {
    // This is to avoid mutating the DOM of the chat
    let node = node.clone_node_with_deep(true).unwrap();

    let node: Element = node.dyn_into().unwrap();

    // This removes the Twitch badges
    // TODO quite hacky, make this more robust
    node.remove_child(&node.first_child().unwrap()).unwrap();

    // Hack to replace emotes with their text version, needed because sometimes fighters have emotes in their name
    // TODO can this be made better somehow ?
    for node in NodeListIter::new(node.query_selector_all("img").unwrap()) {
        let node: HtmlImageElement = node.dyn_into().unwrap();

        node.parent_node()
            .unwrap()
            .replace_child(&DOCUMENT.with(|document| document.create_text_node(&node.alt())), &node)
            .unwrap();
    }

    get_text_content(&node).and_then(|x| parse_message(&x, date))
}


pub fn get_waifu_messages() -> Vec<WaifuMessage> {
    let now: f64 = Date::now();

    NodeListIter::new(query_all("[data-a-target='chat-line-message']"))
        .filter_map(|x| get_waifu_message(x, now))
        .collect()
}


#[wasm_bindgen(start)]
pub fn main_js() {
    console_error_panic_hook::set_once();


    log!("Initializing...");

    let port = ClientPort::connect("twitch_chat");

    let mut debounce = {
        let port = port.clone();

        Debouncer::new(REFRESH_TIME, move || {
            // This will cause the SaltyBet tab to reload
            port.send_message(&vec![WaifuMessage::ReloadPage]);

            // 5 minutes
            const RELOAD_DELAY: u32 = 1000 * 60 * 5;

            // This is an extra precaution in case the SaltyBet tab doesn't reload
            Timeout::new(RELOAD_DELAY, move || reload_page()).forget();
        })
    };

    let observer = {
        let port = port.clone();

        MutationObserver::new(move |records: Vec<MutationRecord>| {
            let now: f64 = Date::now();

            let messages: Vec<WaifuMessage> = records.into_iter().filter_map(|record| {
                assert_eq!(record.type_().as_str(), "childList");

                let inserted_nodes = record.added_nodes();

                if inserted_nodes.length() == 0 {
                    None

                } else {
                    debounce.reset();

                    Some(
                        NodeListIter::new(inserted_nodes)
                            .filter_map(|x| get_waifu_message(x, now))
                    )
                }
            }).flat_map(|x| x).collect();

            if messages.len() != 0 {
                port.send_message(&messages);
            }
        })
    };

    let observer = Rc::new(RefCell::new(Some(observer)));

    {
        let observer = observer.clone();

        DiscardOnDrop::leak(port.on_disconnect(move || {
            *observer.borrow_mut() = None;
        }));
    }

    /*wait_until_defined(|| query("body"), move |body| {
        js! { @(no_return)
            @{body}.style.display = "none";
        }

        log!("Body hidden");
    });*/

    wait_until_defined(|| query("[data-a-target='chat-welcome-message']"), move |welcome| {
        if let Some(observer) = observer.borrow().as_ref() {
            observer.observe(&welcome.parent_node().unwrap(), MutationObserverInit::new()
                .child_list(true));

            log!("Observer initialized");

        } else {
            log!("Port disconnected");
        }
    });
}
