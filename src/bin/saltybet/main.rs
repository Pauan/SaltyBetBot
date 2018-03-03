#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;

use std::rc::Rc;
use std::cell::RefCell;
use salty_bet_bot::common::{parse_f64, parse_money, Port, create_tab, get_text_content, WaifuMessage, WaifuBetsOpen, to_input_element, get_value, click, get_storage, set_storage, query, query_all};
use salty_bet_bot::genetic::{BetStrategy};
use salty_bet_bot::record::{Record, Character, Winner, Mode};
use salty_bet_bot::simulation::{Bet, Simulation};
use stdweb::web::set_timeout;


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
pub struct Information {
    player_is_illuminati: bool,
    left_bettors_illuminati: f64,
    right_bettors_illuminati: f64,
    left_bettors_normal: f64,
    right_bettors_normal: f64,
    bet: Bet,
}


fn lookup_bet(state: &Rc<RefCell<State>>) {
    let mut state = state.borrow_mut();

    if !state.did_bet &&
       query("#betconfirm").is_none() {

        let current_balance = query("#balance")
            .and_then(get_text_content)
            .and_then(|x| parse_f64(x.as_str()));

        let wager_box = query("#wager").and_then(to_input_element);

        let left_button = query("#player1:enabled").and_then(to_input_element);

        let right_button = query("#player2:enabled").and_then(to_input_element);

        // TODO gross
        // TODO figure out a way to avoid the clone
        if let Some(open) = state.open.clone() {
        if let Some(current_balance) = current_balance {
        if let Some(wager_box) = wager_box {
        if let Some(left_button) = left_button {
        if let Some(right_button) = right_button {
            let left_name = get_value(&left_button);
            let right_name = get_value(&right_button);

            // TODO check the date as well ?
            if left_name == open.left &&
               right_name == open.right {

                state.did_bet = true;

                let mut simulation = &mut state.simulation;

                let bet = match open.mode {
                    Mode::Matchmaking => {
                        simulation.in_tournament = false;
                        simulation.sum = current_balance;
                        match simulation.matchmaking_strategy {
                            Some(ref a) => simulation.pick_winner(a, &open.tier, &open.left, &open.right),
                            None => Bet::None,
                        }
                    },
                    Mode::Tournament => {
                        simulation.in_tournament = true;
                        simulation.tournament_sum = current_balance;
                        match simulation.tournament_strategy {
                            Some(ref a) => simulation.pick_winner(a, &open.tier, &open.left, &open.right),
                            None => Bet::None,
                        }
                    },
                };

                log!("Betting: {:#?}", bet);

                match bet {
                    Bet::Left(amount) => {
                        wager_box.set_raw_value(&amount.to_string());
                        click(&left_button);
                    },
                    Bet::Right(amount) => {
                        wager_box.set_raw_value(&amount.to_string());
                        click(&right_button);
                    },
                    Bet::None => {},
                }

            } else {
                log!("Invalid state: {:#?} {:#?} {:#?}", open, left_name, right_name);
            }
        }}}}}
    }
}


fn lookup_information(state: &Rc<RefCell<State>>) {
    let status = query("#betstatus")
        .and_then(get_text_content)
        .map(|x| x == "Bets are locked until the next match.")
        .unwrap_or(false);

    if status &&
       query("#sbettors1 > span.redtext > span.counttext").is_some() &&
       query("#sbettors2 > span.bluetext > span.counttext").is_some() {

        let left_bettors_illuminati = query_all("#bettors1 > p.bettor-line > strong.goldtext").len() as f64;
        let right_bettors_illuminati = query_all("#bettors2 > p.bettor-line > strong.goldtext").len() as f64;

        let left_bettors_normal = query_all("#bettors1 > p.bettor-line > strong:not(.goldtext)").len() as f64;
        let right_bettors_normal = query_all("#bettors2 > p.bettor-line > strong:not(.goldtext)").len() as f64;

        let left_bet = query("#lastbet > span:first-of-type.redtext")
            .and_then(get_text_content)
            .and_then(|x| parse_money(x.as_str()));

        let right_bet = query("#lastbet > span:first-of-type.bluetext")
            .and_then(get_text_content)
            .and_then(|x| parse_money(x.as_str()));

        state.borrow_mut().information = Some(Information {
            // TODO handle the situation where the player is Illuminati
            player_is_illuminati: false,
            left_bettors_illuminati,
            right_bettors_illuminati,
            left_bettors_normal,
            right_bettors_normal,
            bet: match left_bet {
                Some(left) => match right_bet {
                    None => Bet::Left(left),
                    Some(_) => unreachable!(),
                },
                None => match right_bet {
                    Some(right) => Bet::Right(right),
                    None => Bet::None,
                },
            }
        });
    }
}


fn lookup(state: Rc<RefCell<State>>) {
    lookup_bet(&state);
    lookup_information(&state);
    set_timeout(|| lookup(state), 5000);
}


// TODO timer which prints an error message if it's been >5 hours since a successful match recording
// TODO refresh page when mode changes
pub fn observe_changes(state: Rc<RefCell<State>>) {
    let mut old_closed = None;
    let mut mode_switch = None;

    let port = Port::new("saltybet");

    std::mem::forget(port.listen(move |message| {
        let messages: Vec<WaifuMessage> = serde_json::from_str(&message).unwrap();

        for message in messages {
            match message {
                WaifuMessage::BetsOpen(open) => {
                    let mut state = state.borrow_mut();

                    state.did_bet = false;
                    state.open = Some(open);
                    old_closed = None;
                    mode_switch = None;
                    state.information = None;
                },

                WaifuMessage::BetsClosed(closed) => {
                    let mut state = state.borrow_mut();

                    mode_switch = None;
                    state.information = None;

                    match state.open {
                        Some(ref open) => {
                            let duration = closed.date - open.date;

                            if duration >= 0.0 &&
                               duration <= MAX_BET_TIME_LIMIT &&
                               open.left == closed.left.name &&
                               open.right == closed.right.name &&
                               old_closed.is_none() {
                                old_closed = Some(closed);
                                continue;

                            } else {
                                log!("Invalid messages: {:#?} {:#?}", open, closed);
                            }
                        },
                        None => {},
                    }

                    state.open = None;
                    old_closed = None;
                },

                WaifuMessage::ModeSwitch { date } => {
                    let state = state.borrow();

                    match state.open {
                        Some(ref open) => match old_closed {
                            Some(_) => {
                                if mode_switch.is_none() {
                                    mode_switch = Some(date);
                                    continue;

                                } else {
                                    log!("Invalid messages: {:#?} {:#?} {:#?}", open, mode_switch, message);
                                }
                            },
                            None => {
                                log!("Invalid messages: {:#?} {:#?}", open, message);
                            },
                        },
                        None => {},
                    }

                    // TODO should this also reset open and old_closed ?
                    mode_switch = None;
                },

                WaifuMessage::Winner(winner) => {
                    let record = {
                        let state = state.borrow();

                        match state.open {
                            Some(ref open) => match old_closed {
                                Some(ref mut closed) => match state.information {
                                    Some(ref information) => {
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

                                            // TODO figure out a way to avoid the clone
                                            let mut information = information.clone();

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
                                            Some(Record {
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
                                                date,
                                            })

                                        } else {
                                            log!("Invalid messages: {:#?} {:#?} {:#?} {:#?}", open, closed, information, winner);
                                            None
                                        }
                                    },
                                    None => {
                                        log!("Invalid messages: {:#?} {:#?} {:#?}", open, closed, winner);
                                        None
                                    },
                                },
                                None => {
                                    log!("Invalid messages: {:#?} {:#?}", open, winner);
                                    None
                                },
                            },
                            None => None,
                        }
                    };

                    let mut state = state.borrow_mut();

                    if let Some(record) = record {
                        log!("Record saved {:#?}", record);

                        // TODO figure out a way to avoid this clone
                        state.simulation.insert_record(record.clone());

                        state.matches.push(record);

                        set_storage("matches", &serde_json::to_string(&state.matches).unwrap());
                    }

                    state.did_bet = false;
                    state.open = None;
                    old_closed = None;
                    mode_switch = None;
                    state.information = None;
                },
            }
        }
    }));

    std::mem::forget(port);
}


pub struct State {
    did_bet: bool,
    open: Option<WaifuBetsOpen>,
    information: Option<Information>,
    simulation: Simulation<BetStrategy, BetStrategy>,
    matches: Vec<Record>,
}


fn main() {
    stdweb::initialize();

    log!("Initializing...");

    get_storage("matches", |matches| {
        let matches: Vec<Record> = match matches {
            Some(a) => serde_json::from_str(&a).unwrap(),
            None => vec![],
        };

        log!("Initialized {} records", matches.len());

        let mut simulation = Simulation::new();

        let matchmaking_strategy: BetStrategy = serde_json::from_str(include_str!("../../../strategies/matchmaking_strategy")).unwrap();
        let tournament_strategy: BetStrategy = serde_json::from_str(include_str!("../../../strategies/tournament_strategy")).unwrap();

        simulation.matchmaking_strategy = Some(matchmaking_strategy);
        simulation.tournament_strategy = Some(tournament_strategy);
        // TODO figure out a way to avoid the clone
        simulation.insert_records(matches.clone());

        let state = Rc::new(RefCell::new(State {
            did_bet: false,
            open: None,
            information: None,
            simulation: simulation,
            matches: matches,
        }));

        observe_changes(state.clone());
        lookup(state);
    });

    create_tab(|| {
        log!("Tab created");
    });

    stdweb::event_loop();
}
