#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;

use std::cmp::{PartialOrd, Ordering};
use salty_bet_bot::{wait_until_defined, parse_f64, parse_money, Port, create_tab, get_text_content, WaifuMessage, WaifuBetsOpen, WaifuBetsClosed, to_input_element, get_value, click, get_storage, set_storage, query, query_all};
use algorithm::record::{Record, Profit, Character, Winner, Mode, Tier};
use algorithm::simulation::{Bet, Simulation, Strategy, SALT_MINE_AMOUNT, TOURNAMENT_BALANCE};
use algorithm::strategy::{EarningsStrategy, AllInStrategy};
use stdweb::web::{document, INode, Element, IElement};
use stdweb::unstable::TryInto;


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
    Tournament {
        profit: f64,
        money: f64,
    },
    Matchmaking {
        profit: Profit,
        bet: Bet,
        money: f64,
    },
}

#[derive(Debug)]
struct Information {
    max_gain: f64,
    max_loss: f64,

    max_bet: f64,
    min_bet: f64,

    max_money: f64,
    min_money: f64,

    records: Vec<RecordInformation>,
}


fn collect_information(input: Vec<Record>) -> Information {
    let mut simulation = Simulation::new();

    simulation.matchmaking_strategy = Some(EarningsStrategy);
    simulation.tournament_strategy = Some(AllInStrategy);

    let mut max_gain: f64 = 0.0;
    let mut max_loss: f64 = 0.0;

    let mut max_bet: f64 = 0.0;
    let mut min_bet: f64 = 0.0;

    let mut max_money: f64 = simulation.sum;
    let mut min_money: f64 = simulation.sum;

    // TODO
    let date: f64 = js!(
        var date = new Date();
        date.setUTCDate(date.getUTCDate() - 1);
        return date.getTime();
    ).try_into().unwrap();

    let mut records: Vec<RecordInformation> = vec![];

    for record in input {
        let exiting_tournament = simulation.is_exiting_tournament(&record.mode);

        let old_sum = simulation.sum;

        let bet = simulation.bet(&record);

        simulation.calculate(&record, &bet);
        simulation.insert_record(&record);

        let new_sum = simulation.sum;

        let diff = (old_sum - new_sum).abs();

        let profit = match old_sum.partial_cmp(&new_sum).unwrap() {
            // Gain
            Ordering::Less => {
                max_money = max_money.max(new_sum);
                max_gain = max_gain.max(diff);
                Profit::Gain(diff)
            },
            // Loss
            Ordering::Greater => {
                min_money = min_money.min(new_sum);
                max_loss = max_loss.max(diff);
                Profit::Loss(diff)
            },
            Ordering::Equal => Profit::None,
        };

        if let Mode::Matchmaking = record.mode {
            if let Some(amount) = bet.amount() {
                max_bet = max_bet.max(amount);
                min_bet = if min_bet == 0.0 { amount } else { min_bet.min(amount) };
            }
        }

        if record.date >= date || true {
            if exiting_tournament {
                records.push(RecordInformation::Tournament {
                    profit: diff,
                    money: new_sum
                });
            }

            if let Mode::Matchmaking = record.mode {
                records.push(RecordInformation::Matchmaking { bet, profit, money: new_sum });
            }
        }
    }

    Information { max_gain, max_loss, max_bet, min_bet, max_money, min_money, records }
}


fn display_record(record: &Record, information: &Information) -> Element {
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
}


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


fn display_records(records: Vec<Record>) -> Element {
    let node = document().create_element("div").unwrap();

    node.class_list().add("root").unwrap();

    let information = collect_information(records);

    let mut d_gains = vec![];
    let mut d_losses = vec![];
    let mut d_money = vec!["M0,100".to_owned()];
    let mut d_bets = vec![];
    let mut d_tournaments = vec![];

    let total = information.max_gain + information.max_loss;
    let len = information.records.len() as f64;

    let y = (information.max_gain / total) * 100.0;

    for (index, record) in information.records.iter().enumerate() {
        let x = normalize(index as f64, 0.0, len) * 100.0;

        let money = match record {
            RecordInformation::Tournament { profit, money } => {
                d_tournaments.push(format!("M{},{}L{},{}", x, range_inclusive(normalize(*profit, 0.0, information.max_gain), y, 0.0), x, y));
                *money
            },
            RecordInformation::Matchmaking { profit, bet, money } => {
                match *profit {
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
                }

                *money
            },
        };

        d_money.push(format!("L{},{}", x, normalize(money, information.max_money, information.min_money) * 100.0));

        //simulation.insert_record(&record);
    }

    /*for record in records {
        node.append_child(&display_record(record, &information));
    }*/

    fn svg(name: &str) -> Element {
        js!( return document.createElementNS("http://www.w3.org/2000/svg", @{name}); ).try_into().unwrap()
    }

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

    node.append_child(&{
        let node = svg("svg");

        js! { @(no_return)
            var node = @{&node};
            node.style.position = "absolute";
            node.style.top = "0px";
            node.style.left = "0px";
            node.style.width = "100%";
            node.style.height = "100%";
            node.setAttribute("xmlns", "http://www.w3.org/2000/svg");
            node.setAttribute("viewBox", "0 0 100 100");
            node.setAttribute("preserveAspectRatio", "none");
        }

        node.append_child(&make_line(d_gains, "hsla(120, 75%, 50%, 1)"));
        // #6441a5
        node.append_child(&make_line(d_bets, "hsla(0, 100%, 50%, 1)"));
        node.append_child(&make_line(d_losses, "hsla(0, 75%, 50%, 1)"));
        node.append_child(&make_line(d_tournaments, "hsl(240, 100%, 75%)"));
        node.append_child(&make_line(d_money, "white"));

        node.append_child(&{
            let node = svg("text");

            js! { @(no_return)
                var node = @{&node};
                node.setAttribute("x", "1");
                node.setAttribute("y", "4");
                node.setAttribute("font-size", "2px");
                node.setAttribute("fill", "white");
                //node.setAttribute("text-anchor", "end");
                node.textContent = @{information.max_money};
            }

            node
        });

        node.append_child(&{
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
        });

        node
    });

    node
}


fn main() {
    stdweb::initialize();

    log!("Initializing...");

    get_storage("matches", move |matches| {
        let matches: Vec<Record> = match matches {
            Some(a) => serde_json::from_str(&a).unwrap(),
            None => vec![],
        };

        document().body().unwrap().append_child(&display_records(matches));
    });

    stdweb::event_loop();
}
