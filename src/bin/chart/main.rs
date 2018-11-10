#![recursion_limit="256"]
#![feature(async_await, await_macro, futures_api)]

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate salty_bet_bot;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate dominator;
#[macro_use]
extern crate futures_signals;

use std::f64::INFINITY;
use std::rc::Rc;
use salty_bet_bot::{records_get_all, subtract_days, add_days, decimal, Loading, set_panic_hook};
use algorithm::record::{Record, Profit, Mode};
use algorithm::simulation::{Bet, Simulation, Strategy, Simulator, SALT_MINE_AMOUNT};
use algorithm::strategy::{CustomStrategy, MoneyStrategy, BetStrategy, PERCENTAGE_THRESHOLD, GENETIC_STRATEGY};
use stdweb::traits::*;
use stdweb::{spawn_local, unwrap_future};
use stdweb::web::{document, set_timeout, Date};
use stdweb::web::error::Error;
use stdweb::web::html_element::SelectElement;
use stdweb::web::event::{ClickEvent, ChangeEvent, MouseMoveEvent, MouseEnterEvent, MouseLeaveEvent};
use stdweb::unstable::TryInto;
use futures_signals::signal::{Mutable, Signal, SignalExt, and, not, always};
use dominator::{Dom, text};


// 21 days
const DATE_CUTOFF: u32 = 21;

const MATCHMAKING_STARTING_MONEY: f64 = 10_000_000.0;

lazy_static! {
    static ref STARTING_DATE: f64 = subtract_days(Date::now(), DATE_CUTOFF);
}


#[allow(dead_code)]
enum ChartMode<A> {
    SimulateTournament(A),
    SimulateMatchmaking { strategy: A, reset_money: bool },
    RealData { days: Option<u32> },
}


// TODO move into utility module
fn normalize(value: f64, min: f64, max: f64) -> f64 {
    // TODO is this correct ?
    if min == max {
        0.0

    } else {
        ((value - min) * (1.0 / (max - min))).max(0.0).min(1.0)
    }
}

// TODO move into utility module
fn range_inclusive(percentage: f64, low: f64, high: f64) -> f64 {
    low + (percentage * (high - low))
}


#[derive(Debug)]
enum RecordInformation {
    TournamentFinal {
        date: f64,
        profit: f64,
        old_sum: f64,
        new_sum: f64,
    },
    Match {
        date: f64,
        profit: Profit,
        old_sum: f64,
        new_sum: f64,
        match_len: f64,
        mode: Mode,
        won: bool,
        odds: Option<Result<f64, f64>>,
        bet: Bet,
    },
}

impl RecordInformation {
    fn calculate<A: Strategy>(records: &[Record], mode: ChartMode<A>, extra_data: bool) -> Vec<Self> {
        let mut output: Vec<RecordInformation> = vec![];

        let mut simulation = Simulation::new();

        //simulation.sum = PERCENTAGE_THRESHOLD;

        match mode {
            ChartMode::SimulateTournament(strategy) => {
                simulation.tournament_strategy = Some(strategy);

                let mut index: f64 = 0.0;
                let mut sum: f64 = 0.0;

                for record in records {
                    let tournament_profit = simulation.tournament_profit(&record);

                    let bet = if let Mode::Tournament = record.mode {
                        simulation.bet(&record)

                    } else {
                        Bet::None
                    };

                    let old_sum = simulation.tournament_sum;

                    let match_len = simulation.min_matches_len(&record.left.name, &record.right.name);

                    if let Some(amount) = bet.amount() {
                        if amount > 1.0 {
                            simulation.calculate(&record, &bet);

                            let new_sum = simulation.tournament_sum;

                            if let Mode::Tournament = record.mode {
                                let date = index;
                                index += 1.0;

                                let profit = Profit::from_old_new(old_sum, new_sum);

                                output.push(RecordInformation::Match {
                                    date,
                                    profit,
                                    old_sum: old_sum,
                                    new_sum: new_sum,
                                    match_len,
                                    mode: record.mode,
                                    won: record.won(&bet),
                                    odds: record.odds(&bet),
                                    bet,
                                });
                            }
                        }
                    }

                    if let Some(tournament_profit) = tournament_profit {
                        let date = index;
                        index += 1.0;

                        let old_sum = sum;

                        sum += tournament_profit;

                        output.push(RecordInformation::TournamentFinal {
                            date,
                            profit: tournament_profit,
                            old_sum: old_sum,
                            new_sum: sum,
                        });
                    }

                    simulation.insert_record(&record);
                }
            },

            ChartMode::SimulateMatchmaking { strategy, reset_money } => {
                simulation.matchmaking_strategy = Some(strategy);

                //let mut index: f64 = 0.0;

                if extra_data {
                    for record in records.iter() {
                        simulation.insert_record(&record);
                    }
                }

                let mut date_cutoff: Option<f64> = None;

                if reset_money {
                    simulation.sum = MATCHMAKING_STARTING_MONEY;
                }

                for record in records {
                    if reset_money {
                        if let Some(old_date) = date_cutoff {
                            if record.date > old_date {
                                simulation.sum = MATCHMAKING_STARTING_MONEY;
                                date_cutoff = Some(add_days(old_date, DATE_CUTOFF));
                            }

                        // TODO implement this more efficiently
                        } else {
                            let mut ending_date = *STARTING_DATE;

                            while ending_date > record.date {
                                ending_date = subtract_days(ending_date, DATE_CUTOFF);
                            }

                            date_cutoff = Some(add_days(ending_date, DATE_CUTOFF));
                        }
                    }

                    if //simulation.min_matches_len(&record.left.name, &record.right.name) >= 10.0 &&
                       record.mode == Mode::Matchmaking {

                        let old_sum = simulation.sum;

                        let match_len = simulation.min_matches_len(&record.left.name, &record.right.name);

                        let tournament_profit = simulation.tournament_profit(&record);

                        let bet = simulation.bet(&record);

                        if let Some(amount) = bet.amount() {
                            if amount > 1.0 {
                                simulation.calculate(&record, &bet);

                                simulation.sum -= tournament_profit.unwrap_or(0.0);

                                let new_sum = simulation.sum;

                                simulation.insert_sum(new_sum);

                                let profit = Profit::from_old_new(old_sum, new_sum);

                                let date = record.date;
                                //let date = index;
                                //index += 1.0;

                                output.push(RecordInformation::Match {
                                    date,
                                    profit,
                                    old_sum: old_sum,
                                    new_sum: new_sum,
                                    match_len,
                                    mode: record.mode,
                                    won: record.won(&bet),
                                    odds: record.odds(&bet),
                                    bet,
                                });
                            }
                        }
                    }

                    if !extra_data {
                        simulation.insert_record(&record);
                    }
                }
            },

            ChartMode::RealData { days } => {
                //simulation.sum = SALT_MINE_AMOUNT;

                // TODO
                let days: Option<f64> = days.map(|days| subtract_days(Date::now(), days));

                //let input: Vec<&Record> = input.into_iter().filter(|record| record.mode == Mode::Matchmaking).collect();

                for record in records {
                    let date = record.date;

                    if record.sum != -1.0 {
                        match record.mode {
                            Mode::Tournament => {
                                simulation.tournament_sum = record.sum;
                            },
                            Mode::Matchmaking => {
                                simulation.sum = record.sum;
                            },
                        }
                    }

                    if days.map(|days| date >= days).unwrap_or(true) {
                        let old_sum = simulation.sum;

                        let tournament_profit = simulation.tournament_profit(&record);

                        if let Some(tournament_profit) = tournament_profit {
                            let new_sum = old_sum + tournament_profit;

                            output.push(RecordInformation::TournamentFinal {
                                date,
                                profit: tournament_profit,
                                old_sum: old_sum,
                                new_sum: new_sum,
                            });
                        }

                        let old_sum = old_sum + tournament_profit.unwrap_or(0.0);

                        let match_len = simulation.min_matches_len(&record.left.name, &record.right.name);

                        let bet = record.bet.clone();

                        if let Some(amount) = bet.amount() {
                            if amount > 1.0 {
                                simulation.calculate(&record, &bet);

                                if let Mode::Matchmaking = record.mode {
                                    let new_sum = simulation.sum;

                                    simulation.insert_sum(new_sum);

                                    let profit = Profit::from_old_new(old_sum, new_sum);

                                    output.push(RecordInformation::Match {
                                        date,
                                        profit,
                                        old_sum: old_sum,
                                        new_sum: new_sum,
                                        match_len,
                                        mode: record.mode,
                                        won: record.won(&bet),
                                        odds: record.odds(&bet),
                                        bet,
                                    });
                                }
                            }
                        }

                    } else {
                        simulation.calculate(&record, &record.bet);
                    }

                    simulation.insert_record(&record);
                }
            },
        }

        output
    }

    fn date(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { date, .. } => date,
            RecordInformation::Match { date, .. } => date,
        }
    }

    fn old_sum(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { old_sum, .. } => old_sum,
            RecordInformation::Match { old_sum, .. } => old_sum,
        }
    }

    fn new_sum(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { new_sum, .. } => new_sum,
            RecordInformation::Match { new_sum, .. } => new_sum,
        }
    }
}


#[derive(Debug, Clone)]
struct Statistics {
    len: f64,
    wins: f64,
    losses: f64,

    average_odds: f64,
    odds_gain: f64,
    odds_loss: f64,
    max_odds_gain: f64,
    max_odds_loss: f64,

    max_gain: f64,
    max_loss: f64,

    max_bet: f64,
    min_bet: f64,
    average_bet: f64,

    max_money: f64,
    min_money: f64,

    min_match_len: f64,
    max_match_len: f64,

    lowest_date: f64,
    highest_date: f64,
}

impl Statistics {
    fn new(records: &[Record], information: &[RecordInformation], show_recent: bool) -> Self {
        let mut len: f64 = 0.0;
        let mut wins: f64 = 0.0;
        let mut losses: f64 = 0.0;

        let mut average_odds: f64 = 0.0;
        let mut odds_gain: f64 = 0.0;
        let mut odds_loss: f64 = 0.0;
        let mut max_odds_gain: f64 = 0.0;
        let mut max_odds_loss: f64 = 0.0;

        let mut max_gain: f64 = 0.0;
        let mut max_loss: f64 = 0.0;

        let mut max_bet: f64 = 0.0;
        let mut min_bet: f64 = 0.0;
        let mut average_bet: f64 = 0.0;

        let mut min_money: f64 = -1.0;
        let mut max_money: f64 = 0.0;

        let mut min_match_len: f64 = -1.0;
        let mut max_match_len: f64 = 0.0;

        let mut lowest_date: f64 = if show_recent { -1.0 } else { records.first().map(|x| x.date).unwrap_or(0.0) };
        let highest_date: f64 = records.last().map(|x| x.date).unwrap_or(0.0);

        for record in information {
            let date = record.date();
            let old_sum = record.old_sum();
            let new_sum = record.new_sum();

            if show_recent {
                lowest_date = if lowest_date == -1.0 { date } else { lowest_date.min(date) };
                //highest_date = highest_date.max(date);
            }

            min_money = if min_money == -1.0 { old_sum } else { min_money.min(old_sum) };
            max_money = max_money.max(old_sum);

            min_money = if min_money == -1.0 { new_sum } else { min_money.min(new_sum) };
            max_money = max_money.max(new_sum);

            match record {
                RecordInformation::TournamentFinal { profit, old_sum, new_sum, .. } => {
                    assert!(*profit > 0.0);
                    assert!(new_sum > old_sum);
                    max_gain = max_gain.max(*profit);
                },
                RecordInformation::Match { profit, won, odds, bet, match_len, .. } => {
                    if let Some(bet_amount) = bet.amount() {
                        // TODO is this correct ?
                        if bet_amount > 1.0 {
                            len += 1.0;

                            match profit {
                                Profit::Gain(gain) => {
                                    max_gain = max_gain.max(*gain);
                                },
                                Profit::Loss(loss) => {
                                    max_loss = max_loss.max(*loss);
                                },
                                Profit::None => {},
                            }

                            min_match_len = if min_match_len == -1.0 { *match_len } else { min_match_len.min(*match_len) };
                            max_match_len = max_match_len.max(*match_len);

                            if *won {
                                wins += 1.0;

                            } else {
                                losses += 1.0;
                            }

                            match odds {
                                Some(Ok(amount)) => {
                                    average_odds += amount;
                                    odds_gain += amount;
                                    max_odds_gain = max_odds_gain.max(*amount);
                                },

                                Some(Err(amount)) => {
                                    average_odds += -1.0;
                                    odds_loss += -1.0;
                                    max_odds_loss = max_odds_loss.max(*amount);
                                },

                                // TODO is this correct ?
                                None => {},
                            }

                            min_bet = if min_bet == 0.0 { bet_amount } else { min_bet.min(bet_amount) };
                            max_bet = max_bet.max(bet_amount);
                            average_bet += bet_amount;
                        }
                    }
                },
            }
        }

        //let len = information.len() as f64;

        Self {
            average_odds: average_odds / len,
            odds_gain: odds_gain / len,
            odds_loss: odds_loss / len,
            average_bet: average_bet / len,
            max_odds_loss, max_odds_gain, len, wins, losses, max_gain, max_loss, max_bet, min_bet, max_money, min_money, min_match_len, max_match_len, lowest_date, highest_date,
        }
    }

    /*fn merge(&self, other: &Self) -> Self {
        Self {
            max_gain: self.max_gain.max(other.max_gain),
            max_loss: self.max_loss.max(other.max_loss),
            max_bet: self.max_bet.max(other.max_bet),
            min_bet: self.min_bet.min(other.min_bet),
            max_money: self.max_money.max(other.max_money),
            min_money: self.min_money.min(other.min_money),
            lowest_date: self.lowest_date.min(other.lowest_date),
            highest_date: self.highest_date.max(other.highest_date),
        }
    }*/
}


struct Information {
    record_information: Vec<RecordInformation>,
    recent_statistics: Statistics,
    total_statistics: Statistics,
}

impl Information {
    fn new(records: &[Record], record_information: Vec<RecordInformation>, show_only_recent_data: bool) -> Self {
        // TODO is this `false` correct ?
        let total_statistics = Statistics::new(records, &record_information, false);

        let cutoff_date = if show_only_recent_data {
            Some(*STARTING_DATE)
            //Some(1000)

        } else {
            None
        };

        match cutoff_date {
            Some(date) => {
                let record_information: Vec<RecordInformation> = record_information.into_iter().filter(|x| x.date() >= date).collect();
                let recent_statistics = Statistics::new(records, &record_information, cutoff_date.is_some());

                Self {
                    record_information,
                    recent_statistics,
                    total_statistics
                }
            },
            None => {
                Self {
                    record_information,
                    recent_statistics: total_statistics.clone(),
                    total_statistics
                }
            },
        }
    }
}


/*fn display_record(record: &Record, information: &Information) -> Element {
    let node = document().create_element("div").unwrap();

    fn set_height(node: &Element, information: &Information, amount: f64) {
        let total = information.max_gain + information.max_loss;

        js! { @(no_return)
            var node = @{&node};
            node.style.height = @{format!("{}%", (amount / total) * 100.0)};
        }
    }

    let total = information.max_gain + information.max_loss;

    match record.profit(&record.bet) {
        Profit::Gain(amount) => {
            node.class_list().add("record-gain").unwrap();
            set_height(&node, &information, amount);

            js! { @(no_return)
                var node = @{&node};
                node.style.top = @{format!("{}%", ((1.0 - (amount / total)) - (information.max_loss / total)) * 100.0)};
            }
        },
        Profit::Loss(amount) => {
            node.class_list().add("record-loss").unwrap();
            set_height(&node, &information, amount);

            js! { @(no_return)
                var node = @{&node};
                node.style.top = @{format!("{}%", (information.max_gain / total) * 100.0)};
            }
        },
        Profit::None => {
            node.class_list().add("record-none").unwrap();
        },
    }

    node
}*/


/*fn simulation_bet<A: Strategy, B: Strategy>(simulation: &mut Simulation<A, B>, record: &Record, money: f64) -> Bet {
    match record.mode {
        Mode::Matchmaking => {
            simulation.in_tournament = false;
            simulation.sum = money;

            match simulation.matchmaking_strategy {
                Some(ref a) => simulation.pick_winner(a, &record.tier, &record.left.name, &record.right.name),
                None => Bet::None,
            }
        },

        Mode::Tournament => {
            simulation.in_tournament = true;
            simulation.tournament_sum = money;

            match simulation.tournament_strategy {
                Some(ref a) => simulation.pick_winner(a, &record.tier, &record.left.name, &record.right.name),
                None => Bet::None,
            }
        },
    }
}*/


const TEXT_SHADOW: &'static str = "-2px -2px 1px black, -2px 2px 1px black, 2px -2px 1px black, 2px 2px 1px black, -1px -1px 1px black, -1px 1px 1px black, 1px -1px 1px black, 1px 1px 1px black";

const BACKGROUND_COLOR: &'static str = "hsla(0, 0%, 0%, 0.65)";


lazy_static! {
    static ref WIDGET: String = class! {
        .style("height", "20px")
        .style("margin-top", "5px")
    };
}


fn display_records(records: Vec<Record>, loading: Loading) -> Dom {
    struct State {
        simulation_type: Mutable<Rc<String>>,
        money_type: Mutable<Rc<String>>,
        hover_info: Mutable<bool>,
        hover_percentage: Mutable<Option<f64>>,
        average_sums: Mutable<bool>,
        show_only_recent_data: Mutable<bool>,
        round_to_magnitude: Mutable<bool>,
        extra_data: Mutable<bool>,
        reset_money: Mutable<bool>,
        information: Mutable<Rc<Information>>,
        records: Vec<Record>,
    }

    impl State {
        fn new(records: Vec<Record>) -> Self {
            // TODO this should be based on the defaults
            let information = Information::new(&records, Self::real_data(&records), true);

            Self {
                simulation_type: Mutable::new(Rc::new("real-data".to_string())),
                money_type: Mutable::new(Rc::new("expected-bet-winner".to_string())),
                hover_info: Mutable::new(false),
                hover_percentage: Mutable::new(None),
                average_sums: Mutable::new(false),
                show_only_recent_data: Mutable::new(true),
                round_to_magnitude: Mutable::new(false),
                extra_data: Mutable::new(false),
                reset_money: Mutable::new(true),
                information: Mutable::new(Rc::new(information)),
                records,
            }
        }

        fn real_data(records: &[Record]) -> Vec<RecordInformation> {
            let real: ChartMode<()> = ChartMode::RealData { days: None };
            RecordInformation::calculate(records, real, true)
        }

        fn update(&self) {
            let information = match self.simulation_type.lock_ref().as_str() {
                "real-data" => Self::real_data(&self.records),

                simulation_type => RecordInformation::calculate(&self.records, ChartMode::SimulateMatchmaking {
                    reset_money: self.reset_money.get(),
                    strategy: CustomStrategy {
                        average_sums: self.average_sums.get(),
                        scale_by_matches: true,
                        round_to_magnitude: self.round_to_magnitude.get(),
                        money: match self.money_type.lock_ref().as_str() {
                            "expected-bet-winner" => MoneyStrategy::ExpectedBetWinner,
                            "expected-bet" => MoneyStrategy::ExpectedBet,
                            "winner-bet" => MoneyStrategy::WinnerBet,
                            "percentage" => MoneyStrategy::Percentage,
                            "all-in" => MoneyStrategy::AllIn,
                            "fixed" => MoneyStrategy::Fixed,
                            a => panic!("Invalid value {}", a),
                        },
                        bet: match simulation_type {
                            // TODO avoid the clone somehow
                            "Genetic" => BetStrategy::Genetic(GENETIC_STRATEGY.clone()),
                            "expected-bet-winner" => BetStrategy::ExpectedBetWinner,
                            "expected-bet" => BetStrategy::ExpectedBet,
                            "earnings" => BetStrategy::ExpectedProfit,
                            "winner-bet" => BetStrategy::WinnerBet,
                            "upset-percentage" => BetStrategy::Upsets,
                            "upset-odds" => BetStrategy::Odds,
                            "upset-odds-difference" => BetStrategy::OddsDifference,
                            "upset-odds-winner" => BetStrategy::WinnerOdds,
                            "Bettors" => BetStrategy::Bettors,
                            "IlluminatiBettors" => BetStrategy::IlluminatiBettors,
                            "NormalBettors" => BetStrategy::NormalBettors,
                            "winrate-high" => BetStrategy::Wins,
                            "winrate-low" => BetStrategy::Losses,
                            "random-left" => BetStrategy::Left,
                            "random-right" => BetStrategy::Right,
                            "random" => BetStrategy::Random,
                            a => panic!("Invalid value {}", a),
                        },
                    }
                }, self.extra_data.get()),
            //ChartMode::RealData { days: Some(7), matches: None }
            //ChartMode::RealData { days: None }
            //ChartMode::SimulateMatchmaking(EarningsStrategy { expected_profit, winrate, bet_difference: false, winrate_difference: false, use_percentages })
            //ChartMode::SimulateMatchmaking(matchmaking_strategy())
            };

            self.information.set(Rc::new(Information::new(&self.records, information, self.show_only_recent_data.get())));
        }

        fn show_options(&self) -> impl Signal<Item = bool> {
            self.simulation_type.signal_cloned().map(|x| *x != "real-data")
        }
    }


    fn svg_root(state: Rc<State>) -> Dom {
        lazy_static! {
            static ref CLASS: String = class! {
                .style("position", "absolute")
                .style("top", "0px")
                .style("left", "0px")
                .style("width", "100%")
                .style("height", "100%")
            };
        }

        svg!("svg", {
            .class(&*CLASS)
            .attribute("xmlns", "http://www.w3.org/2000/svg")
            .attribute("viewBox", "0 0 100 100")
            .attribute("preserveAspectRatio", "none")

            .with_element(|dom, element| {
                dom.global_event(clone!(state => move |e: MouseMoveEvent| {
                    // TODO don't hardcode this
                    let x = (e.client_x() as f64) - 5.0;
                    // TODO use get_bounding_client_rect instead
                    let width: f64 = js!( return @{&element}.clientWidth; ).try_into().unwrap();

                    let percentage = (x / width).max(0.0).min(1.0);

                    state.hover_percentage.set_neq(Some(percentage));
                }))
            })

            .children_signal_vec(state.information.signal_cloned().map(|information| {
                let statistics = &information.recent_statistics;

                let mut d_gains = vec![];
                let mut d_losses = vec![];
                let mut d_money = vec!["M0,100".to_owned()];
                let mut d_smooth_1 = vec![];
                let mut d_smooth_2 = vec![];
                let mut d_smooth_3 = vec![];
                let mut d_smooth_4 = vec![];
                let mut d_smooth_5 = vec![];
                let mut d_smooth_6 = vec![];
                let mut d_smooth_7 = vec![];
                let mut d_bets = vec![];
                let mut d_match_len = vec![];
                //let mut d_winner_profit = vec![];
                let mut d_tournaments = vec![];

                //let len = information.record_information.len();

                let y = (statistics.max_gain / (statistics.max_gain + statistics.max_loss)) * 100.0;
                //let y = (statistics.max_odds_gain / (statistics.max_odds_gain + statistics.max_odds_loss)) * 100.0;

                let mut first = true;

                let mut smooth_sums = vec![];

                for (_index, record) in information.record_information.iter().enumerate() {
                    let date = record.date();

                    //let x = normalize(index as f64, 0.0, len) * 100.0;
                    let x = normalize(date, statistics.lowest_date, statistics.highest_date) * 100.0;

                    let (old_sum, new_sum) = match record {
                        RecordInformation::TournamentFinal { profit, old_sum, new_sum, .. } => {
                            // TODO code duplication with the Statistics
                            d_tournaments.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(*profit, 0.0, statistics.max_gain), y, 0.0), x, y));
                            (*old_sum, *new_sum)
                        },
                        RecordInformation::Match { profit, bet, old_sum, new_sum, mode, match_len, .. } => {
                            if let Mode::Matchmaking = mode {
                                d_match_len.push(format!("M{},{}L{},{}",
                                    x,
                                    100.0,
                                    x,
                                    normalize(*match_len, statistics.max_match_len, 0.0) * 100.0));

                                /*match odds {
                                    Ok(amount) => {
                                        d_gains.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(*amount, 0.0, statistics.odds_max_gain), y, 0.0), x, y));

                                        let y = range_inclusive(normalize(1.0, 0.0, statistics.odds_max_gain), y, 0.0);
                                        d_bets.push(format!("M{},{}L{},{}", x, y, x, y + 0.3));
                                    },

                                    Err(amount) => {
                                        d_losses.push(format!("M{},{}L{},{}", x, y, x, range_inclusive(normalize(1.0, 0.0, statistics.max_odds_loss), y, 100.0)));

                                        let y = range_inclusive(normalize(*amount, 0.0, statistics.max_odds_loss), y, 100.0);
                                        d_winner_profit.push(format!("M{},{}L{},{}", x, y - 0.3, x, y));
                                    },
                                }*/

                                match *profit {
                                    Profit::Gain(amount) => {
                                        d_gains.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(amount, 0.0, statistics.max_gain), y, 0.0), x, y));

                                        if let Some(amount) = bet.amount() {
                                            let y = range_inclusive(normalize(amount, 0.0, statistics.max_gain), y, 0.0);
                                            d_bets.push(format!("M{},{}L{},{}", x, y, x, y + 0.3));
                                            //format!("M{},100L{},{}", x, x, normalize(amount, information.max_bet, information.min_bet) * 100.0)
                                        }
                                    },
                                    Profit::Loss(amount) => {
                                        d_losses.push(format!("M{},{}L{},{}", x, y, x, range_inclusive(normalize(amount, 0.0, statistics.max_loss), y, 100.0)));
                                    },
                                    Profit::None => {},
                                }
                            }

                            (*old_sum, *new_sum)
                        },
                    };

                    smooth_sums.push((x, date, new_sum));

                    if first {
                        first = false;
                        d_money.push(format!("M{},{}", x, normalize(old_sum, statistics.max_money, statistics.min_money) * 100.0));
                    }

                    d_money.push(format!("L{},{}", x, normalize(new_sum, statistics.max_money, statistics.min_money) * 100.0));
                }

                fn smooth(x: f64, statistics: &Statistics, d_smooth: &mut Vec<String>, smooth_sums: &[(f64, f64, f64)], date_range: f64, date: f64) {
                    let half_range = date_range / 2.0;

                    let date_start = date - half_range;
                    let date_end = date + half_range;

                    let mut sum = 0.0;
                    let mut len = 0.0;

                    // TODO make this more efficient
                    for (_, date, new_sum) in smooth_sums {
                        if *date >= date_start &&
                           *date <= date_end {
                            sum += new_sum;
                            len += 1.0;
                        }
                    }

                    let average = sum / len;

                    if d_smooth.len() == 0 {
                        d_smooth.push(format!("M{},{}", x, normalize(average, statistics.max_money, statistics.min_money) * 100.0));

                    } else {
                        d_smooth.push(format!("L{},{}", x, normalize(average, statistics.max_money, statistics.min_money) * 100.0));
                    }
                }

                const ONE_DAY: f64 = 1000.0 * 60.0 * 60.0 * 24.0;

                for (x, date, _) in smooth_sums.iter() {
                    smooth(*x, &statistics, &mut d_smooth_1, &smooth_sums, ONE_DAY * 1.0, *date);
                    smooth(*x, &statistics, &mut d_smooth_2, &smooth_sums, ONE_DAY * 2.0, *date);
                    smooth(*x, &statistics, &mut d_smooth_3, &smooth_sums, ONE_DAY * 3.0, *date);
                    smooth(*x, &statistics, &mut d_smooth_4, &smooth_sums, ONE_DAY * 4.0, *date);
                    smooth(*x, &statistics, &mut d_smooth_5, &smooth_sums, ONE_DAY * 5.0, *date);
                    smooth(*x, &statistics, &mut d_smooth_6, &smooth_sums, ONE_DAY * 6.0, *date);
                    smooth(*x, &statistics, &mut d_smooth_7, &smooth_sums, ONE_DAY * 7.0, *date);
                }

                fn make_line(d: Vec<String>, color: &str) -> Dom {
                    let d: String = d.iter().flat_map(|x| x.chars()).collect();

                    svg!("path", {
                        .attribute("stroke", color)
                        .attribute("stroke-width", "1px")
                        .attribute("stroke-opacity", "1")
                        .attribute("fill-opacity", "0")
                        .attribute("vector-effect", "non-scaling-stroke")
                        .attribute("d", &d)
                    })
                }

                const HUE_START: f64 = 360.0;
                const HUE_END: f64 = 180.0;

                const LIGHT_START: f64 = 99.9;
                const LIGHT_END: f64 = 80.0;

                vec![
                    make_line(d_match_len, "black"),
                    make_line(d_gains, "hsla(120, 75%, 50%, 1)"),
                    // #6441a5
                    make_line(d_bets, "hsla(0, 100%, 50%, 1)"),
                    make_line(d_losses, "hsla(0, 75%, 50%, 1)"),
                    //make_line(d_winner_profit, "hsla(120, 75%, 50%, 1)"),
                    make_line(d_tournaments, "hsl(240, 100%, 75%)"),
                    make_line(d_smooth_7, &format!("hsl({}, 100%, {}%)", range_inclusive(7.0 / 7.0, HUE_START, HUE_END), range_inclusive(7.0 / 7.0, LIGHT_START, LIGHT_END))),
                    make_line(d_smooth_6, &format!("hsl({}, 100%, {}%)", range_inclusive(6.0 / 7.0, HUE_START, HUE_END), range_inclusive(6.0 / 7.0, LIGHT_START, LIGHT_END))),
                    make_line(d_smooth_5, &format!("hsl({}, 100%, {}%)", range_inclusive(5.0 / 7.0, HUE_START, HUE_END), range_inclusive(5.0 / 7.0, LIGHT_START, LIGHT_END))),
                    make_line(d_smooth_4, &format!("hsl({}, 100%, {}%)", range_inclusive(4.0 / 7.0, HUE_START, HUE_END), range_inclusive(4.0 / 7.0, LIGHT_START, LIGHT_END))),
                    make_line(d_smooth_3, &format!("hsl({}, 100%, {}%)", range_inclusive(3.0 / 7.0, HUE_START, HUE_END), range_inclusive(3.0 / 7.0, LIGHT_START, LIGHT_END))),
                    make_line(d_smooth_2, &format!("hsl({}, 100%, {}%)", range_inclusive(2.0 / 7.0, HUE_START, HUE_END), range_inclusive(2.0 / 7.0, LIGHT_START, LIGHT_END))),
                    make_line(d_smooth_1, &format!("hsl({}, 100%, {}%)", range_inclusive(1.0 / 7.0, HUE_START, HUE_END), range_inclusive(1.0 / 7.0, LIGHT_START, LIGHT_END))),
                    make_line(d_money,    &format!("hsl({}, 100%, {}%)", range_inclusive(0.0 / 7.0, HUE_START, HUE_END), range_inclusive(0.0 / 7.0, LIGHT_START, LIGHT_END))),
                ]
            }).to_signal_vec())
        })
    }


    fn make_info_popup(state: Rc<State>) -> Dom {
        lazy_static! {
            static ref CLASS_LINE: String = class! {
                .style("position", "absolute")
                .style("left", "0px")
                .style("top", "0px")
                .style("width", "1px")
                .style("height", "100%")
                .style("z-index", "1")
                .style("background-color", "white")
                .style("pointer-events", "none")
            };

            static ref CLASS_TEXT: String = class! {
                .style("position", "absolute")
                .style("bottom", "0px")
                .style("color", "white")
                .style("text-shadow", TEXT_SHADOW)
                .style("background-color", BACKGROUND_COLOR)
                .style("font-size", "16px")
                .style("white-space", "pre")
                .style("padding", "10px")
            };
        }

        html!("div", {
            .class(&*CLASS_LINE)

            .style_signal("left", state.hover_percentage.signal().map(|percentage| {
                percentage.map(|percentage| {
                    format!("{}%", percentage * 100.0)
                })
            }))

            .visible_signal(and(
                state.hover_percentage.signal().map(|percentage| percentage.is_some()),
                not(state.hover_info.signal())
            ))

            .children(&mut [
                html!("div", {
                    .class(&*CLASS_TEXT)

                    .style_signal("left", state.hover_percentage.signal().map(|percentage| {
                        percentage.and_then(|percentage| {
                            if percentage <= 0.5 {
                                Some("1px")

                            } else {
                                None
                            }
                        })
                    }))

                    .style_signal("right", state.hover_percentage.signal().map(|percentage| {
                        percentage.and_then(|percentage| {
                            if percentage > 0.5 {
                                Some("1px")

                            } else {
                                None
                            }
                        })
                    }))

                    .text_signal(map_ref! {
                        let percentage = state.hover_percentage.signal(),
                        let information = state.information.signal_cloned() => {
                            if let Some(percentage) = percentage {
                                let first_date = information.record_information.first().map(|x| x.date()).unwrap_or(0.0);
                                let last_date = information.record_information.last().map(|x| x.date()).unwrap_or(0.0);

                                let date = range_inclusive(*percentage, first_date, last_date);

                                let mut index = 0;
                                let mut difference = INFINITY;

                                // TODO rewrite this with combinators ?
                                // TODO make this more efficient
                                for (i, record) in information.record_information.iter().enumerate() {
                                    let diff = (record.date() - date).abs();

                                    if diff < difference {
                                        difference = diff;
                                        index = i;
                                    }
                                }

                                //let index = information.record_information.binary_search_by(|a| a.date().partial_cmp(&date).unwrap());

                                let record = &information.record_information[index];
                                format!("{}\n{:#?}", index, record)
                                //format!("{}\n{:#?}\n{:#?}\n{:#?}\n{:#?}\n{:#?}", information.record_information.len(), first_date, date, percentage, index, difference)

                            } else {
                                "".to_string()
                            }
                        }
                    })
                })
            ])
        })
    }


    fn make_text(state: Rc<State>) -> Dom {
        lazy_static! {
            static ref CLASS: String = class! {
                //.style("width", "100%")
                //.style("height", "100%")
                .style("color", "white")
                .style("text-shadow", TEXT_SHADOW)
                .style("font-size", "16px")
            };
        }

        html!("div", {
            .class(&*CLASS)

            .text_signal(state.information.signal_cloned().map(|information| {
                let statistics = &information.recent_statistics;
                let starting_money = information.record_information.first().map(|x| x.old_sum()).unwrap_or(0.0);
                let final_money = information.record_information.last().map(|x| x.new_sum()).unwrap_or(0.0);

                let total_gains = final_money - starting_money;
                let average_gains = total_gains / statistics.len;

                format!("Minimum: {}\nMaximum: {}\nStarting money: {}\nFinal money: {}\nTotal gains: {}\nAverage gains: {}\nMatches: {} (out of {})\nAverage odds: {}\nAverage bet: {}\nWinrate: {}%",
                    salty_bet_bot::money(statistics.min_money),
                    salty_bet_bot::money(statistics.max_money),
                    salty_bet_bot::money(starting_money),
                    salty_bet_bot::money(final_money),
                    salty_bet_bot::money(total_gains),
                    salty_bet_bot::money(average_gains),
                    decimal(statistics.len),
                    decimal(information.total_statistics.len),
                    statistics.average_odds,
                    salty_bet_bot::money(statistics.average_bet),
                    (statistics.wins / (statistics.wins + statistics.losses)) * 100.0)
            }))
        })
    }


    fn make_dropdown<S>(mutable: Mutable<Rc<String>>, enabled: S, options: &[(&'static str, &'static str)]) -> Dom where S: Signal<Item = bool> + 'static {
        html!("select" => SelectElement, {
            .class(&*WIDGET)

            .property_signal("disabled", not(enabled))

            .with_element(|dom, element| {
                dom.event(clone!(mutable => move |_: ChangeEvent| {
                    if let Some(new_value) = element.value() {
                        mutable.set_neq(Rc::new(new_value));
                    }
                }))
            })

            .children(&mut options.into_iter().map(|(name, value)| {
                let value = *value;
                html!("option", {
                    .attribute("value", value)
                    .property_signal("selected", mutable.signal_cloned().map(move |x| &*x == value))
                    .text(name)
                })
            }).collect::<Vec<Dom>>())
        })
    }


    fn make_checkbox<S>(name: &str, value: Mutable<bool>, enabled: S) -> Dom where S: Signal<Item = bool> + 'static {
        lazy_static! {
            static ref CLASS_LABEL: String = class! {
                .style("color", "white")
                .style("text-shadow", TEXT_SHADOW)
            };

            static ref CLASS_INPUT: String = class! {
                .style("vertical-align", "bottom")
                .style("margin", "0px")
                .style("margin-right", "3px")
            };
        }

        html!("label", {
            .class(&*WIDGET)
            .class(&*CLASS_LABEL)

            .children(&mut [
                html!("input", {
                    .class(&*CLASS_INPUT)

                    .attribute("type", "checkbox")
                    .property_signal("disabled", not(enabled))
                    .property_signal("checked", value.signal())

                    .event(move |e: ChangeEvent| {
                        let node = e.target().unwrap();
                        let checked: bool = js!( return @{node}.checked; ).try_into().unwrap();
                        value.set_neq(checked);
                    })
                }),

                text(name),
            ])
        })
    }


    fn make_button<S, F>(name: &str, enabled: S, mut on_click: F) -> Dom
        where S: Signal<Item = bool> + 'static,
              F: FnMut() + 'static {
        html!("button", {
            .class(&*WIDGET)

            .property_signal("disabled", not(enabled))

            .event(move |_: ClickEvent| {
                on_click();
            })

            .text(name)
        })
    }


    let state = Rc::new(State::new(records));

    lazy_static! {
        static ref CLASS_ROOT: String = class! {
            .style("position", "absolute")
            .style("left", "5px")
            .style("top", "5px")
            .style("width", "calc(100% - 10px)")
            .style("height", "calc(100% - 10px)")
            .style("display", "flex")
        };

        static ref CLASS_ROW: String = class! {
            .style("display", "flex")
            .style("flex-direction", "row")
            .style("align-items", "flex-start")
            .style("justify-content", "flex-start")
        };

        static ref CLASS_COLUMN: String = class! {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("align-items", "flex-start")
            .style("justify-content", "flex-start")
        };

        static ref CLASS_INFO: String = class! {
            .style("position", "absolute")
            .style("left", "-5px")
            .style("top", "-5px")
            .style("z-index", "2")
            .style("background-color", BACKGROUND_COLOR)
            .style("white-space", "pre")
            .style("padding", "10px")
        };
    }

    html!("div", {
        .class(&*CLASS_ROOT)

        .children(&mut [
            //make_info_popup(state.clone()),

            svg_root(state.clone()),

            html!("div", {
                .class(&*CLASS_ROW)
                .class(&*CLASS_INFO)

                .event(clone!(state => move |_: MouseEnterEvent| {
                    state.hover_info.set_neq(true);
                }))

                .event(clone!(state => move |_: MouseLeaveEvent| {
                    state.hover_info.set_neq(false);
                }))

                .children(&mut [
                    html!("div", {
                        .class(&*CLASS_COLUMN)

                        .style("margin-bottom", "5px")
                        .style("margin-right", "15px")

                        .children(&mut [
                            make_dropdown(state.simulation_type.clone(), always(true), &[
                                ("RealData", "real-data"),
                                ("Genetic", "Genetic"),
                                ("ExpectedBetWinner", "expected-bet-winner"),
                                ("ExpectedBet", "expected-bet"),
                                ("WinnerBet", "winner-bet"),
                                ("ExpectedProfit", "earnings"),
                                ("Upsets", "upset-percentage"),
                                ("Odds", "upset-odds"),
                                ("OddsDifference", "upset-odds-difference"),
                                ("WinnerOdds", "upset-odds-winner"),
                                ("Bettors", "Bettors"),
                                ("IlluminatiBettors", "IlluminatiBettors"),
                                ("NormalBettors", "NormalBettors"),
                                ("Wins", "winrate-high"),
                                ("Losses", "winrate-low"),
                                ("Left", "random-left"),
                                ("Right", "random-right"),
                                ("Random", "random"),
                            ]),

                            make_dropdown(state.money_type.clone(), state.show_options(), &[
                                ("ExpectedBetWinner", "expected-bet-winner"),
                                ("ExpectedBet", "expected-bet"),
                                ("Percentage", "percentage"),
                                ("WinnerBet", "winner-bet"),
                                ("Fixed", "fixed"),
                                ("AllIn", "all-in"),
                            ]),

                            make_checkbox("Show only recent data", state.show_only_recent_data.clone(), always(true)),
                            make_checkbox("Use average for current money", state.average_sums.clone(), state.show_options()),
                            make_checkbox("Round to nearest magnitude", state.round_to_magnitude.clone(), state.show_options()),
                            make_checkbox("Reset money at regular intervals", state.reset_money.clone(), state.show_options()),
                            make_checkbox("Simulate extra data", state.extra_data.clone(), state.show_options()),

                            make_button("Run simulation", always(true), clone!(state => move || {
                                loading.show();

                                set_timeout(clone!(loading, state => move || {
                                    state.update();
                                    // TODO handle this better
                                    loading.hide();
                                }), 0);
                            })),
                        ])
                    }),

                    make_text(state.clone()),
                ])
            }),
        ])
    })
}


async fn main_future() -> Result<(), Error> {
    set_panic_hook();

    log!("Initializing...");

    stylesheet!("html, body", {
        .style("width", "100%")
        .style("height", "100%")
        .style("margin", "0px")
        .style("padding", "0px")
        .style("background-color", "#201d2b")
    });

    let loading = Loading::new();

    document().body().unwrap().append_child(loading.element());

    let mut records = await!(records_get_all())?;

    records.sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());

    dominator::append_dom(&dominator::body(), display_records(records, loading.clone()));

    loading.hide();

    Ok(())
}

fn main() {
    stdweb::initialize();

    spawn_local(unwrap_future(main_future()));

    stdweb::event_loop();
}
