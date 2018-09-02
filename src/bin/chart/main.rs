#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;

use std::rc::Rc;
use std::cell::{Cell, RefCell};
use salty_bet_bot::{records_get_all, subtract_days, decimal};
use algorithm::record::{Record, Profit, Mode};
use algorithm::simulation::{Bet, Simulation, Strategy, SALT_MINE_AMOUNT};
use algorithm::strategy::{EarningsStrategy, RandomStrategy, UpsetStrategy, WinrateStrategy, HybridStrategy, PERCENTAGE_THRESHOLD};
use stdweb::traits::*;
use stdweb::web::{document, Element};
use stdweb::web::html_element::SelectElement;
use stdweb::web::event::ChangeEvent;
use stdweb::unstable::TryInto;


#[allow(dead_code)]
enum ChartMode<A> {
    SimulateTournament(A),
    SimulateMatchmaking(A),
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
        mode: Mode,
        won: bool,
        odds: Result<f64, f64>,
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

                    simulation.calculate(&record, &bet);

                    let new_sum = simulation.tournament_sum;

                    if let Mode::Tournament = record.mode {
                        if let Some(odds) = record.odds(&bet) {
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
                                        mode: record.mode,
                                        won: record.won(&bet),
                                        odds,
                                        bet,
                                    });
                                }
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

            ChartMode::SimulateMatchmaking(strategy) => {
                simulation.matchmaking_strategy = Some(strategy);

                let mut index: f64 = 0.0;

                if extra_data {
                    for record in records.iter() {
                        simulation.insert_record(&record);
                    }
                }

                for record in records {
                    if //simulation.min_matches_len(&record.left.name, &record.right.name) >= 10.0 &&
                       record.mode == Mode::Matchmaking {

                        let old_sum = simulation.sum;

                        let tournament_profit = simulation.tournament_profit(&record);

                        let bet = simulation.bet(&record);

                        simulation.calculate(&record, &bet);

                        simulation.sum -= tournament_profit.unwrap_or(0.0);

                        let new_sum = simulation.sum;

                        let profit = Profit::from_old_new(old_sum, new_sum);

                        if let Some(odds) = record.odds(&bet) {
                            if let Some(amount) = bet.amount() {
                                if amount > 1.0 {
                                    let date = index;
                                    index += 1.0;

                                    output.push(RecordInformation::Match {
                                        date,
                                        profit,
                                        old_sum: old_sum,
                                        new_sum: new_sum,
                                        mode: record.mode,
                                        won: record.won(&bet),
                                        odds,
                                        bet,
                                    });
                                }
                            }
                        }
                    }

                    if !extra_data {
                        simulation.insert_record(&record);
                    }
                }
            },

            ChartMode::RealData { days } => {
                simulation.sum = SALT_MINE_AMOUNT;

                // TODO
                let days: Option<f64> = days.map(|days| subtract_days(days));

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

                        /*if let Some(tournament_profit) = tournament_profit {
                            let new_sum = old_sum + tournament_profit;

                            output.push(RecordInformation::TournamentFinal {
                                date,
                                profit: tournament_profit,
                                old_sum: old_sum,
                                new_sum: new_sum,
                            });
                        }*/

                        let old_sum = old_sum + tournament_profit.unwrap_or(0.0);

                        let bet = record.bet.clone();

                        simulation.calculate(&record, &bet);

                        let new_sum = simulation.sum;

                        let profit = Profit::from_old_new(old_sum, new_sum);

                        if let Mode::Matchmaking = record.mode {
                            if let Some(odds) = record.odds(&bet) {
                                if let Some(amount) = bet.amount() {
                                    if amount > 1.0 {
                                        output.push(RecordInformation::Match {
                                            date,
                                            profit,
                                            old_sum: old_sum,
                                            new_sum: new_sum,
                                            mode: record.mode,
                                            won: record.won(&bet),
                                            odds,
                                            bet,
                                        });
                                    }
                                }
                            }
                        }

                    } else {
                        simulation.calculate(&record, &record.bet);
                    }
                }
            },
        }

        output.sort_by(|a, b| a.date().partial_cmp(&b.date()).unwrap());

        output
    }

    fn date(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { date, .. } => date,
            RecordInformation::Match { date, .. } => date,
        }
    }

    fn new_sum(&self) -> f64 {
        match *self {
            RecordInformation::TournamentFinal { new_sum, .. } => new_sum,
            RecordInformation::Match { new_sum, .. } => new_sum,
        }
    }
}


#[derive(Debug)]
struct Statistics {
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

    lowest_date: f64,
    highest_date: f64,
}

impl Statistics {
    fn new(records: &[RecordInformation]) -> Self {
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

        let mut min_money: f64 = 0.0;
        let mut max_money: f64 = 0.0;

        let mut lowest_date: f64 = 0.0;
        let mut highest_date: f64 = 0.0;

        for record in records {
            let date = record.date();
            let new_sum = record.new_sum();

            lowest_date = if lowest_date == 0.0 { date } else { lowest_date.min(date) };
            highest_date = highest_date.max(date);

            min_money = if min_money == 0.0 { new_sum } else { min_money.min(new_sum) };
            max_money = max_money.max(new_sum);

            match record {
                RecordInformation::TournamentFinal { profit, .. } => {
                    assert!(*profit > 0.0);
                    max_gain = max_gain.max(*profit);
                },
                RecordInformation::Match { profit, won, odds, bet, .. } => {
                    match profit {
                        Profit::Gain(gain) => {
                            max_gain = max_gain.max(*gain);
                        },
                        Profit::Loss(loss) => {
                            max_loss = max_loss.max(*loss);
                        },
                        Profit::None => {},
                    }

                    if *won {
                        wins += 1.0;

                    } else {
                        losses += 1.0;
                    }

                    match odds {
                        Ok(amount) => {
                            average_odds += amount;
                            odds_gain += amount;
                            max_odds_gain = max_odds_gain.max(*amount);
                        },

                        Err(amount) => {
                            average_odds += -1.0;
                            odds_loss += -1.0;
                            max_odds_loss = max_odds_loss.max(*amount);
                        },
                    }

                    if let Some(amount) = bet.amount() {
                        min_bet = if min_bet == 0.0 { amount } else { min_bet.min(amount) };
                        max_bet = max_bet.max(amount);
                    }
                },
            }
        }

        let len = records.len() as f64;

        Self {
            average_odds: average_odds / len,
            odds_gain: odds_gain / len,
            odds_loss: odds_loss / len,
            max_odds_loss, max_odds_gain, wins, losses, max_gain, max_loss, max_bet, min_bet, max_money, min_money, lowest_date, highest_date,
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


fn make_checkbox<F>(name: &str, x: &str, y: &str, value: bool, mut callback: F) -> Element where F: FnMut(bool) + 'static {
    let node = document().create_element("label").unwrap();

    js! { @(no_return)
        var node = @{&node};
        node.style.position = "absolute";
        node.style.right = @{x};
        node.style.top = @{y};
        node.style.color = "white";
        node.style.textShadow = @{TEXT_SHADOW};
        node.style.backgroundColor = @{BACKGROUND_COLOR};
        node.style.paddingRight = "5px";
    }

    node.append_child(&{
        let node = document().create_element("input").unwrap();

        js! { @(no_return)
            var node = @{&node};
            node.type = "checkbox";
            node.checked = @{value};
            node.style.verticalAlign = "top";
        }

        node.add_event_listener(move |e: ChangeEvent| {
            let node = e.target().unwrap();

            callback(js!( return @{node}.checked; ).try_into().unwrap());
        });

        node
    });

    node.append_child(&document().create_text_node(name));

    node
}

fn display_records(node: &Element, records: Vec<Record>) {
    fn svg(name: &str) -> Element {
        js!( return document.createElementNS("http://www.w3.org/2000/svg", @{name}); ).try_into().unwrap()
    }

    fn make_text(align: &str) -> Element {
        let node = document().create_element("div").unwrap();

        js! { @(no_return)
            var node = @{&node};
            node.style.position = "absolute";
            node.style.left = "0px";
            node.style.top = "0px";
            //node.style.width = "100%";
            //node.style.height = "100%";
            node.style.color = "white";
            node.style.textShadow = @{TEXT_SHADOW};
            node.style.backgroundColor = @{BACKGROUND_COLOR};
            node.style.padding = "5px";
            node.style.fontSize = "16px";
            node.style.textAlign = @{align};
            node.style.whiteSpace = "pre";
        }

        node
    }

    fn update_svg(svg_root: &Element, text_root: &Element, information: Vec<RecordInformation>, matches_len: Option<usize>) {
        let total_len = information.len();

        let starting_index = matches_len.map(|len| {
            if len > total_len {
                0
            } else {
                total_len - len
            }
        }).unwrap_or(0);

        let information = &information[starting_index..];
        let statistics = Statistics::new(&information);

        /*let information_f_f = Information::new(records, make_chart_mode(false, false, use_percentages), extra_data);
        let information_t_f = Information::new(records, make_chart_mode(true, false, use_percentages), extra_data);
        let information_f_t = Information::new(records, make_chart_mode(false, true, use_percentages), extra_data);
        let information_t_t = Information::new(records, make_chart_mode(true, true, use_percentages), extra_data);

        let statistics = information_f_f.statistics
            .merge(&information_t_f.statistics)
            .merge(&information_f_t.statistics)
            .merge(&information_t_t.statistics);

        let information = if expected_profit {
            if winrate {
                information_t_t
            } else {
                information_t_f
            }
        } else {
            if winrate {
                information_f_t
            } else {
                information_f_f
            }
        };*/

        let mut d_gains = vec![];
        let mut d_losses = vec![];
        let mut d_money = vec!["M0,100".to_owned()];
        let mut d_bets = vec![];
        let mut d_winner_profit = vec![];
        let mut d_tournaments = vec![];

        let mut money: f64 = 0.0;
        let mut starting_money: f64 = money;
        let mut final_money: f64 = money;

        let len = information.len() as f64;

        //let y = (statistics.max_gain / (statistics.max_gain + statistics.max_loss)) * 100.0;
        let y = (statistics.max_odds_gain / (statistics.max_odds_gain + statistics.max_odds_loss)) * 100.0;

        let mut first = true;

        for (index, record) in information.iter().enumerate() {
            let x = normalize(index as f64, 0.0, len) * 100.0;
            //let x = normalize(record.date(), statistics.lowest_date, statistics.highest_date) * 100.0;

            match record {
                RecordInformation::TournamentFinal { profit, old_sum, new_sum, .. } => {
                    if first {
                        starting_money = *old_sum;
                    }

                    d_tournaments.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(*profit, 0.0, statistics.max_gain), y, 0.0), x, y));
                    money = *new_sum;
                },
                RecordInformation::Match { odds, old_sum, new_sum, mode, .. } => {
                    if let Mode::Matchmaking = mode {
                        if first {
                            starting_money = *old_sum;
                        }

                        money = *new_sum;

                        match odds {
                            Ok(amount) => {
                                d_gains.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(*amount, 0.0, statistics.max_odds_gain), y, 0.0), x, y));

                                {
                                    let y = range_inclusive(normalize(1.0, 0.0, statistics.max_odds_gain), y, 0.0);
                                    d_bets.push(format!("M{},{}L{},{}", x, y, x, y + 0.3));
                                }
                            },

                            Err(amount) => {
                                d_losses.push(format!("M{},{}L{},{}", x, y, x, range_inclusive(normalize(1.0, 0.0, statistics.max_odds_loss), y, 100.0)));

                                {
                                    let y = range_inclusive(normalize(*amount, 0.0, statistics.max_odds_loss), y, 100.0);
                                    d_winner_profit.push(format!("M{},{}L{},{}", x, y - 0.3, x, y));
                                }
                            },
                        }

                        /*match *profit {
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
                        }*/
                    }
                },
            }

            final_money = money;

            if first {
                first = false;
                d_money.push(format!("M{},{}", x, normalize(money, statistics.max_money, statistics.min_money) * 100.0));

            } else {
                d_money.push(format!("L{},{}", x, normalize(money, statistics.max_money, statistics.min_money) * 100.0));
            }

            //simulation.insert_record(&record);
        }

        /*for record in records {
            node.append_child(&display_record(record, &information));
        }*/

        fn make_line(d: Vec<String>, color: &str) -> Element {
            let node = svg("path");

            let d: String = d.iter().flat_map(|x| x.chars()).collect();

            js! { @(no_return)
                var node = @{&node};
                node.setAttribute("stroke", @{color});
                node.setAttribute("stroke-width", "1px");
                node.setAttribute("stroke-opacity", "1");
                node.setAttribute("fill-opacity", "0");
                node.setAttribute("vector-effect", "non-scaling-stroke");
                node.setAttribute("d", @{d});
            }

            node
        }

        js! { @(no_return)
            var node = @{&svg_root};
            node.style.position = "absolute";
            node.style.top = "5px";
            node.style.left = "5px";
            node.style.width = "calc(100% - 10px)";
            node.style.height = "calc(100% - 10px)";
            node.setAttribute("xmlns", "http://www.w3.org/2000/svg");
            node.setAttribute("viewBox", "0 0 100 100");
            node.setAttribute("preserveAspectRatio", "none");
            node.innerHTML = "";
        }

        svg_root.append_child(&make_line(d_gains, "hsla(120, 75%, 50%, 1)"));
        // #6441a5
        svg_root.append_child(&make_line(d_bets, "hsla(0, 100%, 50%, 1)"));
        svg_root.append_child(&make_line(d_losses, "hsla(0, 75%, 50%, 1)"));
        svg_root.append_child(&make_line(d_winner_profit, "hsla(120, 75%, 50%, 1)"));
        svg_root.append_child(&make_line(d_tournaments, "hsl(240, 100%, 75%)"));
        svg_root.append_child(&make_line(d_money, "white"));

        let total_gains = final_money - starting_money;
        let average_gains = total_gains / len;
        let average_money = final_money / (total_len as f64);

        js! { @(no_return)
            @{text_root}.textContent = @{format!("Starting money: {}\nFinal money: {}\nAverage money: {}\nTotal gains: {}\nMaximum: {}\nMinimum: {}\nMatches: {}\nAverage gains: {}\nAverage odds: {}\nWinrate: {}%",
                salty_bet_bot::money(starting_money),
                salty_bet_bot::money(final_money),
                salty_bet_bot::money(average_money),
                salty_bet_bot::money(total_gains),
                salty_bet_bot::money(statistics.max_money),
                salty_bet_bot::money(statistics.min_money),
                decimal(total_len as f64),
                salty_bet_bot::money(average_gains),
                statistics.average_odds,
                (statistics.wins / (statistics.wins + statistics.losses)) * 100.0)};
        }
    }

    let svg_root = svg("svg");
    let text_root = make_text("left");

    let simulation_type = Rc::new(RefCell::new("real-data".to_string()));
    let show_only_recent_data = Rc::new(Cell::new(true));
    let extra_data = Rc::new(Cell::new(false));
    let use_percentages = Rc::new(Cell::new(true));
    let records = Rc::new(records);

    let update_svg = Rc::new({
        let svg_root = svg_root.clone();
        let text_root = text_root.clone();
        let simulation_type = simulation_type.clone();
        let show_only_recent_data = show_only_recent_data.clone();
        let extra_data = extra_data.clone();
        let records = records.clone();

        move || {
            fn make_earnings(expected_profit: bool, winrate: bool) -> ChartMode<EarningsStrategy> {
                ChartMode::SimulateMatchmaking(EarningsStrategy { expected_profit, winrate, bet_difference: false, winrate_difference: false, use_percentages: true })
            }

            fn make_information(records: &[Record], value: &str, extra_data: bool) -> Vec<RecordInformation> {
                match value {
                    "real-data" => {
                        let real: ChartMode<()> = ChartMode::RealData { days: None };
                        RecordInformation::calculate(records, real, extra_data)
                    },
                    "earnings" => RecordInformation::calculate(records, make_earnings(true, false), extra_data),
                    "hybrid" => RecordInformation::calculate(records, ChartMode::SimulateMatchmaking(HybridStrategy), extra_data),
                    "upset-percentage" => RecordInformation::calculate(records, ChartMode::SimulateMatchmaking(UpsetStrategy::Percentage), extra_data),
                    "upset-odds" => RecordInformation::calculate(records, ChartMode::SimulateMatchmaking(UpsetStrategy::Odds), extra_data),
                    "winrate-high" => RecordInformation::calculate(records, ChartMode::SimulateMatchmaking(WinrateStrategy::High), extra_data),
                    "winrate-low" => RecordInformation::calculate(records, ChartMode::SimulateMatchmaking(WinrateStrategy::Low), extra_data),
                    "random-left" => RecordInformation::calculate(records, ChartMode::SimulateMatchmaking(RandomStrategy::Left), extra_data),
                    "random-right" => RecordInformation::calculate(records, ChartMode::SimulateMatchmaking(RandomStrategy::Right), extra_data),
                    _ => panic!("Invalid value {}", value),
                }

                //ChartMode::RealData { days: Some(7), matches: None }
                //ChartMode::RealData { days: None }
                //ChartMode::SimulateMatchmaking(EarningsStrategy { expected_profit, winrate, bet_difference: false, winrate_difference: false, use_percentages })
                //ChartMode::SimulateMatchmaking(matchmaking_strategy())
            }

            let simulation_type = simulation_type.borrow();

            let recent_matches = if show_only_recent_data.get() {
                Some(1000)

            } else {
                None
            };

            update_svg(&svg_root, &text_root, make_information(&records, &simulation_type, extra_data.get()), recent_matches);
        }
    });

    update_svg();

    node.append_child(&svg_root);
    node.append_child(&text_root);

    node.append_child(&{
        let node: SelectElement = document().create_element("select").unwrap().try_into().unwrap();

        fn make_option(text: &str, value: &str, default: &str) -> Element {
            let node = document().create_element("option").unwrap();

            js! { @(no_return)
                var node = @{&node};
                node.text = @{text};
                node.value = @{value};
                node.selected = @{value == default};
            }

            node
        }

        js! { @(no_return)
            var node = @{&node};
            node.style.position = "absolute";
            node.style.right = "5px";
            node.style.top = "5px";
        }

        {
            let simulation_type = simulation_type.borrow();

            node.append_child(&make_option("Real data", "real-data", &simulation_type));
            node.append_child(&make_option("Hybrid", "hybrid", &simulation_type));
            node.append_child(&make_option("Earnings", "earnings", &simulation_type));
            node.append_child(&make_option("Upset (percentage)", "upset-percentage", &simulation_type));
            node.append_child(&make_option("Upset (odds)", "upset-odds", &simulation_type));
            node.append_child(&make_option("Winrate (high)", "winrate-high", &simulation_type));
            node.append_child(&make_option("Winrate (low)", "winrate-low", &simulation_type));
            node.append_child(&make_option("Random (left)", "random-left", &simulation_type));
            node.append_child(&make_option("Random (right)", "random-right", &simulation_type));
        }

        node.add_event_listener({
            let node = node.clone();
            let simulation_type = simulation_type.clone();
            let update_svg = update_svg.clone();
            move |_: ChangeEvent| {
                if let Some(value) = node.value() {
                    *simulation_type.borrow_mut() = value;
                    update_svg();
                }
            }
        });

        node
    });

    node.append_child(&make_checkbox("Show only recent data", "5px", "30px", show_only_recent_data.get(), {
        let show_only_recent_data = show_only_recent_data.clone();
        let update_svg = update_svg.clone();
        move |value| {
            show_only_recent_data.set(value);
            update_svg();
        }
    }));

    node.append_child(&make_checkbox("Simulate extra data", "5px", "55px", extra_data.get(), {
        let extra_data = extra_data.clone();
        let update_svg = update_svg.clone();
        move |value| {
            extra_data.set(value);
            update_svg();
        }
    }));

    /*node.append_child(&make_checkbox("Expected profit", "5px", "5px", expected_profit.get(), {
        let svg_root = svg_root.clone();
        let text_root = text_root.clone();
        let expected_profit = expected_profit.clone();
        let winrate = winrate.clone();
        let extra_data = extra_data.clone();
        let use_percentages = use_percentages.clone();
        let records = records.clone();
        move |value| {
            expected_profit.set(value);
            update_svg(&svg_root, &text_root, &records, expected_profit.get(), winrate.get(), extra_data.get(), use_percentages.get());
        }
    }));

    node.append_child(&make_checkbox("Winrate", "5px", "30px", winrate.get(), {
        let svg_root = svg_root.clone();
        let text_root = text_root.clone();
        let expected_profit = expected_profit.clone();
        let winrate = winrate.clone();
        let extra_data = extra_data.clone();
        let use_percentages = use_percentages.clone();
        let records = records.clone();
        move |value| {
            winrate.set(value);
            update_svg(&svg_root, &text_root, &records, expected_profit.get(), winrate.get(), extra_data.get(), use_percentages.get());
        }
    }));

    node.append_child(&make_checkbox("Use percentages", "5px", "80px", use_percentages.get(), {
        let svg_root = svg_root.clone();
        let text_root = text_root.clone();
        let expected_profit = expected_profit.clone();
        let winrate = winrate.clone();
        let extra_data = extra_data.clone();
        let use_percentages = use_percentages.clone();
        let records = records.clone();
        move |value| {
            use_percentages.set(value);
            update_svg(&svg_root, &text_root, &records, expected_profit.get(), winrate.get(), extra_data.get(), use_percentages.get());
        }
    }));*/

    /*node.append_child(&{
        let node = svg("text");

        js! { @(no_return)
            var node = @{&node};
            node.setAttribute("x", "1");
            node.setAttribute("y", "48");
            node.setAttribute("font-size", "2px");
            node.setAttribute("fill", "white");
            //node.setAttribute("text-anchor", "end");
            node.textContent = @{(information.max_money - information.min_money).abs() / 2.0};
        }

        node
    });

    node.append_child(&{
        let node = svg("text");

        js! { @(no_return)
            var node = @{&node};
            node.setAttribute("x", "1");
            node.setAttribute("y", "96");
            node.setAttribute("font-size", "2px");
            node.setAttribute("fill", "white");
            //node.setAttribute("text-anchor", "end");
            node.textContent = @{information.min_money};
        }

        node
    });*/
}


fn main() {
    stdweb::initialize();

    pub fn set_panic_hook() {
        std::panic::set_hook(Box::new(move |info| {
            stdweb::PromiseFuture::print_error_panic(info.to_string());
        }));
    }

    set_panic_hook();

    log!("Initializing...");

    records_get_all(move |records| {
        let node = document().create_element("div").unwrap();

        node.class_list().add("root").unwrap();

        display_records(&node, records);

        document().body().unwrap().append_child(&node);
    });

    stdweb::event_loop();
}
