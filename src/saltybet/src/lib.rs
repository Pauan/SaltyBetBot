use std::cmp::Ordering;
use std::rc::Rc;
use std::cell::RefCell;
use salty_bet_bot::{server_log, decimal, spawn, wait_until_defined, parse_f64, parse_money, parse_name, ClientPort, get_text_content, to_input_element, get_value, click, query, query_all, money, display_odds, get_extension_url, reload_page, log, NodeListIter};
use salty_bet_bot::api::{records_get_all, records_insert, MAX_MATCH_TIME_LIMIT, WaifuMessage, WaifuBetsOpen, WaifuBetsClosed};
use algorithm::record::{Record, Character, Winner, Mode, Tier};
use algorithm::simulation::{Bet, Simulation, Simulator, Strategy, Elo};
use algorithm::strategy::{MATCHMAKING_STRATEGY, TOURNAMENT_STRATEGY, CustomStrategy, winrates, average_odds, needed_odds, expected_profits, bettors};
use futures_core::Stream;
use futures_util::stream::StreamExt;
use futures_signals::map_ref;
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use dominator::{Dom, HIGHEST_ZINDEX, clone, stylesheet, html, events, class};
use gloo_timers::callback::{Timeout, Interval};
use web_sys::{Node, Element, NodeList};
use lazy_static::lazy_static;
use wasm_bindgen::prelude::*;


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
    fn lookup(state: &mut State) -> Option<()> {
        if !state.did_bet &&
           query("#betconfirm").is_none() {

            if let Some(_) = state.closed {
                return None;
            }

            if let Some(_) = state.information {
                return None;
            }

            let open = state.open.as_ref()?;

            let current_balance = query("#balance")
                .and_then(|x| get_text_content(&x))
                .and_then(|x| parse_f64(&x))?;

            let wager_box = query("#wager").and_then(to_input_element)?;

            let left_button = query("#player1:enabled").and_then(to_input_element)?;

            let right_button = query("#player2:enabled").and_then(to_input_element)?;

            let left_info_name = query("#sbettors1 > span.redtext > strong")
                .and_then(|x| get_text_content(&x))?;

            let right_info_name = query("#sbettors2 > span.bluetext > strong")
                .and_then(|x| get_text_content(&x))?;

            // TODO is the `tournament-note` correct ?
            let in_tournament = query("#balance.purpletext").is_some() || query("#tournament-note").is_some();

            let left_name = get_value(&left_button);
            let right_name = get_value(&right_button);

            let correct_mode = match open.mode {
                Mode::Matchmaking => !in_tournament,
                Mode::Tournament => in_tournament,
            };

            // TODO check the date as well ?
            if left_name == open.left &&
               right_name == open.right &&
               left_info_name == open.left &&
               right_info_name == open.right &&
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
                                Some(ref a) => simulation.pick_winner(a, &open.tier, &open.left, &open.right, open.date),
                                None => Bet::None,
                            }
                        }
                    },

                    Mode::Tournament => {
                        let simulation = &mut state.simulation;

                        simulation.in_tournament = true;
                        simulation.tournament_sum = current_balance;

                        match simulation.tournament_strategy {
                            Some(ref a) => simulation.pick_winner(a, &open.tier, &open.left, &open.right, open.date),
                            None => Bet::None,
                        }
                    },
                };

                state.update_info_container(&open.mode, &open.tier, &open.left, &open.right, open.date);

                match bet {
                    Bet::Left(amount) => {
                        if !amount.is_nan() {
                            wager_box.set_value(&amount.to_string());
                            click(&left_button);
                            return Some(());
                        }
                    },
                    Bet::Right(amount) => {
                        if !amount.is_nan() {
                            wager_box.set_value(&amount.to_string());
                            click(&right_button);
                            return Some(());
                        }
                    },
                    Bet::None => {
                        return Some(());
                    },
                }
            }

            server_log!("Invalid state: {:#?} {:#?} {:#?} {:#?} {:#?}", current_balance, open, left_name, right_name, in_tournament);
        }

        None
    }

    lookup(&mut state.borrow_mut());
}


macro_rules! unwrap_log {
    ($e:expr, $($message:tt)+) => {
        match $e {
            Some(x) => x,
            None => {
                server_log!($($message)+);
                return None;
            },
        }
    }
}


fn lookup_information(state: &Rc<RefCell<State>>) {
    fn filtered_len(list: NodeList, name: &str, matched: &mut bool, illuminati_check: bool) -> f64 {
        let mut len = 0.0;

        for bettor in NodeListIter::new(list) {
            let bettor = get_text_content(&bettor).unwrap();

            if bettor == name {
                assert!(!*matched);
                assert!(illuminati_check);
                *matched = true;

            } else {
                len += 1.0;
            }
        }

        len
    }

    fn lookup(state: &mut State) -> Option<()> {
        if let Some(_) = state.information {
            return None;
        }

        let _open = state.open.as_ref()?;
        let closed = state.closed.as_ref()?;

        {
            let status = query("#betstatus")
                .and_then(|x| get_text_content(&x))?;

            if status != "Bets are locked until the next match." {
                return None;
            }
        }

        {
            let left_name = query("#sbettors1 > span.redtext > strong")
                .and_then(|x| get_text_content(&x))?;

            if left_name != closed.left.name {
                server_log!("Left names do not match: {:#?} {:#?}", left_name, closed.left.name);
                return None;
            }

            let left_name = query("#odds > span.redtext")
                .and_then(|x| get_text_content(&x))?;

            if left_name != closed.left.name {
                server_log!("Middle names do not match: {:#?} {:#?}", left_name, closed.left.name);
                return None;
            }
        }

        {
            let right_name = query("#sbettors2 > span.bluetext > strong")
                .and_then(|x| get_text_content(&x))?;

            if right_name != closed.right.name {
                server_log!("Left names do not match: {:#?} {:#?}", right_name, closed.right.name);
                return None;
            }

            let right_name = query("#odds > span.bluetext")
                .and_then(|x| get_text_content(&x))?;

            if right_name != closed.right.name {
                server_log!("Middle names do not match: {:#?} {:#?}", right_name, closed.right.name);
                return None;
            }
        }

        let left_count = unwrap_log!(
            query("#sbettors1 > span.redtext > span.counttext")
                .and_then(|x| get_text_content(&x))
                .and_then(|x| parse_f64(&x)),
            "Left count does not exist"
        );

        let right_count = unwrap_log!(
            query("#sbettors2 > span.bluetext > span.counttext")
                .and_then(|x| get_text_content(&x))
                .and_then(|x| parse_f64(&x)),
            "Right count does not exist"
        );

        // TODO a bit of code duplication with lookup_bet
        let current_balance = unwrap_log!(
            query("#balance")
                .and_then(|x| get_text_content(&x))
                .and_then(|x| parse_f64(&x)),
            "Current balance does not exist"
        );

        let name = unwrap_log!(
            query("#header span.navbar-text")
                .and_then(|x| get_text_content(&x))
                .and_then(|x| parse_name(&x)),
            "Username does not exist"
        );

        // TODO verify that the element exists ?
        let is_illuminati = query("#header span.navbar-text > .goldtext").is_some();

        let mut matched_left = false;
        let mut matched_right = false;

        let left_bettors_illuminati = filtered_len(query_all("#bettors1 > p.bettor-line > strong.goldtext"), &name, &mut matched_left, is_illuminati);
        let right_bettors_illuminati = filtered_len(query_all("#bettors2 > p.bettor-line > strong.goldtext"), &name, &mut matched_right, is_illuminati);

        let left_bettors_normal = filtered_len(query_all("#bettors1 > p.bettor-line > strong:not(.goldtext)"), &name, &mut matched_left, !is_illuminati);
        let right_bettors_normal = filtered_len(query_all("#bettors2 > p.bettor-line > strong:not(.goldtext)"), &name, &mut matched_right, !is_illuminati);

        let left_bet = query("#lastbet > span:first-of-type.redtext")
            .and_then(|x| get_text_content(&x))
            .and_then(|x| parse_money(&x));

        let right_bet = query("#lastbet > span:first-of-type.bluetext")
            .and_then(|x| get_text_content(&x))
            .and_then(|x| parse_money(&x));

        state.information = Some(Information {
            left_bettors_illuminati,
            right_bettors_illuminati,
            left_bettors_normal,
            right_bettors_normal,
            bet: match left_bet {
                Some(left) => match right_bet {
                    None => {
                        assert!(matched_left);
                        assert!(!matched_right);
                        assert_eq!(left_bettors_illuminati + left_bettors_normal + 1.0, left_count);
                        assert_eq!(right_bettors_illuminati + right_bettors_normal, right_count);
                        Bet::Left(left)
                    },
                    Some(_) => unreachable!(),
                },
                None => match right_bet {
                    Some(right) => {
                        assert!(!matched_left);
                        assert!(matched_right);
                        assert_eq!(left_bettors_illuminati + left_bettors_normal, left_count);
                        assert_eq!(right_bettors_illuminati + right_bettors_normal + 1.0, right_count);
                        Bet::Right(right)
                    },
                    None => {
                        assert!(!matched_left);
                        assert!(!matched_right);
                        assert_eq!(left_bettors_illuminati + left_bettors_normal, left_count);
                        assert_eq!(right_bettors_illuminati + right_bettors_normal, right_count);
                        Bet::None
                    },
                },
            },
            sum: current_balance,
        });

        Some(())
    }

    lookup(&mut state.borrow_mut());
}


// TODO timer which prints an error message if it's been >5 hours since a successful match recording
pub async fn observe_changes<A>(state: Rc<RefCell<State>>, messages: A) where A: Stream<Item = Vec<WaifuMessage>> + 'static {
    let mut mode_switch: Option<f64> = None;

    let mut process_messages = move |messages: Vec<WaifuMessage>| {
        for message in messages {
            match message {
                // This will only happen if the Twitch chat stops receiving messages for 15 minutes
                WaifuMessage::ReloadPage => {
                    reload_page();
                    return;
                },

                WaifuMessage::BetsOpen(open) => {
                    let mut state = state.borrow_mut();

                    state.clear_info_container();

                    state.did_bet = false;
                    state.open = Some(open);
                    state.closed = None;
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
                               state.closed.is_none() {
                                state.closed = Some(closed);
                                continue;

                            } else {
                                server_log!("Invalid data: {:#?} {:#?}", open, closed);
                            }
                        },
                        None => {},
                    }

                    state.clear_info_container();

                    state.open = None;
                    state.closed = None;
                },

                WaifuMessage::ModeSwitch { date, is_exhibition } => {
                    // When exhibition mode starts, reload the page, just in case something screwed up on saltybet.com
                    // TODO reload it when the first exhibition match begins, rather than after 15 minutes
                    if is_exhibition {
                        // 15 minutes
                        Timeout::new(900000, || {
                            reload_page();
                        }).forget();
                    }

                    let mut state = state.borrow_mut();

                    match state.open {
                        Some(ref open) => match state.closed {
                            Some(_) => {
                                if mode_switch.is_none() {
                                    mode_switch = Some(date);
                                    continue;

                                } else {
                                    server_log!("Duplicate mode switch: {:#?} {:#?} {:#?}", open, mode_switch, message);
                                }
                            },
                            None => {
                                server_log!("Missing closed: {:#?} {:#?}", open, message);
                            },
                        },
                        None => {},
                    }

                    state.clear_info_container();

                    state.open = None;
                    state.closed = None;
                    mode_switch = None;
                },

                WaifuMessage::Winner(winner) => {
                    let state: &mut State = &mut state.borrow_mut();

                    match state.open {
                        Some(ref open) => match state.closed {
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
                                            let record = Record {
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
                                            };

                                            if let Mode::Matchmaking = record.mode {
                                                state.simulation.insert_sum(record.sum);
                                            }

                                            state.simulation.insert_record(&record);

                                            // TODO figure out a way to avoid this clone
                                            // TODO is this guaranteed to be correctly ordered ?
                                            spawn(records_insert(vec![record.clone()]));

                                            state.records.push(record);

                                        } else {
                                            server_log!("Invalid bet data: {:#?} {:#?} {:#?} {:#?}", open, closed, information, winner);
                                        }

                                    } else {
                                        server_log!("Invalid match data: {:#?} {:#?} {:#?} {:#?}", open, closed, information, winner);
                                    }
                                },
                                None => {
                                    server_log!("Missing information: {:#?} {:#?} {:#?}", open, closed, winner);
                                },
                            },
                            None => {
                                server_log!("Missing closed: {:#?} {:#?}", open, winner);
                            },
                        },
                        None => {},
                    }

                    state.clear_info_container();

                    state.did_bet = false;
                    state.open = None;
                    state.closed = None;
                    state.information = None;
                    mode_switch = None;
                },
            }
        }
    };

    messages.for_each(move |messages| {
        process_messages(messages);
        async {}
    }).await
}


pub struct State {
    did_bet: bool,
    open: Option<WaifuBetsOpen>,
    closed: Option<WaifuBetsClosed>,
    information: Option<Information>,
    simulation: Simulation<CustomStrategy, CustomStrategy>,
    records: Vec<Record>,
    info_container: Rc<InfoContainer>,
}

impl State {
    fn update_info_container(&self, mode: &Mode, tier: &Tier, left: &str, right: &str, date: f64) {
        self.info_container.clear();

        // TODO avoid the to_string somehow
        self.info_container.left.name.set(Some(left.to_string()));
        self.info_container.right.name.set(Some(right.to_string()));

        let (left_bettors, right_bettors) = bettors(&self.simulation, left, right, *tier);
        self.info_container.left.bettors.set(Some(left_bettors));
        self.info_container.right.bettors.set(Some(right_bettors));

        let (left_winrate, right_winrate) = winrates(&self.simulation, left, right, *tier);
        self.info_container.left.winrate.set(Some(left_winrate));
        self.info_container.right.winrate.set(Some(right_winrate));

        self.info_container.left.matches_len.set(Some(self.simulation.matches_len(left, *tier)));
        self.info_container.right.matches_len.set(Some(self.simulation.matches_len(right, *tier)));

        let specific_matches = self.simulation.specific_matches_len(left, right, *tier);
        self.info_container.left.specific_matches_len.set(Some(specific_matches));
        self.info_container.right.specific_matches_len.set(Some(specific_matches));

        let (left_needed_odds, right_needed_odds) = needed_odds(&self.simulation, left, right, *tier);
        self.info_container.left.needed_odds.set(Some(left_needed_odds));
        self.info_container.right.needed_odds.set(Some(right_needed_odds));

        let (left_bet, right_bet) = match *mode {
            Mode::Matchmaking => self.simulation.matchmaking_strategy.as_ref().unwrap().bet_amount(&self.simulation, tier, left, right, date),
            Mode::Tournament => self.simulation.tournament_strategy.as_ref().unwrap().bet_amount(&self.simulation, tier, left, right, date),
        };

        self.info_container.left.bet_amount.set(Some(left_bet));
        self.info_container.right.bet_amount.set(Some(right_bet));

        let (left_odds, right_odds) = average_odds(&self.simulation, left, right, *tier, left_bet, right_bet);
        self.info_container.left.odds.set(Some(left_odds));
        self.info_container.right.odds.set(Some(right_odds));

        let (left_profit, right_profit) = expected_profits(&self.simulation, left, right, *tier, left_bet, right_bet);
        self.info_container.left.expected_profit.set(Some(left_profit));
        self.info_container.right.expected_profit.set(Some(right_profit));

        self.info_container.left.elo.set(Some(self.simulation.elo(left, *tier)));
        self.info_container.right.elo.set(Some(self.simulation.elo(right, *tier)));
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
    bettors: Mutable<Option<f64>>,
    elo: Mutable<Option<Elo>>,
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
            bettors: Mutable::new(None),
            elo: Mutable::new(None),
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
        self.bettors.set(None);
        self.elo.set(None);
    }

    fn render(&self, other: &Self, color: &str) -> Dom {
        fn info_bar<A, S, O, F>(this: &Mutable<Option<A>>, other: S, mut ord: O, mut f: F) -> Dom
            where A: Copy + 'static,
                  S: Signal<Item = Option<A>> + 'static,
                  O: FnMut(&A, &A) -> Option<Ordering> + 'static,
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
                    let other = other => move {
                        let cmp = this.and_then(|this| {
                            other.and_then(|other| {
                                ord(&this, &other)
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

        fn info_bar_elo<S, F>(this: &Mutable<Option<Elo>>, other: S, f: F, name: &'static str) -> Dom
            where S: Signal<Item = Option<Elo>> + 'static,
                  F: Fn(&Elo) -> glicko2::Glicko2Rating + Copy + 'static {
            info_bar(this, other,
                move |x, y| f(x).value.partial_cmp(&f(y).value),
                move |x| {
                    let x: glicko2::GlickoRating = f(&x).into();
                    format!("{}: {} (\u{00B1} {})", name, decimal(x.value), decimal(x.deviation))
                }
            )
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

                info_bar(&self.matches_len, other.matches_len.signal(), PartialOrd::partial_cmp, |x| {
                    format!("Number of past matches (in general): {}", x)
                }),

                info_bar(&self.specific_matches_len, always(Some(0)), PartialOrd::partial_cmp, |x| {
                    format!("Number of past matches (in specific): {}", x)
                }),

                info_bar(&self.bettors, other.bettors.signal(), PartialOrd::partial_cmp, |x| {
                    format!("Bettors: {}%", (-x) * 100.0)
                }),

                info_bar(&self.winrate, other.winrate.signal(), PartialOrd::partial_cmp, |x| {
                    format!("Winrate: {}%", x * 100.0)
                }),

                info_bar(&self.needed_odds, self.odds.signal(), PartialOrd::partial_cmp, |x| {
                    format!("Needed odds: {}", display_odds(x))
                }),

                info_bar(&self.odds, self.needed_odds.signal(), PartialOrd::partial_cmp, |x| {
                    format!("Average odds: {}", display_odds(x))
                }),

                info_bar_elo(&self.elo, other.elo.signal(), |x| x.wins, "Wins ELO"),
                info_bar_elo(&self.elo, other.elo.signal(), |x| x.upsets, "Upsets ELO"),

                info_bar(&self.bet_amount, other.bet_amount.signal(), PartialOrd::partial_cmp, |x| {
                    format!("Simulated bet amount: {}", money(x))
                }),

                info_bar(&self.expected_profit, other.expected_profit.signal(), PartialOrd::partial_cmp, |x| {
                    format!("Simulated expected profit: {}", money(x))
                }),
            ])
        })
    }
}

struct InfoContainer {
    visible: Mutable<bool>,
    left: InfoSide,
    right: InfoSide,
}

impl InfoContainer {
    fn new() -> Self {
        Self {
            visible: Mutable::new(true),
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

            .style_signal("display", self.visible.signal().map(|visible| {
                if visible {
                    None

                } else {
                    Some("none")
                }
            }))
            //.visible_signal(self.visible.signal())

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


async fn initialize_state(container: Rc<InfoContainer>) -> Result<(), JsValue> {
    let observe = {
        let port = ClientPort::connect("saltybet");

        let records = records_get_all().await?;
        //let matches = migrate_records(matches);

        log!("Initialized {} records", records.len());

        let mut simulation = Simulation::new();

        /*let matchmaking_strategy: FormulaStrategy = serde_json::from_str(include_str!("../../../strategies/matchmaking_strategy")).unwrap();
        let tournament_strategy: FormulaStrategy = serde_json::from_str(include_str!("../../../strategies/tournament_strategy")).unwrap();

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
            closed: None,
            information: None,
            simulation: simulation,
            records: records,
            info_container: container,
        }));

        {
            let state = state.clone();

            Interval::new(1000, move || {
                lookup_bet(&state);
            }).forget();
        }

        {
            let state = state.clone();

            Interval::new(10_000, move || {
                lookup_information(&state);
            }).forget();
        }

        observe_changes(state, port.messages())
    };

    log!("Initialized state");

    observe.await;

    Ok(())
}


#[wasm_bindgen(start)]
pub async fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();


    log!("Initializing...");

    // Reloads the page every 24 hours, just in case something screwed up on saltybet.com
    // Normally this doesn't happen, because it reloads the page at the start of exhibitions
    // TODO is 24 hours too long ? can it be made shorter ? should it be made shorter ?
    Timeout::new(86400000, || {
        reload_page();
    }).forget();


    stylesheet!("body", {
        // Same as Twitch chat
        .style_important("font-family", "Roobert, Helvetica Neue, Helvetica, Arial, sans-serif")
    });


    let container = Rc::new(InfoContainer::new());

    wait_until_defined(|| query("#iframeplayer"), clone!(container => move |video: Element| {
        struct Player {
            statistics: Rc<InfoContainer>,
            parent: Node,
            video: Element,
            showing_video: Mutable<bool>,
        }

        impl Player {
            fn swap(&mut self) {
                if self.showing_video.get() {
                    self.hide_video();

                } else {
                    self.show_video();
                }
            }

            fn show_video(&mut self) {
                self.showing_video.set_neq(true);
                self.parent.append_child(&self.video).unwrap();
                self.statistics.visible.set_neq(false);
            }

            fn hide_video(&mut self) {
                self.showing_video.set_neq(false);
                self.parent.remove_child(&self.video).unwrap();
                self.statistics.visible.set_neq(true);
            }

            fn render(mut self) -> Dom {
                html!("img", {
                    .attribute_signal("src", self.showing_video.signal().map(|showing| {
                        // TODO use RefFn to make this more efficient
                        if showing {
                            get_extension_url("icons/eye-blocked.svg")

                        } else {
                            get_extension_url("icons/eye.svg")
                        }
                    }))

                    .style("z-index", HIGHEST_ZINDEX)
                    .style("position", "absolute")
                    .style("bottom", "0px")
                    .style("right", "0px")
                    .style("width", "42px")
                    .style("height", "32px")
                    .style("background-color", "hsla(0, 0%, 100%, 1)")
                    .style("padding", "0px 5px")
                    .style("border-top-left-radius", "5px")
                    .style("cursor", "pointer")

                    .event(move |_: events::Click| {
                        self.swap();
                    })
                })
            }
        }

        let parent = video.parent_node().unwrap();

        dominator::append_dom(&parent, container.render());

        let mut player = Player {
            statistics: container,
            parent: parent.clone(),
            video,
            showing_video: Mutable::new(true),
        };

        player.hide_video();

        dominator::append_dom(&parent, player.render());

        log!("Information initialized, video hidden");
    }));


    initialize_state(container).await
}
