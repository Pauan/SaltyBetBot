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
#[macro_use]
extern crate stdweb;

use std::iter::Iterator;
use salty_bet_bot::{parse_f64, wait_until_defined, Port, get_text_content, WaifuMessage, WaifuBetsOpen, WaifuBetsClosed, WaifuBetsClosedInfo, WaifuWinner, query, query_all, regexp, set_panic_hook};
use algorithm::record::{Tier, Mode, Winner};
use stdweb::web::{INode, MutationObserver, MutationObserverInit, MutationRecord, Date, Node, Element, IParentNode};
use stdweb::unstable::TryInto;


fn parse_tier(input: &str) -> Option<Tier> {
    match input {
        "NEW" => Some(Tier::New),
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
        static ref BET_OPEN_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^Bets are OPEN for (.+) vs (.+?) *! \((NEW|[XSABP]) Tier\) ((?:\(matchmaking\) www\.saltybet\.com)|(?:tournament bracket: http://www\.saltybet\.com/shaker\?bracket=1))$"
        );
    }

    BET_OPEN_REGEX.first_match(input).and_then(|mut captures|
        captures[1].take().and_then(|left|
        captures[2].take().and_then(|right|
        captures[3].as_ref().and_then(|x| parse_tier(x)).and_then(|tier|
        captures[4].as_ref().and_then(|x| parse_mode(x)).map(|mode|
            WaifuMessage::BetsOpen(WaifuBetsOpen { left, right, tier, mode, date }))))))
}


fn parse_bets_closed(input: &str, date: f64) -> Option<WaifuMessage> {
    lazy_static! {
        static ref BETS_CLOSED_REGEX: regexp::RegExp = regexp::RegExp::new(
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

    BETS_CLOSED_REGEX.first_match(input).and_then(|mut captures|
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
    lazy_static! {
        static ref WINNER_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^(.+) wins! Payouts to Team (Red|Blue)\. "
        );
    }

    WINNER_REGEX.first_match(input).and_then(|mut captures|
        captures[1].take().and_then(|name|
        captures[2].as_ref().and_then(|x| parse_side(x)).map(|side|
            WaifuMessage::Winner(WaifuWinner { name, side, date }))))
}


fn parse_mode_switch(input: &str, date: f64) -> Option<WaifuMessage> {
    lazy_static! {
        static ref MODE_SWITCH_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"^(Tournament|Exhibitions|Matchmaking) will start shortly\. Thanks for watching! wtfSALTY$"
        );
    }

    MODE_SWITCH_REGEX.first_match(input).and_then(|mut captures|
        captures[1].take().map(|mode| {
            let is_exhibition = mode == "Exhibitions";
            WaifuMessage::ModeSwitch { date, is_exhibition }
        }))
}


fn check_unknown_message(input: &str) -> Option<WaifuMessage> {
    lazy_static! {
        static ref UNKNOWN_REGEX: regexp::RegExp = regexp::RegExp::new(
            r"(?:^wtfSalt ♫ )|(?:^(?:NEW|None|[XSABP])(?: / (?:NEW|None|[XSABP]))? Tier$)|(?:^Current stage: )|(?:^(?:.+) by(?: .+?)? *, (?:.+) by(?: .+)?$)|(?:^Current odds: [0-9\.]+:[0-9\.]+$)|(?:^The current game mode is: (?:matchmaking|tournament|exhibitions)\. [0-9]+ (?:more matches until the next tournament|characters are left in the bracket|exhibition matches left)!$)|(?:^Download WAIFU Wars at www\.waifuwars\.com! https://clips\.twitch\.tv/UninterestedHumbleCiderWoofer)|(?:^Current pot total: \$[0-9]+$)|(?:^The current tournament bracket can be found at: http://www\.saltybet\.com/shaker\?bracket=1$)|(?:^wtfVeku Note: .*\(from (?:.*?) *\)$)|(?:^wtfSALTY (?:.+) is fighting to stay in [SAB] Tier!$)|(?:^wtfSALTY New Waifu Wars bounties available! Winner: (?:.+) \(wave [0-9,]+\)! Play for free at http://www\.waifuwars\.com$)|(?:^wtfSalt Congrats tournament winner! (?:.+) \(\+\$[0-9,]+\)$)|(?:^The current game mode is: (?:tournament|exhibitions)\. FINAL ROUND! Stay tuned for exhibitions after the tournament!$)|(?:^Bets are locked\. (?:.+?) *- \$[0-9,]+, (?:.+?) *- \$[0-9,]+$)|(?:^(?:.+) vs (?:.+) was requested by (?:.+?) *\. OMGScoots$)|(?:^Palettes of previous match: [0-9]+(?: / [0-9]+)?, [0-9]+(?: / [0-9]+)?$)|(?:^Bets are OPEN for (?:.+) vs (?:.+?) *!(?: \((?:NEW|None|[XSABP])(?: / (?:NEW|None|[XSABP]))? Tier\))? \(Requested by (?:.+?) *\) \(exhibitions\) www\.saltybet\.com$)|(?:^The current game mode is: (?:matchmaking|exhibitions)\. Matchmaking mode will be activated after the next exhibition match!$)|(?:^The current game mode is: tournament\. Tournament mode will be activated after the next match!$)|(?:^wtfSALTY (?:.+) has been demoted!$)|(?:^(?:.+) vs (?:.+) was requested by RNG\. Kappa$)|(?:^The current game mode is: matchmaking\. Tournament mode will be activated after the next match!$)|(?:^wtfSALTY (?:.+) is fighting for a promotion from [ABP] to [SAB] Tier!$)|(?:^wtfSALTY (?:.+) has been promoted!$)|(?:^[0-9,]+ characters are left in NEW tier: http://www\.saltybet\.com/stats\?playerstats=1&new=1$)|(?:^Join the official Salty Bet Illuminati Discord! https://discord\.gg/saltybet$)"
        );
    }

    if !UNKNOWN_REGEX.is_match(input) {
        server_log!("Unknown message: {:#?}", input);
    }

    None
}

fn parse_message(input: &str, date: f64) -> Option<WaifuMessage> {
    lazy_static! {
        static ref NAME_REGEX: regexp::RegExp = regexp::RegExp::new(r"^([^:]+): *(.*)$");
    }

    NAME_REGEX.first_match(input).and_then(|captures|
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
    let node: Element = node.try_into().unwrap();

    // This removes the Twitch badges
    // TODO quite hacky, make this more robust
    node.remove_child(&node.first_child().unwrap()).unwrap();

    // Hack to replace emotes with their text version, needed because sometimes fighters have emotes in their name
    // TODO can this be made better somehow ?
    for node in node.query_selector_all("img").unwrap() {
        // TODO replace with stdweb stuff
        js! { @(no_return)
            var node = @{node};
            node.parentNode.replaceChild(document.createTextNode(node.alt || ""), node);
        }
    }

    get_text_content(node).and_then(|x| parse_message(&x, date))
}


pub fn get_waifu_messages() -> Vec<WaifuMessage> {
    let now: f64 = Date::now();

    query_all("[data-a-target='chat-line-message']").into_iter()
        .filter_map(|x| get_waifu_message(x, now))
        .collect()
}


pub fn observe_changes() {
    set_panic_hook();

    log!("Initializing...");

    let port = Port::connect("twitch_chat");

    let observer = MutationObserver::new(move |records, _| {
        let now: f64 = Date::now();

        let messages: Vec<WaifuMessage> = records.into_iter().filter_map(|record| {
            match record {
                MutationRecord::ChildList { inserted_nodes, .. } => Some(
                    inserted_nodes.into_iter()
                        .filter_map(|x| get_waifu_message(x, now))
                ),
                _ => None,
            }
        }).flat_map(|x| x).collect();

        if messages.len() != 0 {
            port.send_message(&messages);
        }
    });

    wait_until_defined(|| query("body"), move |body| {
        js! { @(no_return)
            @{body}.style.display = "none";
        }

        log!("Body hidden");
    });

    wait_until_defined(|| query("[data-a-target='chat-welcome-message']"), move |welcome| {
        observer.observe(&welcome.parent_node().unwrap(), MutationObserverInit {
            child_list: true,
            attributes: false,
            character_data: false,
            subtree: false,
            attribute_old_value: false,
            character_data_old_value: false,
            attribute_filter: None,
        }).unwrap();

        // TODO use observer.leak()
        std::mem::forget(observer);

        log!("Observer initialized");
    });
}


fn main() {
    stdweb::initialize();

    observe_changes();

    stdweb::event_loop();
}
