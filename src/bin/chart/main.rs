#![recursion_limit="256"]

#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate dominator;
extern crate futures_signals;

use std::rc::Rc;
use salty_bet_bot::{records_get_all, subtract_days, add_days, decimal, Loading, set_panic_hook};
use algorithm::record::{Record, Profit, Mode};
use algorithm::simulation::{Bet, Simulation, Strategy, Simulator, SALT_MINE_AMOUNT};
use algorithm::strategy::{CustomStrategy, MoneyStrategy, BetStrategy, PERCENTAGE_THRESHOLD};
use stdweb::traits::*;
use stdweb::web::{document, set_timeout, Date};
use stdweb::web::html_element::SelectElement;
use stdweb::web::event::{ClickEvent, ChangeEvent, MouseMoveEvent};
use stdweb::unstable::TryInto;
use futures_signals::signal::{Mutable, SignalExt};
use dominator::{Dom, HIGHEST_ZINDEX, text};


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

        simulation.sum = PERCENTAGE_THRESHOLD;

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

                    simulation.calculate(&record, &bet);

                    let new_sum = simulation.tournament_sum;

                    if let Mode::Tournament = record.mode {
                        if let Some(amount) = bet.amount() {
                            if amount > 1.0 {
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

                        //if let Some(amount) = bet.amount() {
                            //if amount > 1.0 {
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
                            //}
                        //}
                    }

                    if !extra_data {
                        simulation.insert_record(&record);
                    }
                }
            },

            ChartMode::RealData { days } => {
                simulation.sum = SALT_MINE_AMOUNT;

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

                        //if let Some(amount) = bet.amount() {
                            //if amount > 1.0 {
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
                            //}
                        //}

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
                    // TODO is this correct ?
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

                    if let Some(amount) = bet.amount() {
                        min_bet = if min_bet == 0.0 { amount } else { min_bet.min(amount) };
                        max_bet = max_bet.max(amount);
                    }
                },
            }
        }

        //let len = information.len() as f64;

        Self {
            average_odds: average_odds / len,
            odds_gain: odds_gain / len,
            odds_loss: odds_loss / len,
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


fn display_records(records: Vec<Record>, loading: Loading) -> Dom {
    struct State {
        simulation_type: Mutable<Rc<String>>,
        money_type: Mutable<Rc<String>>,
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
                            "expected-bet-winner" => BetStrategy::ExpectedBetWinner,
                            "expected-bet" => BetStrategy::ExpectedBet,
                            "earnings" => BetStrategy::ExpectedProfit,
                            "winner-bet" => BetStrategy::WinnerBet,
                            "upset-percentage" => BetStrategy::Upsets,
                            "upset-odds" => BetStrategy::Odds,
                            "upset-odds-winner" => BetStrategy::WinnerOdds,
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
                let mut d_bets = vec![];
                let mut d_match_len = vec![];
                //let mut d_winner_profit = vec![];
                let mut d_tournaments = vec![];

                //let len = information.record_information.len() as f64;

                let y = (statistics.max_gain / (statistics.max_gain + statistics.max_loss)) * 100.0;
                //let y = (statistics.max_odds_gain / (statistics.max_odds_gain + statistics.max_odds_loss)) * 100.0;

                let mut first = true;

                for (_index, record) in information.record_information.iter().enumerate() {
                    //let x = normalize(index as f64, 0.0, len) * 100.0;
                    let x = normalize(record.date(), statistics.lowest_date, statistics.highest_date) * 100.0;

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

                    if first {
                        first = false;
                        d_money.push(format!("M{},{}", x, normalize(old_sum, statistics.max_money, statistics.min_money) * 100.0));
                    }

                    d_money.push(format!("L{},{}", x, normalize(new_sum, statistics.max_money, statistics.min_money) * 100.0));
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

                vec![
                    make_line(d_match_len, "black"),
                    make_line(d_gains, "hsla(120, 75%, 50%, 1)"),
                    // #6441a5
                    make_line(d_bets, "hsla(0, 100%, 50%, 1)"),
                    make_line(d_losses, "hsla(0, 75%, 50%, 1)"),
                    //make_line(d_winner_profit, "hsla(120, 75%, 50%, 1)"),
                    make_line(d_tournaments, "hsl(240, 100%, 75%)"),
                    make_line(d_money, "white"),
                ]
            }).to_signal_vec())
        })
    }


    fn make_info_popup(state: Rc<State>) -> Dom {
        lazy_static! {
            static ref CLASS: String = class! {
                .style("position", "absolute")
                .style("left", "0px")
                .style("top", "0px")
                .style("width", "2px")
                .style("height", "100%")
                .style("z-index", HIGHEST_ZINDEX)
                .style("background-color", "white")
                .style("pointer-events", "none")
            };
        }

        /*let hovered_record = self.hover_percentage.map(|percentage| {
                    log!("Percentage: {}, Length: {}, Index: {}", percentage, information.len() as f64, range_inclusive(percentage, 0.0, information.len() as f64).floor());
                });*/

        html!("div", {
            .class(&*CLASS)

            .style_signal("left", state.hover_percentage.signal().map(|percentage| {
                percentage.map(|percentage| {
                    format!("calc({}% - 1px)", percentage * 100.0)
                })
            }))

            .visible_signal(state.hover_percentage.signal().map(|percentage| percentage.is_some()))
        })
    }


    fn make_text(state: Rc<State>, align: &str) -> Dom {
        lazy_static! {
            static ref CLASS: String = class! {
                .style("position", "absolute")
                .style("left", "0px")
                .style("top", "0px")
                //.style("width", "100%")
                //.style("height", "100%")
                .style("color", "white")
                .style("text-shadow", TEXT_SHADOW)
                .style("background-color", BACKGROUND_COLOR)
                .style("padding", "5px")
                .style("font-size", "16px")
                .style("white-space", "pre")
            };
        }

        html!("div", {
            .class(&*CLASS)

            .style("text-align", align)

            .text_signal(state.information.signal_cloned().map(|information| {
                let statistics = &information.recent_statistics;
                let starting_money = information.record_information.first().map(|x| x.old_sum()).unwrap_or(0.0);
                let final_money = information.record_information.last().map(|x| x.new_sum()).unwrap_or(0.0);

                let total_gains = final_money - starting_money;
                let average_gains = total_gains / statistics.len;
                let average_money = final_money / information.total_statistics.len;

                format!("Starting money: {}\nFinal money: {}\nAverage money: {}\nTotal gains: {}\nMaximum: {}\nMinimum: {}\nMatches: {} (out of {})\nAverage gains: {}\nAverage odds: {}\nWinrate: {}%",
                    salty_bet_bot::money(starting_money),
                    salty_bet_bot::money(final_money),
                    salty_bet_bot::money(average_money),
                    salty_bet_bot::money(total_gains),
                    salty_bet_bot::money(statistics.max_money),
                    salty_bet_bot::money(statistics.min_money),
                    decimal(statistics.len),
                    decimal(information.total_statistics.len),
                    salty_bet_bot::money(average_gains),
                    statistics.average_odds,
                    (statistics.wins / (statistics.wins + statistics.losses)) * 100.0)
            }))
        })
    }


    fn make_dropdown(top: &str, mutable: Mutable<Rc<String>>, options: &[(&'static str, &'static str)]) -> Dom {
        html!("select" => SelectElement, {
            .style("position", "absolute")
            .style("left", "0px")
            .style("top", top)

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


    fn make_checkbox(name: &str, x: &str, y: &str, value: Mutable<bool>) -> Dom {
        lazy_static! {
            static ref CLASS: String = class! {
                .style("position", "absolute")
                .style("color", "white")
                .style("text-shadow", TEXT_SHADOW)
                .style("background-color", BACKGROUND_COLOR)
                .style("padding-right", "5px")
            };
        }

        html!("label", {
            .class(&*CLASS)
            .style("left", x)
            .style("top", y)

            .children(&mut [
                html!("input", {
                    .attribute("type", "checkbox")

                    .property_signal("checked", value.signal())

                    .style("vertical-align", "top")

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


    fn make_button<F>(name: &str, x: &str, y: &str, mut on_click: F) -> Dom where F: FnMut() + 'static {
        html!("button", {
            .style("position", "absolute")
            .style("left", x)
            .style("top", y)

            .event(move |_: ClickEvent| {
                on_click();
            })

            .text(name)
        })
    }


    loading.hide();

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
    }

    html!("div", {
        .class(&*CLASS_ROOT)

        .children(&mut [
            make_info_popup(state.clone()),

            svg_root(state.clone()),

            make_text(state.clone(), "left"),

            make_dropdown("220px", state.simulation_type.clone(), &[
                ("RealData", "real-data"),
                ("ExpectedBetWinner", "expected-bet-winner"),
                ("ExpectedBet", "expected-bet"),
                ("WinnerBet", "winner-bet"),
                ("ExpectedProfit", "earnings"),
                ("Upsets", "upset-percentage"),
                ("Odds", "upset-odds"),
                ("WinnerOdds", "upset-odds-winner"),
                ("Wins", "winrate-high"),
                ("Losses", "winrate-low"),
                ("Left", "random-left"),
                ("Right", "random-right"),
                ("Random", "random"),
            ]),

            make_dropdown("245px", state.money_type.clone(), &[
                ("ExpectedBetWinner", "expected-bet-winner"),
                ("ExpectedBet", "expected-bet"),
                ("Percentage", "percentage"),
                ("WinnerBet", "winner-bet"),
                ("Fixed", "fixed"),
                ("AllIn", "all-in"),
            ]),

            make_checkbox("Use average for current money", "0px", "270px", state.average_sums.clone()),
            make_checkbox("Show only recent data", "0px", "295px", state.show_only_recent_data.clone()),
            make_checkbox("Round to nearest magnitude", "0px", "320px", state.round_to_magnitude.clone()),
            make_checkbox("Reset money at regular intervals", "0px", "345px", state.reset_money.clone()),
            make_checkbox("Simulate extra data", "0px", "370px", state.extra_data.clone()),

            make_button("Run simulation", "0px", "395px", move || {
                loading.show();

                set_timeout(clone!(loading, state => move || {
                    state.update();
                    // TODO handle this better
                    loading.hide();
                }), 0);
            }),
        ])
    })
}


fn main() {
    stdweb::initialize();

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

    records_get_all(move |mut records| {
        records.sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());

        dominator::append_dom(&dominator::body(), display_records(records, loading));
    });

    stdweb::event_loop();
}
