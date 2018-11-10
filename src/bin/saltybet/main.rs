#![recursion_limit="128"]
#![feature(async_await, await_macro, futures_api)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate salty_bet_bot;
#[macro_use]
extern crate dominator;
#[macro_use]
extern crate futures_signals;

use std::cmp::Ordering;
use std::rc::Rc;
use std::cell::RefCell;
use salty_bet_bot::{wait_until_defined, parse_f64, parse_money, parse_name, Port, create_tab, get_text_content, WaifuMessage, WaifuBetsOpen, WaifuBetsClosed, to_input_element, get_value, click, query, query_all, records_get_all, records_insert, money, display_odds, MAX_MATCH_TIME_LIMIT, set_panic_hook};
use algorithm::record::{Record, Character, Winner, Mode, Tier};
use algorithm::simulation::{Bet, Simulation, Simulator, Strategy};
use algorithm::strategy::{MATCHMAKING_STRATEGY, TOURNAMENT_STRATEGY, AllInStrategy, CustomStrategy, winrates, average_odds, needed_odds, expected_profits};
use stdweb::{spawn_local, unwrap_future};
use stdweb::web::{set_timeout, NodeList};
use stdweb::web::error::Error;
use futures_util::stream::StreamExt;
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use dominator::Dom;


const SHOULD_BET: bool = true;

// 10 minutes
// TODO is this high enough ?
const MAX_BET_TIME_LIMIT: f64 = 1000.0 * 60.0 * 10.0;


#[derive(Debug, Clone)]
pub struct Information {
    left_bettors_illuminati: f64,
    right_bettors_illuminati: f64,
    left_bettors_normal: f64,
    right_bettors_normal: f64,
    bet: Bet,
    sum: f64,
}


fn lookup_bet(state: &Rc<RefCell<State>>) {
    let mut state = state.borrow_mut();

    if !state.did_bet &&
       query("#betconfirm").is_none() {

        let current_balance = query("#balance")
            .and_then(get_text_content)
            .and_then(|x| parse_f64(&x));

        let wager_box = query("#wager").and_then(to_input_element);

        let left_button = query("#player1:enabled").and_then(to_input_element);

        let right_button = query("#player2:enabled").and_then(to_input_element);

        let in_tournament = query("#balance.purpletext").is_some();

        // TODO gross
        // TODO figure out a way to avoid the clone
        if let Some(open) = state.open.clone() {
        if let Some(current_balance) = current_balance {
        if let Some(wager_box) = wager_box {
        if let Some(left_button) = left_button {
        if let Some(right_button) = right_button {
            let left_name = get_value(&left_button);
            let right_name = get_value(&right_button);

            let correct_mode = match open.mode {
                Mode::Matchmaking => !in_tournament,
                Mode::Tournament => in_tournament,
            };

            // TODO check the date as well ?
            if left_name == open.left &&
               right_name == open.right &&
               correct_mode {

                state.did_bet = true;

                let bet = match open.mode {
                    Mode::Matchmaking => {
                        // Always bet in tournament mode
                        if !SHOULD_BET {
                            Bet::None

                        } else {
                            let simulation = &mut state.simulation;

                            simulation.in_tournament = false;
                            simulation.sum = current_balance;

                            match simulation.matchmaking_strategy {
                                Some(ref a) => simulation.pick_winner(a, &open.tier, &open.left, &open.right),
                                None => Bet::None,
                            }
                        }
                    },

                    Mode::Tournament => {
                        let simulation = &mut state.simulation;

                        simulation.in_tournament = true;
                        simulation.tournament_sum = current_balance;

                        match simulation.tournament_strategy {
                            Some(ref a) => simulation.pick_winner(a, &open.tier, &open.left, &open.right),
                            None => Bet::None,
                        }
                    },
                };

                state.update_info_container(&open.mode, &open.tier, &open.left, &open.right);

                match bet {
                    Bet::Left(amount) => {
                        if !amount.is_nan() {
                            wager_box.set_raw_value(&amount.to_string());
                            click(&left_button);
                            return;
                        }
                    },
                    Bet::Right(amount) => {
                        if !amount.is_nan() {
                            wager_box.set_raw_value(&amount.to_string());
                            click(&right_button);
                            return;
                        }
                    },
                    Bet::None => {
                        return;
                    },
                }
            }

            server_log!("Invalid state: {:#?} {:#?} {:#?} {:#?} {:#?}", current_balance, open, left_name, right_name, in_tournament);
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

        // TODO a bit of code duplication with lookup_bet
        let current_balance = query("#balance")
            .and_then(get_text_content)
            .and_then(|x| parse_f64(&x));

        // TODO detect whether the player is Illuminati or not ?
        let name = query("#header span.navbar-text")
            .and_then(get_text_content)
            .and_then(|x| parse_name(&x));

        if let Some(current_balance) = current_balance {
            if let Some(name) = name {
                let mut matched_left = false;
                let mut matched_right = false;

                fn filtered_len(list: NodeList, name: &str, matched: &mut bool) -> f64 {
                    let mut len = 0.0;

                    for bettor in list {
                        if let Some(bettor) = get_text_content(bettor) {
                            if bettor != name {
                                len += 1.0;

                            } else {
                                assert!(!*matched);
                                *matched = true;
                            }
                        }
                    }

                    len
                }

                let left_bettors_illuminati = filtered_len(query_all("#bettors1 > p.bettor-line > strong.goldtext"), &name, &mut matched_left);
                let right_bettors_illuminati = filtered_len(query_all("#bettors2 > p.bettor-line > strong.goldtext"), &name, &mut matched_right);

                let left_bettors_normal = filtered_len(query_all("#bettors1 > p.bettor-line > strong:not(.goldtext)"), &name, &mut matched_left);
                let right_bettors_normal = filtered_len(query_all("#bettors2 > p.bettor-line > strong:not(.goldtext)"), &name, &mut matched_right);

                let left_bet = query("#lastbet > span:first-of-type.redtext")
                    .and_then(get_text_content)
                    .and_then(|x| parse_money(&x));

                let right_bet = query("#lastbet > span:first-of-type.bluetext")
                    .and_then(get_text_content)
                    .and_then(|x| parse_money(&x));

                state.borrow_mut().information = Some(Information {
                    left_bettors_illuminati,
                    right_bettors_illuminati,
                    left_bettors_normal,
                    right_bettors_normal,
                    bet: match left_bet {
                        Some(left) => match right_bet {
                            None => {
                                assert!(matched_left);
                                assert!(!matched_right);
                                Bet::Left(left)
                            },
                            Some(_) => unreachable!(),
                        },
                        None => match right_bet {
                            Some(right) => {
                                assert!(!matched_left);
                                assert!(matched_right);
                                Bet::Right(right)
                            },
                            None => {
                                assert!(!matched_left);
                                assert!(!matched_right);
                                Bet::None
                            },
                        },
                    },
                    sum: current_balance,
                });

            }  else {
                server_log!("Unknown name");
            }

        }  else {
            server_log!("Unknown current balance");
        }
    }
}


fn reload_page() {
    js! { @(no_return)
        location.reload();
    }
}


// TODO timer which prints an error message if it's been >5 hours since a successful match recording
pub fn observe_changes(state: Rc<RefCell<State>>, port: Port) {
    let mut old_closed: Option<WaifuBetsClosed> = None;
    let mut mode_switch: Option<f64> = None;

    let mut process_messages = move |messages: Vec<WaifuMessage>| {
        for message in messages {
            match message {
                WaifuMessage::BetsOpen(open) => {
                    let mut state = state.borrow_mut();

                    state.clear_info_container();

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
                                server_log!("Invalid messages: {:#?} {:#?}", open, closed);
                            }
                        },
                        None => {},
                    }

                    state.clear_info_container();

                    state.open = None;
                    old_closed = None;
                },

                WaifuMessage::ModeSwitch { date, is_exhibition } => {
                    // When exhibition mode starts, reload the page, just in case something screwed up on saltybet.com
                    // TODO reload it when the first exhibition match begins, rather than after 15 minutes
                    if is_exhibition {
                        set_timeout(|| {
                            reload_page();
                        // 15 minutes
                        }, 900000);
                    }

                    let state = state.borrow();

                    match state.open {
                        Some(ref open) => match old_closed {
                            Some(_) => {
                                if mode_switch.is_none() {
                                    mode_switch = Some(date);
                                    continue;

                                } else {
                                    server_log!("Invalid messages: {:#?} {:#?} {:#?}", open, mode_switch, message);
                                }
                            },
                            None => {
                                server_log!("Invalid messages: {:#?} {:#?}", open, message);
                            },
                        },
                        None => {},
                    }

                    state.clear_info_container();

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

                                        let is_winner_correct = match winner.side {
                                            Winner::Left => winner.name == closed.left.name,
                                            Winner::Right => winner.name == closed.right.name,
                                        };

                                        if is_winner_correct &&
                                           duration >= 0.0 &&
                                           duration <= MAX_MATCH_TIME_LIMIT {

                                            // TODO figure out a way to avoid the clone
                                            let information = information.clone();

                                            match information.bet {
                                                Bet::Left(amount) => {
                                                    closed.left.bet_amount -= amount;
                                                },
                                                Bet::Right(amount) => {
                                                    closed.right.bet_amount -= amount;
                                                },
                                                Bet::None => {},
                                            }

                                            if closed.left.bet_amount > 0.0 &&
                                               closed.right.bet_amount > 0.0 &&
                                               (information.left_bettors_illuminati + information.left_bettors_normal) > 0.0 &&
                                               (information.right_bettors_illuminati + information.right_bettors_normal) > 0.0 {

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
                                                    sum: information.sum,
                                                })

                                            } else {
                                                server_log!("Invalid messages: {:#?} {:#?} {:#?} {:#?}", open, closed, information, winner);
                                                None
                                            }

                                        } else {
                                            server_log!("Invalid messages: {:#?} {:#?} {:#?} {:#?}", open, closed, information, winner);
                                            None
                                        }
                                    },
                                    None => {
                                        server_log!("Invalid messages: {:#?} {:#?} {:#?}", open, closed, winner);
                                        None
                                    },
                                },
                                None => {
                                    server_log!("Invalid messages: {:#?} {:#?}", open, winner);
                                    None
                                },
                            },
                            None => None,
                        }
                    };

                    let mut state = state.borrow_mut();

                    if let Some(record) = record {
                        if let Mode::Matchmaking = record.mode {
                            state.simulation.insert_sum(record.sum);
                        }

                        // TODO figure out a way to avoid this clone
                        state.simulation.insert_record(&record);

                        // TODO figure out a way to avoid this clone
                        spawn_local(unwrap_future(records_insert(vec![record.clone()])));

                        state.records.push(record);
                    }

                    state.clear_info_container();

                    state.did_bet = false;
                    state.open = None;
                    old_closed = None;
                    mode_switch = None;
                    state.information = None;
                },
            }
        }
    };

    spawn_local(
        port.messages().for_each(move |messages| {
            process_messages(messages);
            async {}
        })
    );
}


pub struct State {
    did_bet: bool,
    open: Option<WaifuBetsOpen>,
    information: Option<Information>,
    simulation: Simulation<CustomStrategy, AllInStrategy>,
    records: Vec<Record>,
    info_container: Rc<InfoContainer>,
}

impl State {
    fn update_info_container(&self, mode: &Mode, tier: &Tier, left: &str, right: &str) {
        self.info_container.clear();

        // TODO avoid the to_string somehow
        self.info_container.left.name.set(Some(left.to_string()));
        self.info_container.right.name.set(Some(right.to_string()));

        let (left_winrate, right_winrate) = winrates(&self.simulation, left, right);
        self.info_container.left.winrate.set(Some(left_winrate));
        self.info_container.right.winrate.set(Some(right_winrate));

        self.info_container.left.matches_len.set(Some(self.simulation.matches_len(left)));
        self.info_container.right.matches_len.set(Some(self.simulation.matches_len(right)));

        let specific_matches = self.simulation.specific_matches_len(left, right);
        self.info_container.left.specific_matches_len.set(Some(specific_matches));
        self.info_container.right.specific_matches_len.set(Some(specific_matches));

        let (left_needed_odds, right_needed_odds) = needed_odds(&self.simulation, left, right);
        self.info_container.left.needed_odds.set(Some(left_needed_odds));
        self.info_container.right.needed_odds.set(Some(right_needed_odds));

        let (left_bet, right_bet) = match *mode {
            Mode::Matchmaking => self.simulation.matchmaking_strategy.as_ref().unwrap().bet_amount(&self.simulation, tier, left, right),
            Mode::Tournament => self.simulation.tournament_strategy.as_ref().unwrap().bet_amount(&self.simulation, tier, left, right),
        };

        self.info_container.left.bet_amount.set(Some(left_bet));
        self.info_container.right.bet_amount.set(Some(right_bet));

        let (left_odds, right_odds) = average_odds(&self.simulation, left, right, left_bet, right_bet);
        self.info_container.left.odds.set(Some(left_odds));
        self.info_container.right.odds.set(Some(right_odds));

        let (left_profit, right_profit) = expected_profits(&self.simulation, left, right, left_bet, right_bet);
        self.info_container.left.expected_profit.set(Some(left_profit));
        self.info_container.right.expected_profit.set(Some(right_profit));
    }

    fn clear_info_container(&self) {
        self.info_container.clear();
    }
}


struct InfoSide {
    name: Mutable<Option<String>>,
    expected_profit: Mutable<Option<f64>>,
    bet_amount: Mutable<Option<f64>>,
    odds: Mutable<Option<f64>>,
    needed_odds: Mutable<Option<f64>>,
    matches_len: Mutable<Option<usize>>,
    specific_matches_len: Mutable<Option<usize>>,
    winrate: Mutable<Option<f64>>,
}

impl InfoSide {
    pub fn new() -> Self {
        Self {
            name: Mutable::new(None),
            expected_profit: Mutable::new(None),
            bet_amount: Mutable::new(None),
            odds: Mutable::new(None),
            needed_odds: Mutable::new(None),
            matches_len: Mutable::new(None),
            specific_matches_len: Mutable::new(None),
            winrate: Mutable::new(None),
        }
    }

    fn clear(&self) {
        self.name.set(None);
        self.expected_profit.set(None);
        self.bet_amount.set(None);
        self.odds.set(None);
        self.needed_odds.set(None);
        self.matches_len.set(None);
        self.specific_matches_len.set(None);
        self.winrate.set(None);
    }

    fn render(&self, other: &Self, color: &str) -> Dom {
        fn info_bar<A, S, F>(this: &Mutable<Option<A>>, other: S, mut f: F) -> Dom
            where A: Copy + PartialOrd + 'static,
                  S: Signal<Item = Option<A>> + 'static,
                  F: FnMut(A) -> String + 'static {
            lazy_static! {
                static ref CLASS: String = class! {
                    .style("display", "flex")
                    .style("align-items", "center")
                    .style("padding", "0px 7px")
                    .style("color", "white")
                };
            }

            html!("div", {
                .class(&*CLASS)

                .style_signal("color", map_ref! {
                    let this = this.signal(),
                    let other = other => {
                        let cmp = this.and_then(|this| {
                            other.and_then(|other| {
                                this.partial_cmp(&other)
                            })
                        }).unwrap_or(Ordering::Equal);

                        // TODO different color if it's None ?
                        match cmp {
                            Ordering::Equal => "white",
                            Ordering::Less => "lightcoral",
                            Ordering::Greater => "limegreen",
                        }
                    }
                })

                .text_signal(this.signal().map(move |x| {
                    // TODO use RefFn
                    x.map(|x| f(x)).unwrap_or_else(|| "".to_string())
                }))
            })
        }

        lazy_static! {
            static ref CLASS: String = class! {
                .style("flex", "1")
                .style("border-right", "1px solid #6441a5")
                .style("margin-right", "-1px")
            };

            static ref CLASS_TEXT: String = class! {
                .style("display", "flex")
                .style("align-items", "center")
                .style("justify-content", "center")
                .style("height", "50px")
                .style("padding", "5px")
                .style("color", "white")
                .style("font-size", "15px")
                .style("box-shadow", "hsla(0, 0%, 0%, 0.5) 0px -1px 2px inset")
                .style("margin-bottom", "2px")
            };
        }

        html!("div", {
            .class(&*CLASS)
            .children(&mut [
                html!("div", {
                    .class(&*CLASS_TEXT)
                    .style("background-color", color)
                    .text_signal(self.name.signal_cloned().map(|x| {
                        // TODO use RefFn
                        x.unwrap_or_else(|| "".to_string())
                    }))
                }),

                info_bar(&self.matches_len, other.matches_len.signal(), |x| {
                    format!("Number of past matches (in general): {}", x)
                }),

                info_bar(&self.specific_matches_len, always(Some(0)), |x| {
                    format!("Number of past matches (in specific): {}", x)
                }),

                info_bar(&self.winrate, other.winrate.signal(), |x| {
                    format!("Winrate: {}%", x * 100.0)
                }),

                info_bar(&self.needed_odds, self.odds.signal(), |x| {
                    format!("Needed odds: {}", display_odds(x))
                }),

                info_bar(&self.odds, self.needed_odds.signal(), |x| {
                    format!("Average odds: {}", display_odds(x))
                }),

                info_bar(&self.bet_amount, other.bet_amount.signal(), |x| {
                    format!("Simulated bet amount: {}", money(x))
                }),

                info_bar(&self.expected_profit, other.expected_profit.signal(), |x| {
                    format!("Simulated expected profit: {}", money(x))
                }),
            ])
        })
    }
}

struct InfoContainer {
    left: InfoSide,
    right: InfoSide,
}

impl InfoContainer {
    fn new() -> Self {
        Self {
            left: InfoSide::new(),
            right: InfoSide::new(),
        }
    }

    fn render(&self) -> Dom {
        lazy_static! {
            static ref CLASS: String = class! {
                .style("display", "flex")
                .style("background-color", "#201d2b") // rgba(100, 65, 165, 0.09)
                .style("border-bottom", "1px solid #6441a5")
                .style("width", "100%")
                .style("height", "100%")
                .style("position", "absolute")
                .style("left", "0px")
                .style("top", "0px")
            };
        }

        html!("div", {
            .class(&*CLASS)
            .children(&mut [
                self.left.render(&self.right, "rgb(176, 68, 68)"),
                self.right.render(&self.left, "rgb(52, 158, 255)"),
            ])
        })
    }

    fn clear(&self) {
        self.left.clear();
        self.right.clear();
    }
}


/*fn migrate_records(mut records: Vec<Record>) -> Vec<Record> {
    records = records.into_iter().filter(|record| {
        record.left.bet_amount > 0.0 &&
        record.right.bet_amount > 0.0 &&
        (record.left.illuminati_bettors + record.left.normal_bettors) > 0.0 &&
        (record.right.illuminati_bettors + record.right.normal_bettors) > 0.0
    }).collect();

    set_storage("matches", &serde_json::to_string(&records).unwrap());

    records
}*/


async fn initialize_state(container: Rc<InfoContainer>) -> Result<(), Error> {
    let port = Port::connect("saltybet");

    let records = await!(records_get_all())?;
    //let matches = migrate_records(matches);

    log!("Initialized {} records", records.len());

    let mut simulation = Simulation::new();

    /*let matchmaking_strategy: BetStrategy = serde_json::from_str(include_str!("../../../strategies/matchmaking_strategy")).unwrap();
    let tournament_strategy: BetStrategy = serde_json::from_str(include_str!("../../../strategies/tournament_strategy")).unwrap();

    simulation.matchmaking_strategy = Some(matchmaking_strategy);
    simulation.tournament_strategy = Some(tournament_strategy);*/

    simulation.matchmaking_strategy = Some(MATCHMAKING_STRATEGY);
    simulation.tournament_strategy = Some(TOURNAMENT_STRATEGY);

    for record in records.iter() {
        if let Mode::Matchmaking = record.mode {
            simulation.insert_sum(record.sum);
        }

        simulation.insert_record(record);
    }

    let state = Rc::new(RefCell::new(State {
        did_bet: false,
        open: None,
        information: None,
        simulation: simulation,
        records: records,
        info_container: container,
    }));

    observe_changes(state.clone(), port);

    fn loop_bet(state: Rc<RefCell<State>>) {
        lookup_bet(&state);
        set_timeout(|| loop_bet(state), 500);
    }

    fn loop_information(state: Rc<RefCell<State>>) {
        lookup_information(&state);
        set_timeout(|| loop_information(state), 10000);
    }

    loop_bet(state.clone());
    loop_information(state);

    Ok(())
}


async fn initialize_tab() -> Result<(), Error> {
    await!(create_tab())?;

    log!("Tab created");

    Ok(())
}


fn main() {
    stdweb::initialize();

    set_panic_hook();

    log!("Initializing...");

    let container = Rc::new(InfoContainer::new());

    wait_until_defined(|| query("#stream"), clone!(container => move |stream| {
        dominator::append_dom(&stream, container.render());
        log!("Information initialized");
    }));

    wait_until_defined(|| query("#iframeplayer"), move |video| {
        // TODO hacky
        js! { @(no_return)
            var video = @{video};
            video.parentNode.removeChild(video);
        }

        log!("Video hidden");
    });

    wait_until_defined(|| query("#chat-frame-stream"), move |chat| {
       // TODO hacky
        js! { @(no_return)
            var chat = @{chat};
            chat.style.display = "none";
        }

        log!("Chat hidden");
    });

    /*wait_until_defined(|| query("#sbettorswrapper"), move |bettors| {
        js! { @(no_return)
            @{bettors}.style.display = "none";
        }

        log!("Bettors hidden");
    });*/

    spawn_local(unwrap_future(initialize_state(container)));

    spawn_local(unwrap_future(initialize_tab()));

    // Reloads the page every 24 hours, just in case something screwed up on saltybet.com
    // Normally this doesn't happen, because it reloads the page at the start of exhibitions
    // TODO is 24 hours too long ? can it be made shorter ? should it be made shorter ?
    set_timeout(|| {
        reload_page();
    // 24 hours
    }, 86400000);

    stdweb::event_loop();
}
