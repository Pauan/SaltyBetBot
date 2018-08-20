#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;

use std::rc::Rc;
use std::cell::Cell;
use salty_bet_bot::{get_storage, subtract_days, matchmaking_strategy};
use algorithm::record::{Record, Profit, Mode};
use algorithm::simulation::{Bet, Simulation, Strategy, Simulator, SALT_MINE_AMOUNT};
use algorithm::strategy::{EarningsStrategy, AllInStrategy, PERCENTAGE_THRESHOLD};
use stdweb::traits::*;
use stdweb::web::{document, Element};
use stdweb::web::event::ChangeEvent;
use stdweb::unstable::TryInto;


#[allow(dead_code)]
enum ChartMode<A> {
    SimulateTournament(A),
    SimulateMatchmaking(A),
    RealData { days: Option<u32>, matches: Option<usize> },
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
    TournamentBet {
        date: f64,
        profit: Profit,
        bet: Bet,
    },
    TournamentFinal {
        date: f64,
        profit: f64,
    },
    Matchmaking {
        date: f64,
        profit: Profit,
        bet: Bet,
    },
}

impl RecordInformation {
    fn date(&self) -> f64 {
        match *self {
            RecordInformation::TournamentBet { date, .. } => date,
            RecordInformation::TournamentFinal { date, .. } => date,
            RecordInformation::Matchmaking { date, .. } => date,
        }
    }
}

#[derive(Debug)]
struct Statistics {
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
    fn merge(&self, other: &Self) -> Self {
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
    }
}

#[derive(Debug)]
struct Information {
    starting_money: f64,
    wins: f64,
    losses: f64,
    odds: f64,
    odds_gain: f64,
    odds_loss: f64,
    statistics: Statistics,
    records: Vec<RecordInformation>,
}

impl Information {
    fn new<A: Strategy>(input: &[Record], mode: ChartMode<A>, extra_data: bool) -> Self {
        let mut simulation = Simulation::new();

        simulation.sum = PERCENTAGE_THRESHOLD;

        let mut starting_money = simulation.sum;

        let mut max_gain: f64 = 0.0;
        let mut max_loss: f64 = 0.0;

        let mut max_bet: f64 = 0.0;
        let mut min_bet: f64 = 0.0;

        let mut max_money: f64 = if let ChartMode::SimulateTournament(_) = mode { simulation.tournament_sum } else { simulation.sum };
        let mut min_money: f64 = max_money;

        let mut lowest_date: f64 = 0.0;
        let mut highest_date: f64 = 0.0;

        let mut len: f64 = 0.0;
        let mut odds_gain: f64 = 0.0;
        let mut odds_loss: f64 = 0.0;
        let mut odds: f64 = 0.0;

        let mut records: Vec<RecordInformation> = vec![];

        match mode {
            ChartMode::SimulateTournament(strategy) => {
                simulation.tournament_strategy = Some(strategy);

                let mut index: f64 = 0.0;
                let mut sum: f64 = 0.0;

                for record in input {
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
                        let date = index;
                        highest_date = date;
                        index += 1.0;

                        if let Some(amount) = bet.amount() {
                            max_bet = max_bet.max(amount);
                            min_bet = if min_bet == 0.0 { amount } else { min_bet.min(amount) };

                            let profit = Profit::from_old_new(old_sum, new_sum);

                            records.push(RecordInformation::TournamentBet {
                                date,
                                bet,
                                profit,
                            });
                        }
                    }

                    if let Some(tournament_profit) = tournament_profit {
                        let date = index;
                        highest_date = date;
                        index += 1.0;

                        sum += tournament_profit;

                        records.push(RecordInformation::TournamentFinal {
                            date,
                            profit: tournament_profit,
                        });

                        max_money = max_money.max(sum);
                        max_gain = max_gain.max(tournament_profit);
                    }

                    simulation.insert_record(&record);
                }
            },

            ChartMode::SimulateMatchmaking(strategy) => {
                simulation.matchmaking_strategy = Some(strategy);

                let mut index: f64 = 0.0;

                if extra_data {
                    for record in input.iter() {
                        simulation.insert_record(&record);
                    }
                }

                for record in input {
                    if simulation.min_matches_len(&record.left.name, &record.right.name) >= 10.0 &&
                       record.mode == Mode::Matchmaking {

                        let date = index;
                        highest_date = date;
                        index += 1.0;

                        let old_sum = simulation.sum;

                        let tournament_profit = simulation.tournament_profit(&record);

                        let bet = simulation.bet(&record);

                        simulation.calculate(&record, &bet);

                        simulation.sum -= tournament_profit.unwrap_or(0.0);

                        let new_sum = simulation.sum;

                        let profit = Profit::from_old_new(old_sum, new_sum);

                        match profit {
                            Profit::Gain(gain) => {
                                max_money = max_money.max(new_sum);
                                max_gain = max_gain.max(gain);
                            },
                            Profit::Loss(loss) => {
                                min_money = min_money.min(new_sum);
                                max_loss = max_loss.max(loss);
                            },
                            Profit::None => {},
                        }

                        if let Some(amount) = bet.amount() {
                            max_bet = max_bet.max(amount);
                            min_bet = if min_bet == 0.0 { amount } else { min_bet.min(amount) };
                        }

                        if let Some(x) = record.odds(&bet) {
                            len += 1.0;
                            odds += x;

                            if x < 0.0 {
                                odds_loss += x;
                            } else {
                                odds_gain += x;
                            }

                            records.push(RecordInformation::Matchmaking {
                                date,
                                bet,
                                profit,
                            });
                        }
                    }

                    if !extra_data {
                        simulation.insert_record(&record);
                    }
                }
            },

            ChartMode::RealData { days, matches } => {
                simulation.sum = SALT_MINE_AMOUNT;
                starting_money = simulation.sum;
                max_money = starting_money;
                min_money = starting_money;

                // TODO
                let days: Option<f64> = days.map(|days| subtract_days(days));

                let mut first = true;

                //let input: Vec<&Record> = input.into_iter().filter(|record| record.mode == Mode::Matchmaking).collect();

                let input_len = input.len();

                for (index, record) in input.iter().enumerate() {
                    let date = record.date;

                    if days.map(|days| date >= days).unwrap_or(true) &&
                       matches.map(|matches| index >= (input_len - matches)).unwrap_or(true) {
                        if first {
                            first = false;
                            starting_money = simulation.sum;
                            max_money = starting_money;
                            min_money = starting_money;
                        }

                        lowest_date = if lowest_date == 0.0 { date } else { lowest_date.min(date) };
                        highest_date = highest_date.max(date);

                        let old_sum = simulation.sum;

                        let tournament_profit = simulation.tournament_profit(&record);

                        if let Some(tournament_profit) = tournament_profit {
                            let new_sum = old_sum + tournament_profit;

                            records.push(RecordInformation::TournamentFinal {
                                date,
                                profit: tournament_profit,
                            });

                            max_money = max_money.max(new_sum);
                            max_gain = max_gain.max(tournament_profit);
                        }

                        let old_sum = old_sum + tournament_profit.unwrap_or(0.0);

                        let bet = record.bet.clone();

                        simulation.calculate(&record, &bet);

                        let new_sum = simulation.sum;

                        max_money = max_money.max(new_sum);
                        min_money = min_money.min(new_sum);

                        let profit = Profit::from_old_new(old_sum, new_sum);

                        match profit {
                            Profit::Gain(gain) => {
                                max_gain = max_gain.max(gain);
                            },
                            Profit::Loss(loss) => {
                                max_loss = max_loss.max(loss);
                            },
                            Profit::None => {},
                        }

                        if let Mode::Matchmaking = record.mode {
                            if let Some(amount) = bet.amount() {
                                max_bet = max_bet.max(amount);
                                min_bet = if min_bet == 0.0 { amount } else { min_bet.min(amount) };
                            }

                            if let Some(x) = record.odds(&bet) {
                                len += 1.0;
                                odds += x;

                                if x < 0.0 {
                                    odds_loss += x;
                                } else {
                                    odds_gain += x;
                                }

                                records.push(RecordInformation::Matchmaking {
                                    date,
                                    bet,
                                    profit,
                                });
                            }
                        }

                    } else {
                        simulation.calculate(&record, &record.bet);
                    }
                }
            },
        }

        let wins = simulation.successes;
        let losses = simulation.failures;

        records.sort_by(|a, b| a.date().partial_cmp(&b.date()).unwrap());

        Self {
            starting_money, wins, losses, odds: odds / len, odds_gain: odds_gain / len, odds_loss: odds_loss / len,
            statistics: Statistics { max_gain, max_loss, max_bet, min_bet, max_money, min_money, lowest_date, highest_date },
            records,
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


fn make_checkbox<F>(name: &str, x: &str, y: &str, value: bool, mut callback: F) -> Element where F: FnMut(bool) + 'static {
    let node = document().create_element("label").unwrap();

    js! { @(no_return)
        var node = @{&node};
        node.style.position = "absolute";
        node.style.right = @{x};
        node.style.top = @{y};
        node.style.color = "white";
    }

    node.append_child(&{
        let node = document().create_element("input").unwrap();

        js! { @(no_return)
            var node = @{&node};
            node.type = "checkbox";
            node.checked = @{value};
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

    fn make_text(x: &str, y: &str, align: &str) -> Element {
        let node = document().create_element("div").unwrap();

        js! { @(no_return)
            var node = @{&node};
            node.style.position = "absolute";
            node.style.left = @{x};
            node.style.top = @{y};
            //node.style.width = "100%";
            //node.style.height = "100%";
            node.style.color = "white";
            node.style.fontSize = "16px";
            node.style.textAlign = @{align};
            node.style.whiteSpace = "pre";
        }

        node
    }

    fn update_svg(svg_root: &Element, text_root: &Element, records: &[Record], expected_profit: bool, winrate: bool, extra_data: bool, use_percentages: bool) {
        fn make_chart_mode(expected_profit: bool, winrate: bool, use_percentages: bool) -> ChartMode<()> {
            ChartMode::RealData { days: None, matches: Some(1000) }
            //ChartMode::RealData { days: Some(7), matches: None }
            //ChartMode::RealData { days: None }
            //ChartMode::SimulateMatchmaking(EarningsStrategy { expected_profit, winrate, bet_difference: false, winrate_difference: false, use_percentages })
            //ChartMode::SimulateMatchmaking(matchmaking_strategy())
        }

        let information = Information::new(records, make_chart_mode(expected_profit, winrate, use_percentages), extra_data);

        let statistics = &information.statistics;

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
        let mut d_tournaments = vec![];

        let mut money: f64 = information.starting_money;
        let mut final_money: f64 = money;

        let total = statistics.max_gain + statistics.max_loss;
        let len = information.records.len() as f64;

        let y = (statistics.max_gain / total) * 100.0;

        let mut first = true;

        for record in information.records.iter() {
            let x = normalize(record.date(), statistics.lowest_date, statistics.highest_date) * 100.0;

            match record {
                RecordInformation::TournamentBet { profit, bet, .. } => {
                    /*match *profit {
                        Profit::Gain(amount) => {
                            d_gains.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(amount, 0.0, information.max_gain), y, 0.0), x, y));

                            if let Some(amount) = bet.amount() {
                                let y = range_inclusive(normalize(amount, 0.0, information.max_gain), y, 0.0);
                                d_bets.push(format!("M{},{}L{},{}", x, y, x, y + 0.3));
                                //format!("M{},100L{},{}", x, x, normalize(amount, information.max_bet, information.min_bet) * 100.0)
                            }
                        },
                        Profit::Loss(amount) => {
                            d_losses.push(format!("M{},{}L{},{}", x, y, x, range_inclusive(normalize(amount, 0.0, information.max_loss), y, 100.0)));
                        },
                        Profit::None => {},
                    }*/
                },
                RecordInformation::TournamentFinal { profit, .. } => {
                    d_tournaments.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(*profit, 0.0, statistics.max_gain), y, 0.0), x, y));
                    money += profit;
                },
                RecordInformation::Matchmaking { profit, bet, .. } => {
                    match *profit {
                        Profit::Gain(amount) => {
                            d_gains.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(amount, 0.0, statistics.max_gain), y, 0.0), x, y));

                            if let Some(amount) = bet.amount() {
                                let y = range_inclusive(normalize(amount, 0.0, statistics.max_gain), y, 0.0);
                                d_bets.push(format!("M{},{}L{},{}", x, y, x, y + 0.3));
                                //format!("M{},100L{},{}", x, x, normalize(amount, information.max_bet, information.min_bet) * 100.0)
                            }

                            money += amount;
                        },
                        Profit::Loss(amount) => {
                            d_losses.push(format!("M{},{}L{},{}", x, y, x, range_inclusive(normalize(amount, 0.0, statistics.max_loss), y, 100.0)));
                            money -= amount;
                        },
                        Profit::None => {},
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
        svg_root.append_child(&make_line(d_tournaments, "hsl(240, 100%, 75%)"));
        svg_root.append_child(&make_line(d_money, "white"));

        let total_gains = final_money - information.starting_money;
        let average_gains = total_gains / len;

        js! { @(no_return)
            @{text_root}.textContent = @{format!("Starting money: {}\nFinal money: {}\nTotal gains: {}\nMaximum: {}\nMinimum: {}\nMatches: {}\nAverage gains: {}\nAverage odds: {}\nAverage odds (win): {}\nAverage odds (loss): {}\nWinrate: {}%", information.starting_money, final_money, total_gains, information.statistics.max_money, information.statistics.min_money, len, average_gains, information.odds, information.odds_gain, information.odds_loss, (information.wins / (information.wins + information.losses)) * 100.0)};
        }
    }

    let svg_root = svg("svg");
    let text_root = make_text("3px", "2px", "left");

    let expected_profit = Rc::new(Cell::new(true));
    let winrate = Rc::new(Cell::new(false));
    let extra_data = Rc::new(Cell::new(false));
    let use_percentages = Rc::new(Cell::new(true));
    let records = Rc::new(records);

    update_svg(&svg_root, &text_root, &records, expected_profit.get(), winrate.get(), extra_data.get(), use_percentages.get());

    node.append_child(&svg_root);
    node.append_child(&text_root);

    node.append_child(&make_checkbox("Expected profit", "5px", "5px", expected_profit.get(), {
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

    node.append_child(&make_checkbox("Extra data", "5px", "55px", extra_data.get(), {
        let svg_root = svg_root.clone();
        let text_root = text_root.clone();
        let expected_profit = expected_profit.clone();
        let winrate = winrate.clone();
        let extra_data = extra_data.clone();
        let use_percentages = use_percentages.clone();
        let records = records.clone();
        move |value| {
            extra_data.set(value);
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
    }));

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

    log!("Initializing...");

    get_storage("matches", move |matches| {
        let matches: Vec<Record> = match matches {
            Some(a) => serde_json::from_str(&a).unwrap(),
            None => vec![],
        };

        let node = document().create_element("div").unwrap();

        node.class_list().add("root").unwrap();

        display_records(&node, matches);

        document().body().unwrap().append_child(&node);
    });

    stdweb::event_loop();
}
