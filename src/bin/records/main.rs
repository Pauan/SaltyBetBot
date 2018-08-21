#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;

use salty_bet_bot::{get_storage, subtract_days};
use algorithm::simulation::{Simulation, SALT_MINE_AMOUNT, Bet};
use algorithm::record::{Record, Character, Winner, Tier, Mode, Profit};
use stdweb::unstable::TryInto;
use stdweb::traits::*;
use stdweb::web::{document, Element};


const SHOW_DAYS: u32 = 7;
const SHOW_MATCHES: usize = 1000;


fn display_records(node: &Element, records: Vec<Record>) {
    fn percentage(p: f64) -> String {
        // Rounds to 2 digits
        // https://stackoverflow.com/a/28656825/449477
        format!("{:.2}%", p * 100.0)
    }

    fn format_float(f: f64) -> String {
        js!(
            return @{f}.toLocaleString("en-US", {
                style: "currency",
                currency: "USD",
                minimumFractionDigits: 0
            });
        ).try_into().unwrap()
    }

    fn decimal(f: f64) -> String {
        js!(
            return @{f}.toLocaleString("en-US", {
                style: "decimal",
                maximumFractionDigits: 2
            });
        ).try_into().unwrap()
    }

    fn money(m: f64) -> String {
        if m < 0.0 {
            format!("-{}", format_float(-m))

        } else {
            format_float(m)
        }
    }

    fn date(d: f64) -> String {
        js!( return new Date(@{d}).toISOString(); ).try_into().unwrap()
    }

    fn th(name: &str) -> Element {
        let th = document().create_element("th").unwrap();
        th.set_text_content(name);
        th
    }

    fn td(class: &str, children: &[Element]) -> Element {
        let td = document().create_element("td").unwrap();

        td.class_list().add(class).unwrap();

        for x in children {
            td.append_child(x);
        }

        td
    }

    fn div(children: &[Element]) -> Element {
        let node = document().create_element("div").unwrap();

        for x in children {
            node.append_child(x);
        }

        node
    }

    fn span(class: &str, text: &str) -> Element {
        let node = document().create_element("span").unwrap();
        node.class_list().add(class).unwrap();
        node.set_text_content(&text);
        node
    }

    fn text(text: &str) -> Element {
        let node = document().create_element("span").unwrap();
        node.set_text_content(&text);
        node
    }

    fn field(name: &str, child: &Element) -> Element {
        let node = document().create_element("div").unwrap();

        node.append_child(&{
            let node = document().create_element("span").unwrap();
            node.class_list().add("field").unwrap();
            node.set_text_content(name);
            node
        });

        node.append_child(child);

        node
    }

    fn display_profit(profit: &Profit) -> Element {
        match profit {
            Profit::Gain(a) => span("gain", &money(*a)),
            Profit::Loss(a) => span("loss", &money(-a)),
            Profit::None => text(""),
        }
    }

    // TODO calculate the illuminati and normal bettors correctly (adding 1 depending on whether it bet or not)
    fn display_character(character: &Character, class: &str, bet_amount: f64) -> Element {
        td("character", &[
            field("Name: ", &span(class, &character.name)),
            field("Bet amount: ", &span("money", &money(character.bet_amount + bet_amount))),
            //field("Win streak: ", &character.win_streak.to_string()),
            //field("Illuminati bettors: ", &character.illuminati_bettors.to_string()),
            //field("Normal bettors: ", &character.normal_bettors.to_string()),
        ])
    }

    node.append_child(&{
        let row = document().create_element("tr").unwrap();

        row.append_child(&th("Mode"));
        row.append_child(&th("Tier"));
        row.append_child(&th("Left character"));
        row.append_child(&th("Right character"));
        row.append_child(&th("Winner"));
        row.append_child(&th("Bet"));
        row.append_child(&th("Odds"));
        row.append_child(&th("Winner Profit"));
        row.append_child(&th("Profit"));
        row.append_child(&th("Profit %"));
        row.append_child(&th("Sum"));
        row.append_child(&th("Duration (seconds)"));
        row.append_child(&th("Date (ISO)"));

        row
    });

    /*let mut simulation: Simulation<(), ()> = Simulation::new();

    simulation.sum = SALT_MINE_AMOUNT;

    //let cutoff = subtract_days(SHOW_DAYS);

    let iterator: Vec<(f64, Record)> = records.into_iter()
        .map(|record| {
            simulation.calculate(&record, &record.bet);

            if let Mode::Tournament = record.mode {
                (simulation.tournament_sum, record)

            } else {
                (simulation.sum, record)
            }
        })
        .collect();*/

    for record in records.into_iter().rev().take(SHOW_MATCHES) {
        let profit = record.profit(&record.bet);
        let bet_amount = record.bet.amount();

        let sum = if record.sum == -1.0 {
            None

        } else {
            Some(match profit {
                Profit::Gain(a) => record.sum + a,
                Profit::Loss(a) => record.sum - a,
                Profit::None => record.sum,
            })
        };

        node.append_child(&{
            let row = document().create_element("tr").unwrap();

            row.append_child(&td("mode", &[
                text(match record.mode {
                    Mode::Matchmaking => "Matchmaking",
                    Mode::Tournament => "Tournament",
                })
            ]));

            row.append_child(&td("tier", &[
                text(match record.tier {
                    Tier::New => "NEW",
                    Tier::P => "P",
                    Tier::B => "B",
                    Tier::A => "A",
                    Tier::S => "S",
                    Tier::X => "X",
                })
            ]));

            row.append_child(&display_character(&record.left, "left", if let Bet::Left(amount) = record.bet { amount } else { 0.0 }));
            row.append_child(&display_character(&record.right, "right", if let Bet::Right(amount) = record.bet { amount } else { 0.0 }));

            row.append_child(&td("winner", &[
                match record.winner {
                    Winner::Left => span("left", "Left"),
                    Winner::Right => span("right", "Right"),
                }
            ]));

            row.append_child(&td("bet", &[
                match record.bet {
                    Bet::Left(a) => div(&[
                        span("left", "Left "),
                        span("money", &money(a)),
                    ]),
                    Bet::Right(a) => div(&[
                        span("right", "Right "),
                        span("money", &money(a)),
                    ]),
                    Bet::None => text(""),
                }
            ]));

            let (left, right) = record.display_odds();

            row.append_child(&td("odds", &[
                span("left", &decimal(left)),
                span("odds-separator", " : "),
                span("right", &decimal(right)),
            ]));

            let winner_bet = match bet_amount {
                Some(amount) => match record.winner {
                    Winner::Left => Bet::Left(amount),
                    Winner::Right => Bet::Right(amount),
                },
                None => Bet::None,
            };

            let winner_profit = record.profit(&winner_bet);

            row.append_child(&td("winner-profit", &[
                display_profit(&winner_profit)
            ]));

            row.append_child(&td("profit", &[
                display_profit(&profit)
            ]));

            row.append_child(&td("profit-percentage", &[
                match bet_amount {
                    Some(a) => match profit {
                        Profit::Gain(b) => span("gain", &percentage(b / a)),
                        Profit::Loss(b) => span("loss", &percentage(-(b / a))),
                        Profit::None => text(""),
                    },
                    None => text(""),
                }
            ]));

            row.append_child(&td("profit-sum", &[
                if let Some(sum) = sum {
                    if sum < 0.0 {
                        span("loss", &money(sum))

                    } else {
                        span("gain", &money(sum))
                    }

                } else {
                    text("")
                }
            ]));

            row.append_child(&td("duration", &[
                text(&(record.duration / 1000.0).floor().to_string())
            ]));

            row.append_child(&td("date", &[
                text(&date(record.date))
            ]));

            row
        });
    }
}


fn main() {
    stdweb::initialize();

    log!("Initializing...");

    get_storage("matches", move |matches| {
        let matches: Vec<Record> = match matches {
            Some(a) => serde_json::from_str(&a).unwrap(),
            None => vec![],
        };

        let node = document().create_element("table").unwrap();

        node.class_list().add("root").unwrap();

        display_records(&node, matches);

        document().body().unwrap().append_child(&node);
    });

    stdweb::event_loop();
}
