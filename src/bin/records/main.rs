#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate salty_bet_bot;
#[macro_use]
extern crate dominator;
#[macro_use]
extern crate lazy_static;

use std::rc::Rc;
use std::cmp::Ordering;
use salty_bet_bot::{records_get_all, percentage, decimal, money, display_odds, Loading, set_panic_hook};
use algorithm::simulation::{Simulation, Simulator, Strategy, Bet, Elo};
use algorithm::strategy::{MATCHMAKING_STRATEGY, TOURNAMENT_STRATEGY, CustomStrategy, winrates, average_odds, needed_odds, expected_profits};
use algorithm::record::{Record, Winner, Tier, Mode, Profit};
use stdweb::{spawn_local, unwrap_future};
use stdweb::unstable::TryInto;
use stdweb::traits::*;
use stdweb::web::document;
use stdweb::web::error::Error;
use stdweb::web::event::ClickEvent;
use futures_signals::signal::{Mutable, SignalExt};
use dominator::Dom;


const PAGE_LIMIT: usize = 100;

fn get_ending_index(starting_index: usize) -> usize {
    starting_index.saturating_add(PAGE_LIMIT)
}


struct Information {
    name: String,
    matches_len: usize,
    specific_matches_len: usize,
    bet_amount: f64,
    win_streak: f64,
    illuminati_bettors: f64,
    normal_bettors: f64,
    winrate: f64,
    needed_odds: f64,
    simulated_bet: f64,
    odds: f64,
    expected_profit: f64,
    elo: Elo,
}

struct State {
    starting_index: Mutable<usize>,
    records: Vec<(Record, Information, Information)>,
}

impl State {
    fn new(records: Vec<Record>) -> Self {
        let mut simulation: Simulation<CustomStrategy, CustomStrategy> = Simulation::new();

        simulation.matchmaking_strategy = Some(MATCHMAKING_STRATEGY);
        simulation.tournament_strategy = Some(TOURNAMENT_STRATEGY);

        Self {
            starting_index: Mutable::new(0),
            records: records.into_iter()
                .map(|record| {
                    let left_len = simulation.matches_len(&record.left.name, record.tier);
                    let right_len = simulation.matches_len(&record.right.name, record.tier);
                    let specific_matches_len = simulation.specific_matches_len(&record.left.name, &record.right.name, record.tier);

                    // TODO code duplication with bin/chart
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

                    // TODO code duplication with bin/saltybet
                    let (left_winrate, right_winrate) = winrates(&simulation, &record.left.name, &record.right.name, record.tier);
                    let (left_needed_odds, right_needed_odds) = needed_odds(&simulation, &record.left.name, &record.right.name, record.tier);

                    // TODO code duplication with bin/saltybet
                    let (left_bet, right_bet) = match record.mode {
                        Mode::Matchmaking => simulation.matchmaking_strategy.as_ref().unwrap().bet_amount(&simulation,&record.tier, &record.left.name, &record.right.name, record.date),
                        Mode::Tournament => simulation.tournament_strategy.as_ref().unwrap().bet_amount(&simulation, &record.tier, &record.left.name, &record.right.name, record.date),
                    };

                    // TODO code duplication with bin/saltybet
                    let (left_odds, right_odds) = average_odds(&simulation, &record.left.name, &record.right.name, record.tier, left_bet, right_bet);
                    let (left_profit, right_profit) = expected_profits(&simulation, &record.left.name, &record.right.name, record.tier, left_bet, right_bet);

                    let left = Information {
                        // TODO avoid this clone somehow ?
                        name: record.left.name.clone(),
                        matches_len: left_len,
                        specific_matches_len: specific_matches_len,
                        win_streak: record.left.win_streak,
                        illuminati_bettors: record.left.illuminati_bettors,
                        normal_bettors: record.left.normal_bettors,
                        bet_amount: record.left.bet_amount + if let Bet::Left(amount) = record.bet { amount } else { 0.0 },
                        winrate: left_winrate,
                        needed_odds: left_needed_odds,
                        simulated_bet: left_bet,
                        odds: left_odds,
                        expected_profit: left_profit,
                        elo: simulation.elo(&record.left.name, record.tier),
                    };

                    let right = Information {
                        // TODO avoid this clone somehow ?
                        name: record.right.name.clone(),
                        matches_len: right_len,
                        specific_matches_len: specific_matches_len,
                        win_streak: record.right.win_streak,
                        illuminati_bettors: record.right.illuminati_bettors,
                        normal_bettors: record.right.normal_bettors,
                        bet_amount: record.right.bet_amount + if let Bet::Right(amount) = record.bet { amount } else { 0.0 },
                        winrate: right_winrate,
                        needed_odds: right_needed_odds,
                        simulated_bet: right_bet,
                        odds: right_odds,
                        expected_profit: right_profit,
                        elo: simulation.elo(&record.right.name, record.tier),
                    };

                    // TODO code duplication with bin/chart
                    simulation.calculate(&record, &record.bet, 1.0);

                    // TODO code duplication with bin/chart
                    if let Mode::Matchmaking = record.mode {
                        let new_sum = simulation.sum;
                        simulation.insert_sum(new_sum);
                    }

                    // TODO code duplication with bin/chart
                    simulation.insert_record(&record);

                    (record, left, right)
                    /*simulation.calculate(&record, &record.bet);

                    if let Mode::Tournament = record.mode {
                        (simulation.tournament_sum, record)

                    } else {
                        (simulation.sum, record)
                    }*/
                })
                .collect(),
        }
    }
}


fn display_records(records: Vec<Record>) -> Dom {
    let state = Rc::new(State::new(records));

    lazy_static! {
        static ref CLASS_ROOT: String = class! {
            .style("width", "100%")
            .style("height", "100%")
        };

        static ref CLASS_COLORED: String = class! {
            .style("border", "1px solid #6441a5")
            .style("background-color", "#08080a")
        };

        static ref CLASS_BUTTON: String = class! {
            .style("padding", "5px")
            .style("border-radius", "0px")
            .style("color", "white")
            .style("margin", "-1px")
        };

        static ref CLASS_BOLD: String = class! {
            .style("font-weight", "bold")
        };

        static ref CLASS_ALIGN_LEFT: String = class! {
            .style("text-align", "left")
        };

        static ref CLASS_ALIGN_CENTER: String = class! {
            .style("text-align", "center")
        };

        static ref CLASS_ALIGN_RIGHT: String = class! {
            .style("text-align", "right")
        };

        static ref CLASS_NORMAL: String = class! {
            .style("color", "white")
        };

        static ref CLASS_RED: String = class! {
            .style("color", "#D14836")
        };

        static ref CLASS_BLUE: String = class! {
            .style("color", "rgb(52, 158, 255)")
        };

        static ref CLASS_LOSS: String = class! {
            .style("color", "#D14836")
        };

        static ref CLASS_GAIN: String = class! {
            .style("color", "#4db044")
        };

        static ref CLASS_MONEY: String = class! {
            .style("color", "orange")
        };

        static ref CLASS_ODDS_SEPARATOR: String = class! {

        };

        static ref CLASS_TABLE: String = class! {
            .style("width", "100%")
            .style("height", "100%")
            .style("border-spacing", "0px")
            .style("border-collapse", "collapse")
        };

        static ref CLASS_CELL: String = class! {
            .style("border", "1px solid #6441a5")
            .style("padding", "5px")
        };

        static ref CLASS_HEADER: String = class! {
            .style("border-top", "none")
            .style("position", "sticky")
            .style("top", "0px")

            .style("text-align", "center")
            .style("background-color", "#19191f")

            .style("font-size", "14px")
            .style("padding", "5px")
        };

        static ref CLASS_PAGINATOR: String = class! {
            .style("width", "100%")
            .style("display", "flex")
            .style("position", "sticky")
            .style("left", "0px")
            .style("bottom", "0px")
        };

        static ref CLASS_PAGE_SECTION: String = class! {
            .style("display", "flex")
        };

        static ref CLASS_PAGE_BUTTON_SELECTED: String = class! {
            .style("background-color", "#201d2b")
        };

        static ref CLASS_PAGE_ARROW: String = class! {
            .style("padding", "0px 10px")
        };

        static ref CLASS_PAGE_DIVIDER: String = class! {
            .style("padding", "0px 20px")
        };

        static ref CLASS_DISABLED: String = class! {
            .style("color", "gray")
        };
    }

    fn th(name: &str) -> Dom {
        html!("th", {
            .class(&*CLASS_CELL)
            .class(&*CLASS_HEADER)
            .text(name)
        })
    }

    html!("div", {
        .class(&*CLASS_ROOT)

        .children(&mut [
            html!("table", {
                .class(&*CLASS_TABLE)
                .children(&mut [
                    html!("thead", {
                        .children(&mut [
                            html!("tr", {
                                .children(&mut [
                                    th("Mode"),
                                    th("Tier"),
                                    th("Left character"),
                                    th("Right character"),
                                    th("Winner"),
                                    th("Bet"),
                                    th("Odds"),
                                    th("Winner Profit"),
                                    th("Profit"),
                                    th("Profit %"),
                                    th("Sum"),
                                    th("Duration (seconds)"),
                                    th("Date (ISO)"),
                                ])
                            }),
                        ])
                    }),

                    html!("tbody", {
                        .children_signal_vec(state.starting_index.signal().map(clone!(state => move |starting_index| {
                            let ending_index = get_ending_index(starting_index);

                            log!("Displaying indexes {} - {}", starting_index, ending_index);

                            fn date(d: f64) -> String {
                                js!( return new Date(@{d}).toISOString(); ).try_into().unwrap()
                            }

                            fn td(class: &str, children: &mut [Dom]) -> Dom {
                                html!("td", {
                                    .class(&*CLASS_CELL)
                                    .class(class)
                                    .children(children)
                                })
                            }

                            fn text(text: &str) -> Dom {
                                html!("span", {
                                    .text(text)
                                })
                            }

                            fn div(children: &mut [Dom]) -> Dom {
                                html!("div", {
                                    .children(children)
                                })
                            }

                            fn span(class: &str, text: &str) -> Dom {
                                html!("span", {
                                    .class(class)
                                    .text(text)
                                })
                            }

                            fn field(name: &str, child: Dom, order: Ordering) -> Dom {
                                html!("div", {
                                    .children(&mut [
                                        html!("span", {
                                            .class(match order {
                                                Ordering::Equal => &*CLASS_NORMAL,
                                                Ordering::Less => &*CLASS_LOSS,
                                                Ordering::Greater => &*CLASS_GAIN,
                                            })
                                            .text(name)
                                        }),
                                        child,
                                    ])
                                })
                            }

                            fn display_profit(profit: &Profit) -> Dom {
                                match profit {
                                    Profit::Gain(a) => span(&*CLASS_GAIN, &money(*a)),
                                    Profit::Loss(a) => span(&*CLASS_LOSS, &money(-a)),
                                    Profit::None => text(""),
                                }
                            }

                            fn display_elo(name: &str, elo: glicko2::Glicko2Rating, opponent: glicko2::Glicko2Rating) -> Dom {
                                let formatted: glicko2::GlickoRating = elo.into();
                                field(name, text(&format!("{} (\u{00B1} {})", decimal(formatted.value), decimal(formatted.deviation))), elo.value.partial_cmp(&opponent.value).unwrap())
                            }

                            // TODO calculate the illuminati and normal bettors correctly (adding 1 depending on whether it bet or not)
                            fn display_character(this: &Information, other: &Information, class: &str) -> Dom {
                                td(&*CLASS_ALIGN_LEFT, &mut [
                                    field("Name: ", span(class, &this.name), Ordering::Equal),
                                    field("Bet amount: ", span(&*CLASS_MONEY, &money(this.bet_amount)), this.bet_amount.partial_cmp(&other.bet_amount).unwrap()),
                                    field("Win streak: ", text(&this.win_streak.to_string()), this.win_streak.partial_cmp(&other.win_streak).unwrap()),
                                    field("Illuminati bettors: ", text(&this.illuminati_bettors.to_string()), this.illuminati_bettors.partial_cmp(&other.illuminati_bettors).unwrap()),
                                    field("Normal bettors: ", text(&this.normal_bettors.to_string()), this.normal_bettors.partial_cmp(&other.normal_bettors).unwrap()),
                                    field("Number of past matches (in general): ", text(&this.matches_len.to_string()), this.matches_len.cmp(&other.matches_len)),
                                    field("Number of past matches (in specific): ", text(&this.specific_matches_len.to_string()), this.specific_matches_len.cmp(&0)),
                                    field("Winrate: ", text(&format!("{}%", this.winrate * 100.0)), this.winrate.partial_cmp(&other.winrate).unwrap()),
                                    field("Needed odds: ", text(&display_odds(this.needed_odds)), this.needed_odds.partial_cmp(&this.odds).unwrap()),
                                    field("Average odds: ", text(&display_odds(this.odds)), this.odds.partial_cmp(&this.needed_odds).unwrap()),
                                    display_elo("Wins ELO: ", this.elo.wins, other.elo.wins),
                                    display_elo("Upsets ELO: ", this.elo.upsets, other.elo.upsets),
                                    field("Simulated bet amount: ", span(&*CLASS_MONEY, &money(this.simulated_bet)), this.simulated_bet.partial_cmp(&other.simulated_bet).unwrap()),
                                    field("Simulated expected profit: ", span(&*CLASS_MONEY, &money(this.expected_profit)), this.expected_profit.partial_cmp(&other.expected_profit).unwrap()),
                                ])
                            }

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

                            let start_index = state.records.len().saturating_sub(ending_index);
                            let end_index = state.records.len().saturating_sub(starting_index);

                            state.records[start_index..end_index].iter().rev().map(|(record, left_information, right_information)| {
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

                                let (left, right) = record.display_odds();

                                let winner_bet = match bet_amount {
                                    Some(amount) => match record.winner {
                                        Winner::Left => Bet::Left(amount),
                                        Winner::Right => Bet::Right(amount),
                                    },
                                    None => Bet::None,
                                };

                                let winner_profit = record.profit(&winner_bet);

                                html!("tr", {
                                    .children(&mut [
                                        td(&*CLASS_ALIGN_CENTER, &mut [
                                            text(match record.mode {
                                                Mode::Matchmaking => "Matchmaking",
                                                Mode::Tournament => "Tournament",
                                            })
                                        ]),

                                        td(&*CLASS_ALIGN_CENTER, &mut [
                                            text(match record.tier {
                                                Tier::New => "NEW",
                                                Tier::P => "P",
                                                Tier::B => "B",
                                                Tier::A => "A",
                                                Tier::S => "S",
                                                Tier::X => "X",
                                            })
                                        ]),

                                        display_character(
                                            &left_information,
                                            &right_information,
                                            &*CLASS_RED,
                                        ),

                                        display_character(
                                            &right_information,
                                            &left_information,
                                            &*CLASS_BLUE,
                                        ),

                                        td(&*CLASS_ALIGN_CENTER, &mut [
                                            match record.winner {
                                                Winner::Left => span(&*CLASS_RED, "Left"),
                                                Winner::Right => span(&*CLASS_BLUE, "Right"),
                                            }
                                        ]),

                                        td(&*CLASS_ALIGN_RIGHT, &mut [
                                            match record.bet {
                                                Bet::Left(a) => div(&mut [
                                                    span(&*CLASS_RED, "Left "),
                                                    span(&*CLASS_MONEY, &money(a)),
                                                ]),
                                                Bet::Right(a) => div(&mut [
                                                    span(&*CLASS_BLUE, "Right "),
                                                    span(&*CLASS_MONEY, &money(a)),
                                                ]),
                                                Bet::None => text(""),
                                            }
                                        ]),

                                        td(&*CLASS_ALIGN_CENTER, &mut [
                                            span(&*CLASS_RED, &decimal(left)),
                                            span(&*CLASS_ODDS_SEPARATOR, " : "),
                                            span(&*CLASS_BLUE, &decimal(right)),
                                        ]),

                                        td(&*CLASS_ALIGN_RIGHT, &mut [
                                            display_profit(&winner_profit)
                                        ]),

                                        td(&*CLASS_ALIGN_RIGHT, &mut [
                                            display_profit(&profit)
                                        ]),

                                        td(&*CLASS_ALIGN_RIGHT, &mut [
                                            match bet_amount {
                                                Some(a) => match profit {
                                                    Profit::Gain(b) => span(&*CLASS_GAIN, &percentage(b / a)),
                                                    Profit::Loss(b) => span(&*CLASS_LOSS, &percentage(-(b / a))),
                                                    Profit::None => text(""),
                                                },
                                                None => text(""),
                                            }
                                        ]),

                                        td(&*CLASS_ALIGN_RIGHT, &mut [
                                            if let Some(sum) = sum {
                                                span(&*CLASS_MONEY, &money(sum))

                                            } else {
                                                text("")
                                            }
                                        ]),

                                        td(&*CLASS_ALIGN_RIGHT, &mut [
                                            text(&(record.duration / 1000.0).floor().to_string())
                                        ]),

                                        td(&*CLASS_ALIGN_CENTER, &mut [
                                            text(&date(record.date))
                                        ]),
                                    ])
                                })
                            }).collect()
                        })).to_signal_vec())
                    }),
                ])
            }),

            {
                fn page_divider() -> Dom {
                    html!("div", {
                        .class(&*CLASS_COLORED)
                        .class(&*CLASS_BUTTON)
                        .class(&*CLASS_DISABLED)
                        .class(&*CLASS_PAGE_DIVIDER)
                        .text("...")
                    })
                }

                html!("div", {
                    .class(&*CLASS_COLORED)
                    .class(&*CLASS_PAGINATOR)

                    .children_signal_vec(state.starting_index.signal().map(clone!(state => move |starting_index| {
                        let ending_index = get_ending_index(starting_index);

                        let starting_cutoff = (5 * PAGE_LIMIT).min(state.records.len());
                        let ending_cutoff = state.records.len().saturating_sub(5 * PAGE_LIMIT);

                        let middle_starting_cutoff = starting_index
                            .saturating_sub(7 * PAGE_LIMIT)
                            .min(ending_cutoff.saturating_sub(15 * PAGE_LIMIT))
                            .max(starting_cutoff);

                        let middle_ending_cutoff = middle_starting_cutoff.saturating_add(15 * PAGE_LIMIT);

                        let mut starting_pages = vec![];
                        let mut middle_pages = vec![];
                        let mut ending_pages = vec![];

                        for (index, new_starting_index) in (0..state.records.len()).step_by(PAGE_LIMIT).enumerate() {
                            let pages = if new_starting_index < starting_cutoff {
                                Some(&mut starting_pages)

                            // TODO is this correct ?
                            } else if new_starting_index >= ending_cutoff {
                                Some(&mut ending_pages)

                            } else if new_starting_index >= middle_starting_cutoff &&
                                      new_starting_index < middle_ending_cutoff {
                                Some(&mut middle_pages)

                            } else {
                                None
                            };

                            if let Some(pages) = pages {
                                let new_ending_index = get_ending_index(new_starting_index);

                                pages.push(html!("button", {
                                    .class(&*CLASS_COLORED)
                                    .class(&*CLASS_BUTTON)
                                    .class(&*CLASS_BOLD)

                                    .apply_if(new_starting_index >= starting_index && new_ending_index <= ending_index, |dom| {
                                        dom.class(&*CLASS_PAGE_BUTTON_SELECTED)
                                    })

                                    .style("flex", "1")
                                    .text(&(index.saturating_add(1)).to_string())
                                    .event(clone!(state => move |_: ClickEvent| {
                                        state.starting_index.set_neq(new_starting_index);
                                    }))
                                }));
                            }
                        }

                        vec![
                            html!("button", {
                                .class(&*CLASS_COLORED)
                                .class(&*CLASS_BUTTON)
                                .class(&*CLASS_BOLD)
                                .class(&*CLASS_PAGE_ARROW)

                                .apply_if(starting_index == 0, |dom| {
                                    dom.class(&*CLASS_DISABLED)
                                })

                                .text("<")
                                .event(clone!(state => move |_: ClickEvent| {
                                    // TODO replace_with_neq ?
                                    let starting_index = state.starting_index.get();
                                    state.starting_index.set_neq(starting_index.saturating_sub(PAGE_LIMIT));
                                }))
                            }),

                            html!("div", {
                                .class(&*CLASS_PAGE_SECTION)
                                .style("flex", "1")
                                .children(&mut starting_pages)
                            }),

                            page_divider(),

                            html!("div", {
                                .class(&*CLASS_PAGE_SECTION)
                                .style("flex", "3")
                                .children(&mut middle_pages)
                            }),

                            page_divider(),

                            html!("div", {
                                .class(&*CLASS_PAGE_SECTION)
                                .style("flex", "1")
                                .children(&mut ending_pages)
                            }),

                            {
                                let len = state.records.len().saturating_sub(PAGE_LIMIT);

                                html!("button", {
                                    .class(&*CLASS_COLORED)
                                    .class(&*CLASS_BUTTON)
                                    .class(&*CLASS_BOLD)
                                    .class(&*CLASS_PAGE_ARROW)

                                    .apply_if(starting_index >= len, |dom| {
                                        dom.class(&*CLASS_DISABLED)
                                    })

                                    .text(">")
                                    .event(clone!(state => move |_: ClickEvent| {
                                        // TODO replace_with_neq ?
                                        let starting_index = state.starting_index.get();

                                        if starting_index < len {
                                            let starting_index = starting_index.saturating_add(PAGE_LIMIT);
                                            state.starting_index.set_neq(starting_index);
                                        }
                                    }))
                                })
                            },
                        ]
                    })).to_signal_vec())
                })
            },
        ])
    })
}


async fn main_future() -> Result<(), Error> {
    set_panic_hook();

    log!("Initializing...");

    stylesheet!("*", {
        .style("box-sizing", "border-box")
        .style("margin", "0px")
        .style("padding", "0px")
        .style("font-size", "12px")
    });

    stylesheet!("html, body", {
        .style("width", "100%")
        .style("height", "100%")
    });

    stylesheet!("body", {
        .style("overflow", "auto")
        .style("background-color", "#201d2b")
        .style("color", "white")
    });

    let loading = Loading::new();

    document().body().unwrap().append_child(loading.element());

    let records = records_get_all().await?;

    dominator::append_dom(&dominator::body(), display_records(records));

    loading.hide();

    Ok(())
}

fn main() {
    stdweb::initialize();

    spawn_local(unwrap_future(main_future()));

    stdweb::event_loop();
}
