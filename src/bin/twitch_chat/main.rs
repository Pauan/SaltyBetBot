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

// WAIFU4u: "wtfSalt ♫ "

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate salty_bet_bot;
extern crate regex;
extern crate serde_json;
#[macro_use]
extern crate stdweb;

use std::iter::Iterator;
use salty_bet_bot::common::{parse_f64, wait_until_defined, Port, get_text_content, WaifuMessage, WaifuBetsOpen, WaifuBetsClosed, WaifuBetsClosedInfo, WaifuWinner};
use salty_bet_bot::record::{Tier, Mode, Winner};
use stdweb::web::{document, IElement, Element, MutationObserver, MutationObserverInit, MutationRecord, Date, Node};
use stdweb::unstable::{TryInto};


// TODO can this performance be improved ?
fn to_string(x: regex::Match) -> String {
    x.as_str().to_string()
}


fn parse_tier(input: &str) -> Option<Tier> {
    match input {
        "X" => Some(Tier::X),
        "S" => Some(Tier::S),
        "A" => Some(Tier::A),
        "B" => Some(Tier::B),
        "P" => Some(Tier::P),
        _ => None,
    }
}

fn parse_mode(input: &str) -> Option<Mode> {
    match input {
        "(matchmaking) www.saltybet.com" => Some(Mode::Matchmaking),
        "tournament bracket: http://www.saltybet.com/shaker?bracket=1" => Some(Mode::Tournament),
        _ => None,
    }
}

fn parse_bets_open(input: &str, date: f64) -> Option<WaifuMessage> {
    lazy_static! {
        static ref BET_OPEN_REGEX: regex::Regex = regex::Regex::new(
            r"^Bets are OPEN for (.+) vs (.+?) *! \(([XSABP]) Tier\) ((?:\(matchmaking\) www\.saltybet\.com)|(?:tournament bracket: http://www\.saltybet\.com/shaker\?bracket=1))$"
        ).unwrap();
    }

    BET_OPEN_REGEX.captures(input).and_then(|capture|
        capture.get(1).map(to_string).and_then(|left|
        capture.get(2).map(to_string).and_then(|right|
        capture.get(3).and_then(|x| parse_tier(x.as_str())).and_then(|tier|
        capture.get(4).and_then(|x| parse_mode(x.as_str())).map(|mode|
            WaifuMessage::BetsOpen(WaifuBetsOpen { left, right, tier, mode, date }))))))
}


fn parse_bets_closed(input: &str, date: f64) -> Option<WaifuMessage> {
    lazy_static! {
        static ref BETS_CLOSED_REGEX: regex::Regex = regex::Regex::new(
            r"^Bets are locked\. (.+) \((\-?[0-9,]+)\) \- \$([0-9,]+), (.+) \((\-?[0-9,]+)\) \- \$([0-9,]+)$"
        ).unwrap();
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

    BETS_CLOSED_REGEX.captures(input).and_then(|capture|
        capture.get(1).map(to_string).and_then(|left_name|
        capture.get(2).and_then(|x| parse_f64(x.as_str())).and_then(|left_win_streak|
        capture.get(3).and_then(|x| parse_f64(x.as_str())).and_then(|left_bet_amount|
        capture.get(4).map(to_string).and_then(|right_name|
        capture.get(5).and_then(|x| parse_f64(x.as_str())).and_then(|right_win_streak|
        capture.get(6).and_then(|x| parse_f64(x.as_str())).map(|right_bet_amount|
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
    lazy_static! {
        static ref WINNER_REGEX: regex::Regex = regex::Regex::new(
            r"^(.+) wins! Payouts to Team (Red|Blue)\. "
        ).unwrap();
    }

    WINNER_REGEX.captures(input).and_then(|capture|
        capture.get(1).map(to_string).and_then(|name|
        capture.get(2).and_then(|x| parse_side(x.as_str())).map(|side|
            WaifuMessage::Winner(WaifuWinner { name, side, date }))))
}


fn parse_mode_switch(input: &str, date: f64) -> Option<WaifuMessage> {
    lazy_static! {
        static ref MODE_SWITCH_REGEX: regex::Regex = regex::Regex::new(
            r"^(?:Tournament|Exhibitions|Matchmaking) will start shortly\. Thanks for watching! wtfSALTY$"
        ).unwrap();
    }

    if MODE_SWITCH_REGEX.is_match(input) {
        Some(WaifuMessage::ModeSwitch { date })

    } else {
        None
    }
}


fn check_unknown_message(input: &str) -> Option<WaifuMessage> {
    lazy_static! {
        // wtfSALTY Bridget has been demoted!
        static ref UNKNOWN_REGEX: regex::Regex = regex::Regex::new(
            r"(?:^wtfSalt ♫ )|(?:^[XSABP](?: / [XSABP])? Tier$)|(?:^Current stage: )|(?:^(?:.+) by (?:.+?) *, (?:.+) by (?:.+)$)|(?:^Current odds: [0-9\.]+:[0-9\.]+$)|(?:^The current game mode is: (?:matchmaking|tournament|exhibitions)\. [0-9]+ (?:more matches until the next tournament|characters are left in the bracket|exhibition matches left)!$)|(?:^Download WAIFU Wars at www\.waifuwars\.com !$)|(?:^Current pot total: \$[0-9]+$)|(?:^The current tournament bracket can be found at: http://www\.saltybet\.com/shaker\?bracket=1$)|(?:^wtfVeku Note: (?:.+) \(from (?:.+?) *\)$)|(?:^wtfSALTY (?:.+) is fighting to stay in [SAB] Tier!$)|(?:^wtfSALTY New Waifu Wars bounties available! Winner: (?:.+) \(wave [0-9,]+\)! Play for free at http://www\.waifuwars\.com$)|(?:^wtfSalt Congrats tournament winner! (?:.+) \(\+\$[0-9,]+\)$)|(?:^The current game mode is: tournament\. FINAL ROUND! Stay tuned for exhibitions after the tournament!$)|(?:^Bets are locked\. (?:.+?) *\- \$[0-9,]+, (?:.+?) *\- \$[0-9,]+$)|(?:^(?:.+) vs (?:.+) was requested by (?:.+?) *\. OMGScoots$)|(?:^Palettes of previous match: [0-9]+(?: / [0-9]+)?, [0-9]+(?: / [0-9]+)?$)|(?:^Bets are OPEN for (?:.+) vs (?:.+?) *!(?: \([XSABP](?: / [XSABP])? Tier\))? \(Requested by (?:.+?) *\) \(exhibitions\) www\.saltybet\.com$)|(?:^The current game mode is: matchmaking\. Matchmaking mode will be activated after the next exhibition match!$)|(?:^The current game mode is: tournament\. Tournament mode will be activated after the next match!$)|(?:^wtfSALTY (?:.+) has been demoted!$)"
        ).unwrap();
    }

    if !UNKNOWN_REGEX.is_match(input) {
        log!("Unknown message: {:#?}", input);
    }

    None
}

fn parse_message(input: &str, date: f64) -> Option<WaifuMessage> {
    parse_bets_open(input, date).or_else(||
    parse_bets_closed(input, date).or_else(||
    parse_winner(input, date).or_else(||
    parse_mode_switch(input, date).or_else(||
    check_unknown_message(input)))))
}


fn get_waifu_message(node: Node, date: f64) -> Option<WaifuMessage> {
    // TODO better error handling ?
    node.try_into().ok().and_then(|node: Element|
        node.query_selector("span.from")
            .and_then(get_text_content)
            .and_then(|name| {
                if name == "WAIFU4u" || name == "SaltyBet" {
                    node.query_selector("span.message")
                        .and_then(get_text_content)
                        .and_then(|x| parse_message(&x, date))

                } else {
                    None
                }
            }))
}


pub fn get_waifu_messages() -> Vec<WaifuMessage> {
    let now: f64 = Date::now();

    document().query_selector_all("ul.chat-lines > li.message-line.chat-line").into_iter()
        .filter_map(|x| get_waifu_message(x, now))
        .collect()
}


pub fn observe_changes() {
    let port = Port::new("twitch_chat");

    let observer = MutationObserver::new(move |records, _| {
        let now: f64 = Date::now();

        let messages: Vec<WaifuMessage> = records.into_iter().filter_map(|record|
            match record {
                MutationRecord::ChildList { inserted_nodes, .. } => Some(
                    inserted_nodes.into_iter()
                        .filter_map(|x| get_waifu_message(x, now))
                ),
                _ => None,
            }
        ).flat_map(|x| x).collect();

        if messages.len() != 0 {
            port.send_message(&serde_json::to_string(&messages).unwrap());
        }
    });

    wait_until_defined(|| document().query_selector("ul.chat-lines"), move |lines| {
        observer.observe(&lines, MutationObserverInit {
            child_list: true,
            attributes: false,
            character_data: false,
            subtree: false,
            attribute_old_value: false,
            character_data_old_value: false,
            attribute_filter: None,
        });

        std::mem::forget(observer);

        log!("Observer initialized");
    });
}


fn main() {
    stdweb::initialize();

    observe_changes();

    stdweb::event_loop();
}
