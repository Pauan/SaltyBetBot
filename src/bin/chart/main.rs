#![recursion_limit="256"]

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
use std::collections::{BTreeSet, BTreeMap};
use salty_bet_bot::{records_get_all, spawn, subtract_days, decimal, Loading, set_panic_hook, find_starting_index, round_to_hour};
use algorithm::record::{Record, Profit, Mode};
use algorithm::simulation::{Bet, Simulation, Strategy, Simulator, TOURNAMENT_BALANCE};
use algorithm::strategy::{CustomStrategy, MoneyStrategy, BetStrategy, Permutate, MATCHMAKING_STRATEGY, TOURNAMENT_STRATEGY, FIXED_BET_AMOUNT};
use stdweb::traits::*;
use stdweb::spawn_local;
use stdweb::web::{document, set_timeout, Date, wait};
use stdweb::web::error::Error;
use stdweb::web::html_element::{InputElement, SelectElement};
use stdweb::web::event::{ClickEvent, ChangeEvent, MouseEnterEvent, MouseLeaveEvent};
use stdweb::unstable::TryInto;
use futures_signals::signal::{Mutable, Signal, SignalExt, and, not, always};
use dominator::{Dom, text};


const DEFAULT_DAYS_SHOWN: u32 = 365;

const MATCHMAKING_STARTING_MONEY: f64 = 10_000_000.0;


lazy_static! {
    static ref CURRENT_DATE: f64 = Date::now();
}


#[allow(dead_code)]
enum ChartMode<A> {
    SimulateTournament {
        strategy: A,
        reset_money: bool,
        extra_data: bool,
    },
    SimulateMatchmaking {
        strategy: A,
        reset_money: bool,
        extra_data: bool,
    },
    RealData {
        days: Option<u32>,
    },
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


#[derive(Debug, Clone)]
struct Hourly {
    date: f64,
    bettors: f64,
    bet_amount: f64,
}

#[derive(Debug)]
struct Match {
    date: f64,
    profit: Profit,
    old_sum: f64,
    new_sum: f64,
    match_len: f64,
    characters: (String, String),
    mode: Mode,
    won: bool,
    odds: Option<f64>,
    odds_winner: Option<Result<f64, f64>>,
    bet: Bet,
}

#[derive(Debug)]
enum RecordInformation {
    TournamentFinal {
        date: f64,
        profit: f64,
        old_sum: f64,
        new_sum: f64,
    },
    Match(Match),
}

impl RecordInformation {
    fn is_tournament(&self) -> bool {
        match *self {
            RecordInformation::TournamentFinal { .. } => true,
            RecordInformation::Match(_) => false,
        }
    }

    fn date(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { date, .. } => date,
            RecordInformation::Match(Match { date, .. }) => date,
        }
    }

    fn old_sum(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { old_sum, .. } => old_sum,
            RecordInformation::Match(Match { old_sum, .. }) => old_sum,
        }
    }

    fn new_sum(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { new_sum, .. } => new_sum,
            RecordInformation::Match(Match { new_sum, .. }) => new_sum,
        }
    }
}


#[derive(Debug)]
struct Information {
    is_tournament: bool,
    records: Vec<RecordInformation>,
    hourly: Vec<Hourly>,
    statistics: Statistics,
}

impl Information {
    fn new<A: Strategy>(records: &[Record], mode: ChartMode<A>, days_shown: Option<u32>) -> Self {
        fn add_hourly(rounded: &mut BTreeMap<u64, Hourly>, record: &Record) {
            let rounded_date = round_to_hour(record.date);

            // TODO verify that the `as u64` is correct
            let hourly = rounded.entry(rounded_date as u64).or_insert_with(|| Hourly {
                date: rounded_date,
                bettors: 0.0,
                bet_amount: 0.0,
            });

            hourly.bettors += record.left.bettors() + record.right.bettors();
            hourly.bet_amount += record.left.bet_amount + record.right.bet_amount;
        }


        let mut simulation: Simulation<A, A> = Simulation::new();


        let starting_date = days_shown.map(|days_shown| subtract_days(*CURRENT_DATE, days_shown));


        let is_tournament = match mode {
            ChartMode::SimulateTournament { .. } => true,
            _ => false,
        };


        let mut output: Vec<RecordInformation> = vec![];

        let mut hourly = BTreeMap::new();

        let mut total_len = 0.0;

        //simulation.sum = PERCENTAGE_THRESHOLD;

        match mode {
            ChartMode::SimulateTournament { strategy, reset_money, extra_data } => {
                simulation.tournament_strategy = Some(strategy);

                if extra_data {
                    for record in records.iter() {
                        simulation.insert_record(&record);
                    }
                }

                if reset_money {
                    simulation.sum = TOURNAMENT_BALANCE;
                }

                //let mut date_cutoff: Option<f64> = None;

                for record in records {
                    if let Mode::Tournament = record.mode {
                        total_len += 1.0;
                    }

                    if starting_date.map(|starting_date| record.date >= starting_date).unwrap_or(true) {
                        add_hourly(&mut hourly, record);

                        // TODO code duplication
                        /*if reset_money {
                            if let Some(starting_date) = starting_date {
                                if let Some(old_date) = date_cutoff {
                                    if record.date > old_date {
                                        simulation.sum = TOURNAMENT_BALANCE;
                                        date_cutoff = Some(add_days(old_date, days_shown));
                                    }

                                // TODO implement this more efficiently
                                } else {
                                    let mut ending_date = starting_date;

                                    while ending_date > record.date {
                                        ending_date = subtract_days(ending_date, days_shown);
                                    }

                                    date_cutoff = Some(add_days(ending_date, days_shown));
                                }
                            }
                        }*/

                        let old_sum = simulation.sum;
                        let old_tournament_sum = simulation.tournament_sum;

                        let match_len = simulation.min_matches_len(&record.left.name, &record.right.name);

                        let tournament_profit = simulation.tournament_profit(&record);

                        let bet = if let Mode::Tournament = record.mode {
                            simulation.bet(&record)

                        } else {
                            Bet::None
                        };

                        // TODO should this skip if it's Bet::None ?
                        simulation.calculate(&record, &bet);

                        let new_sum = simulation.sum;
                        let new_tournament_sum = simulation.tournament_sum;

                        if let Some(_amount) = bet.amount() {
                            let date = record.date;

                            let profit = Profit::from_old_new(old_tournament_sum, new_tournament_sum);

                            output.push(RecordInformation::Match(Match {
                                date,
                                profit,
                                old_sum: old_tournament_sum,
                                new_sum: new_tournament_sum,
                                match_len,
                                characters: (record.left.name.clone(), record.right.name.clone()),
                                mode: record.mode,
                                won: record.won(&bet),
                                odds: record.odds(&bet),
                                odds_winner: record.odds_winner(&bet),
                                bet,
                            }));
                        }

                        if let Some(tournament_profit) = tournament_profit {
                            let date = record.date;

                            assert_eq!(tournament_profit, (new_sum - old_sum));

                            output.push(RecordInformation::TournamentFinal {
                                date,
                                profit: tournament_profit,
                                old_sum: old_sum,
                                new_sum: new_sum,
                            });
                        }

                    } else {
                        simulation.skip(record);
                    }

                    if !extra_data {
                        simulation.insert_record(&record);
                    }
                }
            },

            ChartMode::SimulateMatchmaking { strategy, reset_money, extra_data } => {
                simulation.matchmaking_strategy = Some(strategy);

                //let mut index: f64 = 0.0;

                if extra_data {
                    for record in records.iter() {
                        simulation.insert_record(&record);
                    }
                }

                //let mut date_cutoff: Option<f64> = None;

                if reset_money {
                    simulation.sum = MATCHMAKING_STARTING_MONEY;
                }

                for record in records {
                    if let Mode::Matchmaking = record.mode {
                        total_len += 1.0;
                    }

                    let date = record.date;

                    if starting_date.map(|starting_date| date >= starting_date).unwrap_or(true) {
                        /*if reset_money {
                            if let Some(starting_date) = starting_date {
                                if let Some(old_date) = date_cutoff {
                                    if record.date > old_date {
                                        simulation.sum = MATCHMAKING_STARTING_MONEY;
                                        date_cutoff = Some(add_days(old_date, days_shown));
                                    }

                                // TODO implement this more efficiently
                                } else {
                                    let mut ending_date = starting_date;

                                    while ending_date > record.date {
                                        ending_date = subtract_days(ending_date, days_shown);
                                    }

                                    date_cutoff = Some(add_days(ending_date, days_shown));
                                }
                            }
                        }*/

                        add_hourly(&mut hourly, record);

                        if //simulation.min_matches_len(&record.left.name, &record.right.name) >= 10.0 &&
                           record.mode == Mode::Matchmaking {

                            let old_sum = simulation.sum;

                            let match_len = simulation.min_matches_len(&record.left.name, &record.right.name);

                            let tournament_profit = simulation.tournament_profit(&record);

                            let bet = simulation.bet(&record);

                            if let Some(_amount) = bet.amount() {
                                //if amount > 1.0 {
                                    simulation.calculate(&record, &bet);

                                    simulation.sum -= tournament_profit.unwrap_or(0.0);

                                    let new_sum = simulation.sum;

                                    simulation.insert_sum(new_sum);

                                    let profit = Profit::from_old_new(old_sum, new_sum);

                                    //let date = index;
                                    //index += 1.0;

                                    output.push(RecordInformation::Match(Match {
                                        date,
                                        profit,
                                        old_sum: old_sum,
                                        new_sum: new_sum,
                                        match_len,
                                        characters: (record.left.name.clone(), record.right.name.clone()),
                                        mode: record.mode,
                                        won: record.won(&bet),
                                        odds: record.odds(&bet),
                                        odds_winner: record.odds_winner(&bet),
                                        bet,
                                    }));
                                //}
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
                    total_len += 1.0;

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

                    add_hourly(&mut hourly, record);

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

                        if let Some(_amount) = bet.amount() {
                            //if amount > 1.0 {
                                simulation.calculate(&record, &bet);

                                if let Mode::Matchmaking = record.mode {
                                    let new_sum = simulation.sum;

                                    simulation.insert_sum(new_sum);

                                    let profit = Profit::from_old_new(old_sum, new_sum);

                                    output.push(RecordInformation::Match(Match {
                                        date,
                                        profit,
                                        old_sum: old_sum,
                                        new_sum: new_sum,
                                        match_len,
                                        characters: (record.left.name.clone(), record.right.name.clone()),
                                        mode: record.mode,
                                        won: record.won(&bet),
                                        odds: record.odds(&bet),
                                        odds_winner: record.odds_winner(&bet),
                                        bet,
                                    }));
                                }
                            //}
                        }

                    } else {
                        simulation.calculate(&record, &record.bet);
                    }

                    simulation.insert_record(&record);
                }
            },
        }

        let hourly: Vec<Hourly> = hourly.values().cloned().collect();

        Information {
            is_tournament,
            statistics: Statistics::new(&hourly, &output, is_tournament, total_len),
            records: output,
            hourly: hourly,
        }
    }

    fn starting_money(&self) -> f64 {
        self.records.iter().find(|x| {
            if self.is_tournament {
                x.is_tournament()
            } else {
                true
            }
        }).map(|x| x.old_sum()).unwrap_or(0.0)
    }

    fn final_money(&self) -> f64 {
        self.records.iter().rfind(|x| {
            if self.is_tournament {
                x.is_tournament()
            } else {
                true
            }
        }).map(|x| x.new_sum()).unwrap_or(0.0)
    }

    fn total_gains(&self) -> f64 {
        self.final_money() - self.starting_money()
    }
}


#[derive(Debug, Clone)]
struct Range {
    min: f64,
    max: f64,
}

impl Range {
    fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    fn normalize(&self, value: f64) -> f64 {
        normalize(value, self.min, self.max)
    }

    fn reverse(&self) -> Self {
        Self {
            min: self.max,
            max: self.min,
        }
    }
}


struct RangeBuilder {
    min: Option<f64>,
    max: Option<f64>,
}

impl RangeBuilder {
    fn new() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    fn set(&mut self, value: f64) {
        self.min = Some(match self.min {
            Some(min) => min.min(value),
            None => value,
        });

        self.max = Some(match self.max {
            Some(max) => max.max(value),
            None => value,
        });
    }

    fn into_range(self, default: f64) -> Range {
        Range {
            min: self.min.unwrap_or(default),
            max: self.max.unwrap_or(default),
        }
    }
}


#[derive(Debug, Clone)]
struct Statistics {
    len: f64,
    total_len: f64,
    wins: f64,
    upsets: f64,

    number_of_characters: usize,

    average_odds: f64,
    odds_gain: f64,
    odds_loss: f64,
    max_odds_gain: f64,
    max_odds_loss: f64,

    max_gain: f64,
    max_loss: f64,

    average_bet: f64,

    date: Range,
    bet: Range,
    money: Range,
    match_len: Range,
    bettors: Range,
    bet_amount: Range,
}

impl Statistics {
    fn new(hourly: &[Hourly], records: &[RecordInformation], is_tournament: bool, total_len: f64) -> Self {
        let mut matches_len: f64 = 0.0;
        let mut len: f64 = 0.0;
        let mut wins: f64 = 0.0;
        let mut upsets: f64 = 0.0;

        let mut seen_characters = BTreeSet::new();
        let mut number_of_characters: usize = 0;

        let mut average_odds: f64 = 0.0;
        let mut odds_gain: f64 = 0.0;
        let mut odds_loss: f64 = 0.0;
        let mut max_odds_gain: f64 = 0.0;
        let mut max_odds_loss: f64 = 0.0;

        let mut max_gain: f64 = 0.0;
        let mut max_loss: f64 = 0.0;

        let mut average_bet: f64 = 0.0;

        let mut date = RangeBuilder::new();
        let mut bet = RangeBuilder::new();
        let mut money = RangeBuilder::new();
        let mut match_len = RangeBuilder::new();
        let mut bettors = RangeBuilder::new();
        let mut bet_amount = RangeBuilder::new();

        for hourly in hourly {
            date.set(hourly.date);
            bettors.set(hourly.bettors);
            bet_amount.set(hourly.bet_amount);
        }

        for record in records {
            date.set(record.date());

            match record {
                RecordInformation::TournamentFinal { profit, old_sum, new_sum, .. } => {
                    assert!(*profit > 0.0);
                    assert!(new_sum > old_sum);

                    money.set(*old_sum);
                    money.set(*new_sum);

                    max_gain = max_gain.max(*profit);

                    len += 1.0;
                },
                RecordInformation::Match(m) => {
                    if !is_tournament {
                        money.set(m.old_sum);
                        money.set(m.new_sum);
                    }

                    if seen_characters.insert(&m.characters.0) {
                        number_of_characters += 1;
                    }

                    if seen_characters.insert(&m.characters.1) {
                        number_of_characters += 1;
                    }

                    if let Some(bet_amount) = m.bet.amount() {
                        // TODO is this correct ?
                        if bet_amount > 1.0 {
                            matches_len += 1.0;

                            if !is_tournament {
                                len += 1.0;

                                match m.profit {
                                    Profit::Gain(gain) => {
                                        max_gain = max_gain.max(gain);
                                    },
                                    Profit::Loss(loss) => {
                                        max_loss = max_loss.max(loss);
                                    },
                                    Profit::None => {},
                                }
                            }

                            match_len.set(m.match_len);

                            if m.won {
                                wins += 1.0;

                                if m.odds.unwrap() > 1.0 {
                                    upsets += 1.0;
                                }
                            }

                            match m.odds_winner.unwrap() {
                                Ok(amount) => {
                                    average_odds += amount;
                                    odds_gain += amount;
                                    max_odds_gain = max_odds_gain.max(amount);
                                },

                                Err(amount) => {
                                    average_odds += -1.0;
                                    odds_loss += -1.0;
                                    max_odds_loss = max_odds_loss.max(amount);
                                },
                            }

                            bet.set(bet_amount);
                            average_bet += bet_amount;
                        }
                    }
                },
            }
        }

        Self {
            average_odds: average_odds / matches_len,
            odds_gain: odds_gain / matches_len,
            odds_loss: odds_loss / matches_len,
            average_bet: average_bet / matches_len,
            wins: wins / matches_len,
            upsets: upsets / matches_len,
            number_of_characters: number_of_characters,
            max_odds_loss: max_odds_loss,
            max_odds_gain: max_odds_gain,
            total_len: total_len,
            len: len,
            max_gain: max_gain,
            max_loss: max_loss,
            bet: bet.into_range(0.0),
            money: money.into_range(0.0),
            match_len: match_len.into_range(0.0),
            bettors: bettors.into_range(0.0),
            bet_amount: bet_amount.into_range(0.0),
            date: date.into_range(0.0),
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
                Some(ref a) => simulation.pick_winner(a, &record.tier, &record.left.name, &record.right.name, record.date),
                None => Bet::None,
            }
        },

        Mode::Tournament => {
            simulation.in_tournament = true;
            simulation.tournament_sum = money;

            match simulation.tournament_strategy {
                Some(ref a) => simulation.pick_winner(a, &record.tier, &record.left.name, &record.right.name, record.date),
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


#[derive(Debug, PartialEq)]
enum SimulationType {
    RealData,
    BetStrategy(BetStrategy),
}

struct State {
    simulation_type: Mutable<Rc<SimulationType>>,
    days_shown: Mutable<Option<u32>>,
    money_type: Mutable<MoneyStrategy>,
    hover_percentage: Mutable<Option<f64>>,
    hover_info: Mutable<bool>,
    hover_popup: Mutable<bool>,
    average_sums: Mutable<bool>,
    round_to_magnitude: Mutable<bool>,
    scale_by_matches: Mutable<bool>,
    scale_by_money: Mutable<bool>,
    extra_data: Mutable<bool>,
    reset_money: Mutable<bool>,
    simulation_mode: Mutable<Mode>,
    information: Mutable<Option<Rc<Information>>>,
    records: Vec<Record>,
}

impl State {
    fn new(records: Vec<Record>) -> Self {
        let this = Self {
            simulation_type: Mutable::new(Rc::new(SimulationType::RealData)),
            days_shown: Mutable::new(Some(DEFAULT_DAYS_SHOWN)),
            money_type: Mutable::new(MoneyStrategy::AllIn),
            hover_percentage: Mutable::new(None),
            hover_info: Mutable::new(false),
            hover_popup: Mutable::new(false),
            average_sums: Mutable::new(false),
            round_to_magnitude: Mutable::new(false),
            scale_by_matches: Mutable::new(false),
            scale_by_money: Mutable::new(false),
            extra_data: Mutable::new(false),
            reset_money: Mutable::new(false),
            simulation_mode: Mutable::new(Mode::Matchmaking),
            information: Mutable::new(None),
            records,
        };

        this.reset_simulation_mode();

        this
    }

    fn reset_simulation_mode(&self) {
        let mode = self.simulation_mode.get();

        let strategy = if let Mode::Matchmaking = mode {
            MATCHMAKING_STRATEGY

        } else {
            TOURNAMENT_STRATEGY
        };

        self.simulation_type.set(Rc::new(SimulationType::BetStrategy(strategy.bet)));
        self.money_type.set_neq(strategy.money);
        self.average_sums.set_neq(strategy.average_sums);

        self.days_shown.set_neq(if let Mode::Matchmaking = mode {
            Some(DEFAULT_DAYS_SHOWN)
        } else {
            None
        });

        self.round_to_magnitude.set_neq(strategy.round_to_magnitude);
        self.scale_by_matches.set_neq(strategy.scale_by_matches);
        self.scale_by_money.set_neq(strategy.scale_by_money);

        self.reset_money.set_neq(if let Mode::Matchmaking = mode {
            true
        } else {
            false
        });
    }

    fn average_sums(&self) -> bool {
        if self.simulation_mode.get() == Mode::Tournament {
            false

        } else {
            self.average_sums.get()
        }
    }

    async fn find_best(&self) {
        let reset_money = self.reset_money.get();
        let extra_data = self.extra_data.get();

        let average_sums = self.average_sums();
        let scale_by_matches = self.scale_by_matches.get();
        let scale_by_money = self.scale_by_money.get();
        let round_to_magnitude = self.round_to_magnitude.get();
        let simulation_mode = self.simulation_mode.get();

        let mut strategies = vec![];

        Permutate::each(|money: MoneyStrategy| {
            Permutate::each(|bet: BetStrategy| {
                strategies.push(CustomStrategy {
                    average_sums,
                    scale_by_matches,
                    scale_by_money,
                    scale_by_time: None,
                    round_to_magnitude,
                    bet,
                    money,
                });
            });
        });

        let mut results = Vec::with_capacity(strategies.len());

        // TODO pre-calculate up to the recent data
        for strategy in strategies {
            let information = Information::new(
                &self.records,
                match simulation_mode {
                    Mode::Matchmaking => ChartMode::SimulateMatchmaking {
                        reset_money,
                        extra_data,
                        // TODO figure out a way to avoid this clone
                        strategy: strategy.clone(),
                    },
                    Mode::Tournament => ChartMode::SimulateTournament {
                        reset_money,
                        extra_data,
                        // TODO figure out a way to avoid this clone
                        strategy: strategy.clone(),
                    },
                },
                self.days_shown.get(),
            );

            let total_gains = information.total_gains();

            results.push((total_gains, strategy));

            wait(0).await;
        }

        results.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());

        for (total_gains, strategy) in &results[(results.len() - 10)..] {
            log!("{} {:#?}", total_gains, strategy);
        }

        if let Some((_, strategy)) = results.pop() {
            self.average_sums.set_neq(strategy.average_sums);
            self.scale_by_matches.set_neq(strategy.scale_by_matches);
            self.scale_by_money.set_neq(strategy.scale_by_money);
            self.round_to_magnitude.set_neq(strategy.round_to_magnitude);
            self.money_type.set_neq(strategy.money);
            self.simulation_type.set_neq(Rc::new(SimulationType::BetStrategy(strategy.bet)));
            //self.information.set(Rc::new(information));
            self.update();
        }
    }

    fn get_custom_strategy(&self) -> Option<CustomStrategy> {
        match **self.simulation_type.lock_ref() {
            SimulationType::RealData => {
                None
            },

            SimulationType::BetStrategy(ref bet_strategy) => {
                Some(CustomStrategy {
                    average_sums: self.average_sums(),
                    scale_by_matches: self.scale_by_matches.get(),
                    scale_by_money: self.scale_by_money.get(),
                    scale_by_time: None,
                    round_to_magnitude: self.round_to_magnitude.get(),
                    money: self.money_type.get(),
                    // TODO figure out a way to avoid this clone ?
                    bet: bet_strategy.clone(),
                })
            },
        }
    }

    fn calculate_custom_strategy(&self, strategy: CustomStrategy) -> f64 {
        let information = Information::new(
            &self.records,
            match self.simulation_mode.get() {
                Mode::Matchmaking => ChartMode::SimulateMatchmaking {
                    reset_money: self.reset_money.get(),
                    extra_data: self.extra_data.get(),
                    strategy,
                },
                Mode::Tournament => ChartMode::SimulateTournament {
                    reset_money: self.reset_money.get(),
                    extra_data: self.extra_data.get(),
                    strategy,
                },
            },
            self.days_shown.get(),
        );

        let gains = information.total_gains();

        self.information.set(Some(Rc::new(information)));

        gains
    }

    fn update(&self) {
        if let Some(strategy) = self.get_custom_strategy() {
            self.calculate_custom_strategy(strategy);

        } else {
            let real: ChartMode<()> = ChartMode::RealData { days: None };

            let information = Information::new(
                &self.records,
                real,
                self.days_shown.get(),
            );

            self.information.set(Some(Rc::new(information)));
        }

        //ChartMode::RealData { days: Some(7), matches: None }
        //ChartMode::RealData { days: None }
        //ChartMode::SimulateMatchmaking(EarningsStrategy { expected_profit, winrate, bet_difference: false, winrate_difference: false, use_percentages })
        //ChartMode::SimulateMatchmaking(matchmaking_strategy())
    }

    fn show_options(&self) -> impl Signal<Item = bool> {
        self.simulation_type.signal_ref(|x| **x != SimulationType::RealData)
    }

    fn is_tournament(&self) -> impl Signal<Item = bool> {
        self.simulation_mode.signal_ref(|x| *x == Mode::Tournament)
    }
}


fn display_records(records: Vec<Record>, loading: Loading) -> Dom {
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
                dom.global_event(clone!(state => move |e: ClickEvent| {
                    if state.hover_popup.get() {
                        // Do nothing

                    } else if state.hover_info.get() || state.hover_percentage.lock_ref().is_some() {
                        state.hover_percentage.set_neq(None);

                    } else {
                        // TODO don't hardcode this
                        let x = (e.client_x() as f64) - 5.0;
                        // TODO use get_bounding_client_rect instead
                        let width: f64 = js!( return @{&element}.clientWidth; ).try_into().unwrap();

                        let percentage = (x / width).max(0.0).min(1.0);

                        state.hover_percentage.set_neq(Some(percentage));
                    }
                }))
            })

            .children_signal_vec(state.information.signal_cloned().map(|information| {
                if let Some(information) = information {
                    let statistics = &information.statistics;

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
                    let mut d_bettors = vec!["M0,100".to_owned()];
                    let mut d_bet_amount = vec!["M0,100".to_owned()];

                    //let len = information.record_information.len();

                    let y = (statistics.max_gain / (statistics.max_gain + statistics.max_loss)) * 100.0;
                    //let y = (statistics.max_odds_gain / (statistics.max_odds_gain + statistics.max_odds_loss)) * 100.0;

                    let mut first = true;

                    let mut smooth_sums = vec![];

                    /*for hourly in information.hourly.iter() {
                        let x = statistics.date.normalize(hourly.date) * 100.0;
                        d_bettors.push(format!("L{},{}", x, statistics.bettors.reverse().normalize(hourly.bettors) * 100.0));
                        d_bet_amount.push(format!("L{},{}", x, statistics.bet_amount.reverse().normalize(hourly.bet_amount) * 100.0));
                    }*/

                    for (_index, record) in information.records.iter().enumerate() {
                        let date = record.date();

                        //let x = normalize(index as f64, 0.0, len) * 100.0;
                        let x = statistics.date.normalize(date) * 100.0;

                        let (old_sum, new_sum) = match record {
                            RecordInformation::TournamentFinal { profit, .. } => {
                                // TODO code duplication with the Statistics
                                d_tournaments.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(*profit, 0.0, statistics.max_gain), 100.0, 0.0), x, 100.0));
                                continue;
                                //(*old_sum, *new_sum)
                            },
                            RecordInformation::Match(m) => {
                                if let Mode::Matchmaking = m.mode {
                                    d_match_len.push(format!("M{},{}L{},{}",
                                        x,
                                        100.0,
                                        x,
                                        normalize(m.match_len, statistics.match_len.max, 0.0) * 100.0));

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

                                    match m.profit {
                                        Profit::Gain(amount) => {
                                            d_gains.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(amount, 0.0, statistics.max_gain), y, 0.0), x, y));

                                            if let Some(amount) = m.bet.amount() {
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

                                    (m.old_sum, m.new_sum)

                                } else {
                                    continue;
                                }
                            },
                        };

                        smooth_sums.push((x, date, new_sum));

                        if first {
                            first = false;
                            d_money.push(format!("M{},{}", x, statistics.money.reverse().normalize(old_sum) * 100.0));
                        }

                        d_money.push(format!("L{},{}", x, statistics.money.reverse().normalize(new_sum) * 100.0));
                    }

                    fn smooth(x: f64, statistics: &Statistics, d_smooth: &mut Vec<String>, smooth_sums: &[(f64, f64, f64)], date_range: f64, date: f64) {
                        let half_range = date_range / 2.0;

                        let date_start = date - half_range;
                        let date_end = date + half_range;

                        let index = find_starting_index(smooth_sums, |(_, date, _)| date.partial_cmp(&date_start).unwrap());

                        let mut sum = 0.0;
                        let mut len = 0.0;

                        for (_, date, new_sum) in &smooth_sums[index..] {
                            assert!(*date >= date_start);

                            if *date <= date_end {
                                sum += new_sum;
                                len += 1.0;

                            } else {
                                break;
                            }
                        }

                        let average = sum / len;

                        if d_smooth.len() == 0 {
                            d_smooth.push(format!("M{},{}", x, statistics.money.reverse().normalize(average) * 100.0));

                        } else {
                            d_smooth.push(format!("L{},{}", x, statistics.money.reverse().normalize(average) * 100.0));
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

                        make_line(d_bettors, "hsl(0, 100%, 50%)"),
                        make_line(d_bet_amount, "hsl(180, 100%, 50%)"),
                    ]

                } else {
                    vec![]
                }
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
                .style("z-index", "3")
                .style("background-color", "white")
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

            .event(clone!(state => move |_: MouseEnterEvent| {
                state.hover_popup.set_neq(true);
            }))

            .event(clone!(state => move |_: MouseLeaveEvent| {
                state.hover_popup.set_neq(false);
            }))

            .style_signal("left", state.hover_percentage.signal().map(|percentage| {
                percentage.map(|percentage| {
                    format!("{}%", percentage * 100.0)
                })
            }))

            .visible_signal(state.hover_percentage.signal().map(|percentage| percentage.is_some()))

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
                                if let Some(information) = information {
                                    let first_date = information.records.first().map(|x| x.date()).unwrap_or(0.0);
                                    let last_date = information.records.last().map(|x| x.date()).unwrap_or(0.0);

                                    let date = range_inclusive(*percentage, first_date, last_date);

                                    let mut index = None;
                                    let mut difference = INFINITY;

                                    // TODO rewrite this with combinators ?
                                    // TODO make this more efficient
                                    for (i, record) in information.records.iter().enumerate() {
                                        let diff = (record.date() - date).abs();

                                        if diff < difference {
                                            difference = diff;
                                            index = Some(i);
                                        }
                                    }

                                    //let index = information.binary_search_by(|a| a.date().partial_cmp(&date).unwrap());

                                    if let Some(index) = index {
                                        let record = &information.records[index];
                                        format!("{}\n{:#?}", index, record)

                                    } else {
                                        "".to_string()
                                    }

                                    //format!("{}\n{:#?}\n{:#?}\n{:#?}\n{:#?}\n{:#?}", information.record_information.len(), first_date, date, percentage, index, difference)

                                } else {
                                    "".to_string()
                                }

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
                if let Some(information) = information {
                    let statistics = &information.statistics;
                    let starting_money = information.starting_money();
                    let final_money = information.final_money();

                    let total_gains = final_money - starting_money;
                    let average_gains = total_gains / statistics.len;

                    format!("\
                        Total gains: {total_gains}\n\
                        Minimum: {min_money}\n\
                        Maximum: {max_money}\n\
                        Starting money: {starting_money}\n\
                        Final money: {final_money}\n\
                        Average gains: {average_gains}\n\
                        Maximum gain: {max_gain}\n\
                        Matches: {matches} (out of {total_matches})\n\
                        Number of characters: {characters}\n\
                        Average odds: {average_odds}\n\
                        Average bet: {average_bet}\n\
                        Winrate: {winrate}%\n\
                        Upsets: {upsets}%",
                        min_money = salty_bet_bot::money(statistics.money.min),
                        max_money = salty_bet_bot::money(statistics.money.max),
                        starting_money = salty_bet_bot::money(starting_money),
                        final_money = salty_bet_bot::money(final_money),
                        total_gains = salty_bet_bot::money(total_gains),
                        average_gains = salty_bet_bot::money(average_gains),
                        max_gain = salty_bet_bot::money(statistics.max_gain),

                        // TODO are these correct ?
                        matches = decimal(statistics.len),
                        total_matches = decimal(statistics.total_len),

                        characters = statistics.number_of_characters,
                        average_odds = statistics.average_odds,
                        average_bet = salty_bet_bot::money(statistics.average_bet),
                        winrate = statistics.wins * 100.0,
                        upsets = statistics.upsets * 100.0)

                } else {
                    "".to_string()
                }
            }))
        })
    }


    fn make_dropdown<A, S, F>(mutable: Mutable<A>, enabled: S, options: &[&'static str], mut f: F) -> Dom
        where A: Clone + PartialEq + 'static,
              S: Signal<Item = bool> + 'static,
              F: FnMut(String) -> A + 'static {
        html!("select" => SelectElement, {
            .class(&*WIDGET)

            .property_signal("disabled", not(enabled))

            .children(&mut options.into_iter().map(|name| {
                let value = f(name.to_string());
                html!("option", {
                    .property_signal("selected", mutable.signal_cloned().map(move |x| x == value))
                    .text(name)
                })
            }).collect::<Vec<Dom>>())

            .with_element(|dom, element| {
                dom.event(clone!(mutable => move |_: ChangeEvent| {
                    if let Some(new_value) = element.value() {
                        mutable.set_neq(f(new_value));
                    }
                }))
            })
        })
    }


    fn make_label(children: &mut [Dom]) -> Dom {
        lazy_static! {
            static ref CLASS_LABEL: String = class! {
                .style("color", "white")
                .style("text-shadow", TEXT_SHADOW)
            };
        }

        html!("label", {
            .class(&*WIDGET)
            .class(&*CLASS_LABEL)

            .children(children)
        })
    }

    fn make_checkbox<S>(name: &str, value: Mutable<bool>, enabled: S) -> Dom where S: Signal<Item = bool> + 'static {
        lazy_static! {
            static ref CLASS_INPUT: String = class! {
                .style("vertical-align", "bottom")
                .style("margin", "0px")
                .style("margin-right", "3px")
            };
        }

        make_label(&mut [
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

    state.update();

    js! { @(no_return)
        window.find_best = @{clone!(state, loading => move || {
            spawn_local(clone!(state, loading => async move {
                loading.show();
                wait(0).await;
                state.find_best().await;
                // TODO handle this better
                loading.hide();
            }));
        })};

        window.calculate_custom_strategy = @{clone!(state, loading => move |strategy: String| {
            loading.show();

            set_timeout(clone!(loading, state => move || {
                match serde_json::from_str(&strategy) {
                    Ok(strategy) => {
                        console!(log, salty_bet_bot::money(state.calculate_custom_strategy(strategy)));
                    },
                    Err(e) => {
                        console!(error, e.to_string());
                    },
                }

                // TODO handle this better
                loading.hide();
            }), 0);
        })};

        window.get_custom_strategy = @{clone!(state => move || {
            if let Some(strategy) = state.get_custom_strategy() {
                console!(log, serde_json::to_string_pretty(&strategy).unwrap());
            }
        })};
    }

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
            make_info_popup(state.clone()),

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

                        .future(state.simulation_mode.signal().for_each(clone!(state => move |_| {
                            state.reset_simulation_mode();
                            async {}
                        })))

                        .children(&mut [
                            make_dropdown(state.simulation_mode.clone(), always(true), &[
                                "Matchmaking",
                                "Tournament",
                            ], |value| {
                                match value.as_str() {
                                    "Matchmaking" => Mode::Matchmaking,
                                    "Tournament" => Mode::Tournament,
                                    a => panic!("Invalid value {}", a),
                                }
                            }),

                            make_dropdown(state.money_type.clone(), state.show_options(), &[
                                "ExpectedBetWinner",
                                "ExpectedBet",
                                "Percentage",
                                "BetDifference",
                                "BetDifferenceWinner",
                                "Fixed",
                                "AllIn",
                                "Matchmaking",
                                "Tournament",
                                "UpsetsElo",
                            ], |value| {
                                match value.as_str() {
                                    "ExpectedBetWinner" => MoneyStrategy::ExpectedBetWinner,
                                    "ExpectedBet" => MoneyStrategy::ExpectedBet,
                                    "Percentage" => MoneyStrategy::Percentage,
                                    "BetDifference" => MoneyStrategy::BetDifference,
                                    "BetDifferenceWinner" => MoneyStrategy::BetDifferenceWinner,
                                    "Fixed" => MoneyStrategy::Fixed(FIXED_BET_AMOUNT),
                                    "AllIn" => MoneyStrategy::AllIn,
                                    "Matchmaking" => MoneyStrategy::Matchmaking { max_bet: FIXED_BET_AMOUNT },
                                    "Tournament" => MoneyStrategy::Tournament,
                                    "UpsetsElo" => MoneyStrategy::UpsetsElo { max_bet: FIXED_BET_AMOUNT },
                                    a => panic!("Invalid value {}", a),
                                }
                            }),

                            make_dropdown(state.simulation_type.clone(), always(true), &[
                                "RealData",
                                //"Genetic",
                                "ExpectedBetWinner",
                                "ExpectedBet",
                                "BetDifference",
                                "BetDifferenceWinner",
                                "ExpectedProfit",
                                "Upsets",
                                "Odds",
                                "OddsDifference",
                                "WinnerOdds",
                                "Bettors",
                                "BettorsRatio",
                                "IlluminatiBettors",
                                "NormalBettors",
                                "BetAmount",
                                "BetPercentage",
                                "Wins",
                                "Losses",
                                "Left",
                                "Right",
                                "Random",
                                "Elo",
                                "UpsetsElo",
                                "Matchmaking",
                                "Tournament",
                                "Money",
                            ], |value| {
                                Rc::new(match value.as_str() {
                                    "RealData" => SimulationType::RealData,
                                    a => SimulationType::BetStrategy(match a {
                                        // TODO avoid the clone somehow
                                        //"Genetic" => BetStrategy::Genetic(GENETIC_STRATEGY.clone()),
                                        "ExpectedBetWinner" => BetStrategy::ExpectedBetWinner,
                                        "ExpectedBet" => BetStrategy::ExpectedBet,
                                        "BetDifference" => BetStrategy::BetDifference,
                                        "BetDifferenceWinner" => BetStrategy::BetDifferenceWinner,
                                        "ExpectedProfit" => BetStrategy::ExpectedProfit,
                                        "Upsets" => BetStrategy::Upsets,
                                        "Odds" => BetStrategy::Odds,
                                        "OddsDifference" => BetStrategy::OddsDifference,
                                        "WinnerOdds" => BetStrategy::WinnerOdds,
                                        "Bettors" => BetStrategy::Bettors,
                                        "BettorsRatio" => BetStrategy::BettorsRatio,
                                        "IlluminatiBettors" => BetStrategy::IlluminatiBettors,
                                        "NormalBettors" => BetStrategy::NormalBettors,
                                        "BetAmount" => BetStrategy::BetAmount,
                                        "BetPercentage" => BetStrategy::BetPercentage,
                                        "Wins" => BetStrategy::Wins,
                                        "Losses" => BetStrategy::Losses,
                                        "Left" => BetStrategy::Left,
                                        "Right" => BetStrategy::Right,
                                        "Random" => BetStrategy::Random,
                                        "Elo" => BetStrategy::Elo,
                                        "UpsetsElo" => BetStrategy::UpsetsElo,
                                        "Matchmaking" => BetStrategy::Matchmaking,
                                        "Tournament" => BetStrategy::Tournament,
                                        "Money" => BetStrategy::Money,
                                        a => panic!("Invalid value {}", a),
                                    }),
                                })
                            }),

                            make_label(&mut [
                                html!("input" => InputElement, {
                                    .attribute("type", "number")
                                    .attribute("min", "1")
                                    .attribute("step", "1")

                                    .property_signal("value", state.days_shown.signal()
                                        .map(|x| {
                                            x.map(|x| x.to_string()).unwrap_or_else(|| "".to_string())
                                        }))

                                    .style("width", "55px")
                                    .style("height", "20px")
                                    .style("text-align", "left")
                                    .style("padding", "0px 4px")
                                    .style("box-sizing", "border-box")

                                    .with_element(|dom, element| {
                                        dom.event(clone!(state => move |_: ChangeEvent| {
                                            let value = element.raw_value();
                                            let value = value.trim();

                                            if value == "" {
                                                state.days_shown.set(None);

                                            } else {
                                                if let Ok(days) = value.parse() {
                                                    state.days_shown.set(Some(days));

                                                } else {
                                                    // TODO hacky, should have an `invalidate` method or similar
                                                    state.days_shown.set(state.days_shown.get());
                                                }
                                            }
                                        }))
                                    })
                                }),
                                text(" days shown"),
                            ]),

                            make_checkbox("Use average for current money", state.average_sums.clone(), and(state.show_options(), not(state.is_tournament()))),
                            make_checkbox("Round to nearest magnitude", state.round_to_magnitude.clone(), state.show_options()),
                            make_checkbox("Scale by match length", state.scale_by_matches.clone(), state.show_options()),
                            make_checkbox("Scale by money", state.scale_by_money.clone(), state.show_options()),
                            make_checkbox("Reset money at regular intervals", state.reset_money.clone(), state.show_options()),
                            make_checkbox("Simulate extra data", state.extra_data.clone(), state.show_options()),

                            make_button("Run simulation", always(true), clone!(state, loading => move || {
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

    let mut records = records_get_all().await?;

    records.sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());

    dominator::append_dom(&dominator::body(), display_records(records, loading.clone()));

    loading.hide();

    Ok(())
}

fn main() {
    stdweb::initialize();

    spawn(main_future());

    stdweb::event_loop();
}
