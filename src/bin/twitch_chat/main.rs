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
extern crate salty_bet_bot;
extern crate regex;
extern crate serde_json;
extern crate stdweb;

use std::rc::Rc;
use std::cell::RefCell;
use std::iter::Iterator;
use salty_bet_bot::common::{parse_f64, wait_until_defined, Port, Message, Information, remove_newlines, collapse_whitespace, get_text_content};
use salty_bet_bot::simulation::{Bet};
use salty_bet_bot::record::{Tier, Mode, Winner, Record, Character};
use stdweb::{Value};
use stdweb::web::{document, IElement, INode, Element, MutationObserver, MutationObserverInit, MutationObserverHandle, MutationRecord, set_timeout, Date, Node};
use stdweb::unstable::{TryInto};


// 10 minutes
// TODO is this high enough ?
const MAX_BET_TIME_LIMIT: f64 = 1000.0 * 60.0 * 10.0;

// 50 minutes
// TODO is this high enough ?
const MAX_MATCH_TIME_LIMIT: f64 = 1000.0 * 60.0 * 50.0;


// ~7.7 minutes
const NORMAL_MATCH_TIME: f64 = 1000.0 * (60.0 + (80.0 * 5.0));

// TODO
const MAX_EXHIBITS_DURATION: f64 = NORMAL_MATCH_TIME * 1.0;

// ~1.92 hours
const MAX_TOURNAMENT_DURATION: f64 = NORMAL_MATCH_TIME * 15.0;


#[derive(Debug, Clone)]
pub struct WaifuBetsOpen {
    left: String,
    right: String,
    tier: Tier,
    mode: Mode,
    date: f64,
}

#[derive(Debug, Clone)]
pub struct WaifuBetsClosedInfo {
    name: String,
    win_streak: f64,
    bet_amount: f64,
}

#[derive(Debug, Clone)]
pub struct WaifuBetsClosed {
    left: WaifuBetsClosedInfo,
    right: WaifuBetsClosedInfo,
    date: f64,
}

#[derive(Debug, Clone)]
pub struct WaifuWinner {
    name: String,
    side: Winner,
    date: f64,
}

#[derive(Debug, Clone)]
pub enum WaifuMessage {
    BetsOpen(WaifuBetsOpen),
    BetsClosed(WaifuBetsClosed),
    Winner(WaifuWinner),
    ModeSwitch { date: f64 },
}


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
        // TODO can this performance be improved ?
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
        static ref UNKNOWN_REGEX: regex::Regex = regex::Regex::new(
            r"(?:^wtfSalt ♫ )|(?:^[XSABP] Tier$)|(?:^Current stage: )|(?:^(?:.+) by (?:.+?) *, (?:.+) by (?:.+)$)|(?:^Current odds: [0-9\.]+:[0-9\.]+$)|(?:^The current game mode is: (?:matchmaking|tournament|exhibitions)\. [0-9]+ (?:more matches until the next tournament|characters are left in the bracket|exhibition matches left)!$)|(?:^Download WAIFU Wars at www\.waifuwars\.com !$)|(?:^Current pot total: \$[0-9]+$)|(?:^The current tournament bracket can be found at: http://www\.saltybet\.com/shaker\?bracket=1$)|(?:^wtfVeku Note: (?:.+) \(from (?:.+?) *\)$)|(?:^wtfSALTY (?:.+) is fighting to stay in [SAB] Tier!$)|(?:^wtfSALTY New Waifu Wars bounties available! Winner: (?:.+) \(wave [0-9,]+\)! Play for free at http://www\.waifuwars\.com$)|(?:^wtfSalt Congrats tournament winner! (?:.+) \(\+\$[0-9,]+\)$)|(?:^The current game mode is: tournament\. FINAL ROUND! Stay tuned for exhibitions after the tournament!$)|(?:^Bets are locked\. (?:.+?) *\- \$[0-9,]+, (?:.+?) *\- \$[0-9,]+$)|(?:^(?:.+) vs (?:.+) was requested by (?:.+?) *\. OMGScoots$)|(?:^Palettes of previous match: [0-9]+(?: / [0-9]+)?, [0-9]+(?: / [0-9]+)?$)|(?:^Bets are OPEN for (?:.+) vs (?:.+?) *!(?: \([XSABP](?: / [XSABP])? Tier\))? \(Requested by (?:.+?) *\) \(exhibitions\) www\.saltybet\.com$)|(?:^The current game mode is: matchmaking\. Matchmaking mode will be activated after the next exhibition match!$)"
        ).unwrap();
    }

    if !UNKNOWN_REGEX.is_match(input) {
        println!("Unknown message: {:#?}", input);
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


// TODO timer which prints an error message if it's been >5 hours since a successful match recording
pub fn observe_changes() {
    let mut old_open = None;
    let mut old_closed = None;
    let mut mode_switch = None;

    let information: Rc<RefCell<Option<Information>>> = Rc::new(RefCell::new(None));

    let observer = {
        let information = information.clone();

        MutationObserver::new(move |records, _| {
            let now: f64 = Date::now();

            let mut information = information.borrow_mut();

            for record in records {
                match record {
                    MutationRecord::ChildList { inserted_nodes, .. } => {
                        let messages = inserted_nodes.into_iter()
                            .filter_map(|x| get_waifu_message(x, now));

                        for message in messages {
                            match message {
                                WaifuMessage::BetsOpen(open) => {
                                    old_open = Some(open);
                                    old_closed = None;
                                    mode_switch = None;
                                    *information = None;
                                },

                                WaifuMessage::BetsClosed(closed) => {
                                    mode_switch = None;
                                    *information = None;

                                    match old_open {
                                        Some(ref open) => {
                                            let duration = closed.date - open.date;

                                            if duration >= 0.0 &&
                                               duration <= MAX_BET_TIME_LIMIT &&
                                               open.left == closed.left.name &&
                                               open.right == closed.right.name {
                                                old_closed = Some(closed);
                                                continue;

                                            } else {
                                                println!("Invalid messages: {:#?} {:#?}", open, closed);
                                            }
                                        },
                                        None => {},
                                    }

                                    old_open = None;
                                    old_closed = None;
                                },

                                WaifuMessage::ModeSwitch { date } => {
                                    match old_open {
                                        Some(ref open) => match old_closed {
                                            Some(_) => {
                                                mode_switch = Some(date);
                                                continue;
                                            },
                                            None => {
                                                println!("Invalid messages: {:#?} {:#?}", open, message);
                                            },
                                        },
                                        None => {},
                                    }

                                    mode_switch = None;
                                },

                                WaifuMessage::Winner(winner) => {
                                    match old_open {
                                        Some(ref open) => match old_closed {
                                            Some(ref mut closed) => match *information {
                                                Some(ref mut information) => {
                                                    let date = match mode_switch {
                                                        Some(date) => date,
                                                        None => winner.date,
                                                    };

                                                    let duration = date - closed.date;

                                                    if duration >= 0.0 &&
                                                       duration <= MAX_MATCH_TIME_LIMIT &&
                                                       match winner.side {
                                                           Winner::Left => winner.name == closed.left.name,
                                                           Winner::Right => winner.name == closed.right.name,
                                                       } {

                                                        match information.bet {
                                                            Bet::Left(amount) => {
                                                                if information.player_is_illuminati {
                                                                    information.left_bettors_illuminati -= 1.0;

                                                                } else {
                                                                    information.left_bettors_normal -= 1.0;
                                                                }

                                                                closed.left.bet_amount -= amount;
                                                            },
                                                            Bet::Right(amount) => {
                                                                if information.player_is_illuminati {
                                                                    information.right_bettors_illuminati -= 1.0;

                                                                } else {
                                                                    information.right_bettors_normal -= 1.0;
                                                                }

                                                                closed.right.bet_amount -= amount;
                                                            },
                                                            Bet::None => {},
                                                        }

                                                        // TODO figure out a way to avoid clone
                                                        println!("Successful match: {:#?}", Record {
                                                            left: Character {
                                                                name: closed.left.name.clone(),
                                                                bet_amount: closed.left.bet_amount,
                                                                win_streak: closed.left.win_streak,
                                                                illuminati_bettors: information.left_bettors_illuminati,
                                                                normal_bettors: information.left_bettors_normal,
                                                            },
                                                            right: Character {
                                                                name: closed.right.name.clone(),
                                                                bet_amount: closed.right.bet_amount,
                                                                win_streak: closed.right.win_streak,
                                                                illuminati_bettors: information.right_bettors_illuminati,
                                                                normal_bettors: information.right_bettors_normal,
                                                            },
                                                            winner: winner.side,
                                                            tier: open.tier.clone(),
                                                            mode: open.mode.clone(),
                                                            bet: information.bet.clone(),
                                                            duration,
                                                            date: date,
                                                        });

                                                    } else {
                                                        println!("Invalid messages: {:#?} {:#?} {:#?} {:#?}", open, closed, information, winner);
                                                    }
                                                },
                                                None => {
                                                    println!("Invalid messages: {:#?} {:#?} {:#?}", open, closed, winner);
                                                },
                                            },
                                            None => {
                                                println!("Invalid messages: {:#?} {:#?}", open, winner);
                                            },
                                        },
                                        None => {},
                                    }

                                    old_open = None;
                                    old_closed = None;
                                    mode_switch = None;
                                    *information = None;
                                },
                            }
                        }
                    },
                    _ => {},
                }
            }
        })
    };

    wait_until_defined(|| document().query_selector("ul.chat-lines"), move |lines| {
        let port = Port::new("twitch_chat");

        {
            let information = information.clone();

            std::mem::forget(port.listen(move |message| {
                let mut information = information.borrow_mut();

                match serde_json::from_str(&message).unwrap() {
                    Message::Information(message) => {
                        *information = Some(message);
                    },
                    _ => {},
                }
            }));
        }

        observer.observe(&lines, MutationObserverInit {
            child_list: true,
            attributes: false,
            character_data: false,
            subtree: false,
            attribute_old_value: false,
            character_data_old_value: false,
            attribute_filter: None,
        });

        std::mem::forget(port);
        std::mem::forget(observer);

        println!("Observer initialized");
    });
}


fn main() {
    stdweb::initialize();

    observe_changes();

    stdweb::event_loop();
}
