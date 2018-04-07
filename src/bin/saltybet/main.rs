#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;

use std::cmp::Ordering;
use std::rc::Rc;
use std::cell::RefCell;
use salty_bet_bot::{wait_until_defined, parse_f64, parse_money, Port, create_tab, get_text_content, WaifuMessage, WaifuBetsOpen, WaifuBetsClosed, to_input_element, get_value, click, get_storage, set_storage, query, query_all};
use algorithm::record::{Record, Character, Winner, Mode, Tier};
use algorithm::simulation::{Bet, Simulation, Simulator};
use algorithm::strategy::{EarningsStrategy, AllInStrategy};
use stdweb::web::{document, set_timeout, Element, INode};


const SHOULD_BET: bool = true;

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
                            let mut simulation = &mut state.simulation;

                            simulation.in_tournament = false;
                            simulation.sum = current_balance;

                            match simulation.matchmaking_strategy {
                                Some(ref a) => simulation.pick_winner(a, &open.tier, &open.left, &open.right),
                                None => Bet::None,
                            }
                        }
                    },

                    Mode::Tournament => {
                        let mut simulation = &mut state.simulation;

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

            log!("Invalid state: {:#?} {:#?} {:#?} {:#?} {:#?}", current_balance, open, left_name, right_name, in_tournament);
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
            .and_then(|x| parse_money(&x));

        let right_bet = query("#lastbet > span:first-of-type.bluetext")
            .and_then(get_text_content)
            .and_then(|x| parse_money(&x));

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


// TODO timer which prints an error message if it's been >5 hours since a successful match recording
// TODO refresh page when mode changes
pub fn observe_changes(state: Rc<RefCell<State>>) {
    let mut old_closed: Option<WaifuBetsClosed> = None;
    let mut mode_switch: Option<f64> = None;

    let port = Port::new("saltybet");

    std::mem::forget(port.listen(move |message| {
        let messages: Vec<WaifuMessage> = serde_json::from_str(&message).unwrap();

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
                                log!("Invalid messages: {:#?} {:#?}", open, closed);
                            }
                        },
                        None => {},
                    }

                    state.clear_info_container();

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
                                                })

                                            } else {
                                                log!("Invalid messages: {:#?} {:#?} {:#?} {:#?}", open, closed, information, winner);
                                                None
                                            }

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
                        // TODO figure out a way to avoid this clone
                        state.simulation.insert_record(record.clone());

                        state.matches.push(record);

                        set_storage("matches", &serde_json::to_string(&state.matches).unwrap());
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
    }));

    std::mem::forget(port);
}


pub struct State {
    did_bet: bool,
    open: Option<WaifuBetsOpen>,
    information: Option<Information>,
    simulation: Simulation<EarningsStrategy, AllInStrategy>,
    matches: Vec<Record>,
    info_container: Rc<InfoContainer>,
}

impl State {
    fn update_info_container(&self, mode: &Mode, tier: &Tier, left: &str, right: &str) {
        self.info_container.clear();

        self.info_container.left.set_name(left);
        self.info_container.right.set_name(right);

        let left_winrate = self.simulation.winrate(left);
        let right_winrate = self.simulation.winrate(right);
        self.info_container.left.set_winrate(left_winrate, left_winrate.partial_cmp(&right_winrate).unwrap_or(Ordering::Equal));
        self.info_container.right.set_winrate(right_winrate, right_winrate.partial_cmp(&left_winrate).unwrap_or(Ordering::Equal));

        let left_matches = self.simulation.matches_len(left);
        let right_matches = self.simulation.matches_len(right);
        self.info_container.left.set_matches_len(left_matches, left_matches.partial_cmp(&right_matches).unwrap_or(Ordering::Equal));
        self.info_container.right.set_matches_len(right_matches, right_matches.partial_cmp(&left_matches).unwrap_or(Ordering::Equal));

        let specific_matches = self.simulation.specific_matches_len(left, right);
        self.info_container.left.set_specific_matches_len(specific_matches, Ordering::Equal);
        self.info_container.right.set_specific_matches_len(specific_matches, Ordering::Equal);

        match *mode {
            Mode::Matchmaking => {
                let (left_profit, right_profit) = EarningsStrategy.expected_profits(&self.simulation, tier, left, right);
                self.info_container.left.set_expected_profit(left_profit, left_profit.partial_cmp(&right_profit).unwrap_or(Ordering::Equal));
                self.info_container.right.set_expected_profit(right_profit, right_profit.partial_cmp(&left_profit).unwrap_or(Ordering::Equal));
            },
            Mode::Tournament => {},
        }
    }

    fn clear_info_container(&self) {
        self.info_container.clear();
    }
}


struct InfoBar {
    pub element: Element,
}

impl InfoBar {
    pub fn new() -> Self {
        let element = document().create_element("div").unwrap();

        js! { @(no_return)
            var element = @{&element};
            element.style.display = "flex";
            element.style.alignItems = "center";
            element.style.padding = "0px 5px";
            element.style.color = "black";
        }

        InfoBar {
            element,
        }
    }

    pub fn set(&self, text: &str) {
        self.element.set_text_content(text);
    }

    pub fn set_color(&self, cmp: Ordering) {
        let color = match cmp {
            Ordering::Equal => "black",
            Ordering::Less => "lightcoral",
            Ordering::Greater => "limegreen",
        };

        js! { @(no_return)
            @{&self.element}.style.color = @{color};
        }
    }
}

struct InfoSide {
    pub element: Element,
    name: Element,
    expected_profit: InfoBar,
    matches_len: InfoBar,
    specific_matches_len: InfoBar,
    winrate: InfoBar,
}

impl InfoSide {
    pub fn new(color: &str) -> Self {
        let element = document().create_element("div").unwrap();

        js! { @(no_return)
            var element = @{&element};
            element.style.flex = "1";
            element.style.border = "1px solid black";
            element.style.borderRight = "none";
            element.style.borderLeftColor = "rgb(100, 65, 165)";
        }

        let name = document().create_element("div").unwrap();

        js! { @(no_return)
            var name = @{&name};
            name.style.display = "flex";
            name.style.alignItems = "center";
            name.style.justifyContent = "center";
            name.style.height = "50px";
            name.style.padding = "5px";
            name.style.color = "white";
            name.style.fontSize = "15px";
            name.style.borderBottom = "2px solid black";
            name.style.boxShadow = "0px 0px 5px black";
            name.style.marginBottom = "5px";
            name.style.backgroundColor = @{color};
        }

        element.append_child(&name);

        let expected_profit = InfoBar::new();
        element.append_child(&expected_profit.element);

        let winrate = InfoBar::new();
        element.append_child(&winrate.element);

        let matches_len = InfoBar::new();
        element.append_child(&matches_len.element);

        let specific_matches_len = InfoBar::new();
        element.append_child(&specific_matches_len.element);

        Self {
            element,
            name,
            expected_profit,
            matches_len,
            specific_matches_len,
            winrate
        }
    }

    pub fn set_name(&self, name: &str) {
        self.name.set_text_content(name);
    }

    pub fn set_expected_profit(&self, profits: f64, cmp: Ordering) {
        self.expected_profit.set_color(cmp);
        self.expected_profit.set(&format!("Expected profit: ${}", profits.round()));
    }

    pub fn set_matches_len(&self, len: usize, cmp: Ordering) {
        self.matches_len.set_color(cmp);
        self.matches_len.set(&format!("Number of past matches (in general): {}", len));
    }

    pub fn set_specific_matches_len(&self, len: usize, cmp: Ordering) {
        self.specific_matches_len.set_color(cmp);
        self.specific_matches_len.set(&format!("Number of past matches (in specific): {}", len));
    }

    pub fn set_winrate(&self, percentage: f64, cmp: Ordering) {
        self.winrate.set_color(cmp);
        self.winrate.set(&format!("Winrate: {}%", percentage * 100.0));
    }

    pub fn clear(&self) {
        self.name.set_text_content("");
        self.expected_profit.set("");
        self.matches_len.set("");
        self.specific_matches_len.set("");
        self.winrate.set("");
    }
}

struct InfoContainer {
    pub element: Element,
    pub left: InfoSide,
    pub right: InfoSide,
}

impl InfoContainer {
    pub fn new() -> Self {
        let element = document().create_element("div").unwrap();

        js! { @(no_return)
            var element = @{&element};
            element.style.display = "flex";
            element.style.backgroundColor = "#f2f2f2"; // rgba(100, 65, 165, 0.09)
            element.style.width = "100%";
            element.style.height = "100%";
            element.style.position = "absolute";
            element.style.left = "0px";
            element.style.top = "0px";
        }

        let left = InfoSide::new("rgb(176, 68, 68)");
        element.append_child(&left.element);

        let right = InfoSide::new("rgb(52, 158, 255)");
        element.append_child(&right.element);

        Self {
            element,
            left,
            right,
        }
    }

    pub fn clear(&self) {
        self.left.clear();
        self.right.clear();
    }
}


fn migrate_records(mut records: Vec<Record>) -> Vec<Record> {
    records = records.into_iter().filter(|record| {
        record.left.bet_amount > 0.0 &&
        record.right.bet_amount > 0.0 &&
        (record.left.illuminati_bettors + record.left.normal_bettors) > 0.0 &&
        (record.right.illuminati_bettors + record.right.normal_bettors) > 0.0
    }).collect();

    set_storage("matches", &serde_json::to_string(&records).unwrap());

    records
}


fn main() {
    stdweb::initialize();

    log!("Initializing...");

    let container = Rc::new(InfoContainer::new());

    {
        let container = container.clone();

        wait_until_defined(|| query("#stream"), move |stream| {
            stream.append_child(&container.element);
        });
    }

    wait_until_defined(|| query("#iframeplayer"), move |video| {
        // TODO hacky
        js! { @(no_return)
            var video = @{video};
            video.parentNode.removeChild(video);
        }

        log!("Video hidden");
    });

    /*wait_until_defined(|| query("#chat-frame-stream"), move |chat| {
       // TODO hacky
        js! { @(no_return)
            var chat = @{chat};
            chat.parentNode.removeChild(chat);
        }

        log!("Chat hidden");
    });

    wait_until_defined(|| query("#sbettorswrapper"), move |bettors| {
        js! { @(no_return)
            @{bettors}.style.display = "none";
        }

        log!("Bettors hidden");
    });*/

    get_storage("matches", move |matches| {
        let matches: Vec<Record> = match matches {
            Some(a) => serde_json::from_str(&a).unwrap(),
            None => vec![],
        };

        //let matches = migrate_records(matches);

        log!("Initialized {} records", matches.len());

        let mut simulation = Simulation::new();

        /*let matchmaking_strategy: BetStrategy = serde_json::from_str(include_str!("../../../strategies/matchmaking_strategy")).unwrap();
        let tournament_strategy: BetStrategy = serde_json::from_str(include_str!("../../../strategies/tournament_strategy")).unwrap();

        simulation.matchmaking_strategy = Some(matchmaking_strategy);
        simulation.tournament_strategy = Some(tournament_strategy);*/

        simulation.matchmaking_strategy = Some(EarningsStrategy);
        simulation.tournament_strategy = Some(AllInStrategy);

        // TODO figure out a way to avoid the clone
        simulation.insert_records(matches.clone());

        let state = Rc::new(RefCell::new(State {
            did_bet: false,
            open: None,
            information: None,
            simulation: simulation,
            matches: matches,
            info_container: container,
        }));

        observe_changes(state.clone());

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
    });

    create_tab(|| {
        log!("Tab created");
    });

    stdweb::event_loop();
}
