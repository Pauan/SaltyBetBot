#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;
extern crate serde_json;
#[macro_use]
extern crate salty_bet_bot;
extern crate algorithm;

use std::rc::Rc;
use salty_bet_bot::{records_get_all, percentage, decimal, money, Loading};
use algorithm::simulation::{Simulation, Simulator, Bet};
use algorithm::record::{Record, Character, Winner, Tier, Mode, Profit};
use stdweb::unstable::TryInto;
use stdweb::traits::*;
use stdweb::web::{document, Element};
use stdweb::web::event::ClickEvent;


fn display_records(top: &Element, records: Rc<Vec<(usize, usize, Record)>>, starting_index: usize, ending_index: usize) {
    log!("Displaying indexes {} - {}", starting_index, ending_index);

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
    fn display_character(character: &Character, class: &str, matches_len: usize, bet_amount: f64) -> Element {
        td("character", &[
            field("Name: ", &span(class, &character.name)),
            field("Bet amount: ", &span("money", &money(character.bet_amount + bet_amount))),
            field("Number of matches: ", &text(&matches_len.to_string())),
            //field("Win streak: ", &character.win_streak.to_string()),
            //field("Illuminati bettors: ", &character.illuminati_bettors.to_string()),
            //field("Normal bettors: ", &character.normal_bettors.to_string()),
        ])
    }

    top.append_child(&{
        let node = document().create_element("table").unwrap();

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

        /*{
            let len = records.len() as f64;

            let mut bet_left: f64 = 0.0;
            let mut bet_right: f64 = 0.0;

            let mut odds_left: f64 = 0.0;
            let mut odds_right: f64 = 0.0;

            for record in records.iter() {
                bet_left += record.left.bet_amount;
                bet_right += record.right.bet_amount;

                odds_left += record.odds_left(0.0);
                odds_right += record.odds_right(0.0);
            }

            log!("Left bets: {}\nRight bets: {}\nLeft odds: {}\nRight odds: {}", bet_left / len, bet_right / len, odds_left / len, odds_right / len);
        }*/

        let start_index = records.len().saturating_sub(ending_index);
        let end_index = records.len().saturating_sub(starting_index);

        for (left_matches_len, right_matches_len, record) in records[start_index..end_index].iter().rev() {
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

                row.append_child(&display_character(&record.left, "left", *left_matches_len, if let Bet::Left(amount) = record.bet { amount } else { 0.0 }));
                row.append_child(&display_character(&record.right, "right", *right_matches_len, if let Bet::Right(amount) = record.bet { amount } else { 0.0 }));

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
                        span("money", &money(sum))

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

        node
    });

    fn button<F>(class: &str, name: &str, mut f: F) -> Element where F: FnMut() + 'static {
        let node = document().create_element("button").unwrap();

        node.class_list().add(class).unwrap();
        node.set_text_content(name);

        // TODO cleanup
        node.add_event_listener(move |_: ClickEvent| {
            f();
        });

        node
    }

    fn page_section(name: &str) -> Element {
        let node = document().create_element("div").unwrap();

        js! { @(no_return)
            @{&node}.id = @{name};
        }

        node.class_list().add("page-section").unwrap();

        node
    }

    fn page_divider() -> Element {
        let node = document().create_element("div").unwrap();
        node.class_list().add("page-divider").unwrap();
        node.set_text_content("...");
        node
    }

    top.append_child(&{
        let node = document().create_element("div").unwrap();

        node.class_list().add("paginator").unwrap();

        {
            if starting_index < 100 {
                node.append_child(&button("page-arrow-disabled", "<", move || {}));

            } else {
                let top = top.clone();
                let records = records.clone();

                node.append_child(&button("page-arrow", "<", move || {
                    let starting_index = starting_index.saturating_sub(100);
                    let ending_index = ending_index.saturating_sub(100);

                    move_to_page(&top, records.clone(), starting_index, ending_index);
                }));
            }
        }

        let starting_cutoff = 500.min(records.len());
        let ending_cutoff = records.len().saturating_sub(500);

        let middle_starting_cutoff = starting_index.saturating_sub(700).min(ending_cutoff.saturating_sub(1500)).max(starting_cutoff);
        let middle_ending_cutoff = middle_starting_cutoff.saturating_add(1500);

        let starting_pages = page_section("left-page-section");
        let middle_pages = page_section("middle-page-section");
        let ending_pages = page_section("right-page-section");

        for (index, new_starting_index) in (0..records.len()).step_by(100).enumerate() {
            let node = if new_starting_index < starting_cutoff {
                Some(&starting_pages)

            // TODO is this correct ?
            } else if new_starting_index >= ending_cutoff {
                Some(&ending_pages)

            } else if new_starting_index >= middle_starting_cutoff &&
                      new_starting_index < middle_ending_cutoff {
                Some(&middle_pages)

            } else {
                None
            };

            if let Some(node) = node {
                let new_ending_index = new_starting_index.saturating_add(100);

                let top = top.clone();
                let records = records.clone();

                let class = if new_starting_index >= starting_index && new_ending_index <= ending_index {
                    "page-button-selected"
                } else {
                    "page-button"
                };

                node.append_child(&button(class, &(index.saturating_add(1)).to_string(), move || {
                    move_to_page(&top, records.clone(), new_starting_index, new_ending_index);
                }));
            }
        }

        node.append_child(&starting_pages);
        node.append_child(&page_divider());
        node.append_child(&middle_pages);
        node.append_child(&page_divider());
        node.append_child(&ending_pages);

        {
            let len = records.len().saturating_sub(100);

            if starting_index > len {
                node.append_child(&button("page-arrow-disabled", ">", move || {}));

            } else {
                let top = top.clone();
                let records = records.clone();

                node.append_child(&button("page-arrow", ">", move || {
                    let starting_index = starting_index.saturating_add(100);
                    let ending_index = ending_index.saturating_add(100);

                    move_to_page(&top, records.clone(), starting_index, ending_index);
                }));
            }
        }

        node
    });
}

fn move_to_page(top: &Element, records: Rc<Vec<(usize, usize, Record)>>, starting_index: usize, ending_index: usize) {
    js! { @(no_return)
        @{&top}.innerHTML = "";
    }

    display_records(top, records, starting_index, ending_index);
}


fn main() {
    stdweb::initialize();

    log!("Initializing...");

    let loading = Loading::new();

    document().body().unwrap().append_child(loading.element());

    records_get_all(move |records| {
        let node = document().create_element("div").unwrap();

        node.class_list().add("root").unwrap();

        {
            let mut simulation: Simulation<(), ()> = Simulation::new();

            //simulation.sum = SALT_MINE_AMOUNT;

            //let cutoff = subtract_days(SHOW_DAYS);

            let records: Vec<(usize, usize, Record)> = records.into_iter()
                .map(|record| {
                    let left_len = simulation.matches_len(&record.left.name);
                    let right_len = simulation.matches_len(&record.right.name);
                    simulation.insert_record(&record);
                    (left_len, right_len, record)
                    /*simulation.calculate(&record, &record.bet);

                    if let Mode::Tournament = record.mode {
                        (simulation.tournament_sum, record)

                    } else {
                        (simulation.sum, record)
                    }*/
                })
                .collect();

            display_records(&node, Rc::new(records), 0, 100);
        }

        document().body().unwrap().append_child(&node);

        loading.hide();
    });

    stdweb::event_loop();
}
