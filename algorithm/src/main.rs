#![allow(dead_code)]

extern crate indicatif;
extern crate rayon;
extern crate algorithm;
extern crate chrono;
extern crate serde;
extern crate serde_json;

use algorithm::{genetic, types};
use algorithm::record::{Record, Mode};
use algorithm::simulation::Strategy;
use algorithm::strategy::{EarningsStrategy, AllInStrategy};
use algorithm::random::shuffle;

use serde::Serialize;
use serde::de::DeserializeOwned;
use chrono::offset::Utc;
use std::borrow::Borrow;
use std::io::{BufReader, BufWriter};
use std::fs::File;


const POPULATION_SIZE: usize = 100;
const GENERATIONS: u64 = 200;


/*fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(&Path::new(path))?;

    let mut s = String::new();

    file.read_to_string(&mut s)?;

    Ok(s)
}*/


/*fn write_file(filename: &str) -> Result<(), std::io::Error> {
    let records = {
        let data = include_str!("../records/saltyRecordsM--2018-1-16-14.29.txt");
        record::parse_csv(&data).unwrap()
    };

    let settings = genetic::SimulationSettings {
        mode: record::Mode::Tournament,
        records: &records,
    };

    let mut population: genetic::Population<genetic::BetStrategy, genetic::SimulationSettings> = genetic::Population::new(1000, &settings);

    log!("Initializing...");

    population.init();

    // TODO file an issue for Rust about adding in documentation to File encouraging people to use BufWriter
    let mut buffer = BufWriter::new(File::create(filename)?);

    {
        let best = population.best();
        write!(buffer, "{:#?}\n", population.populace)?;
        write!(buffer, "<<<<<<<<<<<<<<<<<<<<<<<<<<\n")?;
        buffer.flush()?;
        log!("Initialized: {}", best.fitness);
    }

    for i in 0..1000 {
        population.next_generation();

        let best = population.best();
        write!(buffer, "{:#?}\n", best)?;
        buffer.flush()?;
        log!("Generation {}: {}", i + 1, best.fitness);
    }

    write!(buffer, ">>>>>>>>>>>>>>>>>>>>>>>>>>\n")?;
    write!(buffer, "{:#?}\n", population.populace)?;
    buffer.flush()?;

    Ok(())
}*/


/*fn read_strategy(filename: &str) -> Result<genetic::BetStrategy, std::io::Error> {
    let buffer = BufReader::new(File::open(filename)?);
    Ok(serde_json::from_reader(buffer)?)
}

fn write_strategy<A: simulation::Strategy + serde::Serialize>(filename: &str, strategy: &A) -> Result<(), std::io::Error> {
    let buffer = BufWriter::new(File::create(filename)?);
    Ok(serde_json::to_writer_pretty(buffer, strategy)?)
}*/


fn read<A: DeserializeOwned>(path: &str) -> std::io::Result<A> {
    let reader = BufReader::new(File::open(path)?);
    Ok(serde_json::from_reader(reader)?)
}

fn write<A: Serialize>(path: &str, value: &A) -> std::io::Result<()> {
    let writer = BufWriter::new(File::create(path)?);
    Ok(serde_json::to_writer_pretty(writer, value)?)
}

fn current_time() -> String {
    Utc::now().format("%FT%H.%M.%S").to_string()
}

fn find_nearest_index(records: &[Record]) -> usize {
    let index = records.len() / 2;

    if records[index].mode == Mode::Matchmaking {
        index

    } else {
        let left_index = records[..index].iter().rposition(|x| x.mode == Mode::Matchmaking).unwrap();
        let right_index = records[(index + 1)..].iter().position(|x| x.mode == Mode::Matchmaking).unwrap();

        if (index - left_index) < (right_index - index) {
            left_index

        } else {
            right_index
        }
    }
}

// TODO rather than splitting it in the middle, instead rotate it around a random point
fn split_records(mut records: Vec<Record>) -> (Vec<Record>, Vec<Record>) {
    let index = find_nearest_index(&records);
    let right = records.split_off(index);
    (records, right)
}


#[derive(Debug)]
pub struct Boundary {
    pub mode: Mode,
    pub start: usize,
    pub end: usize,
}

pub struct BoundaryIterator<A> {
    iter: Option<A>,
    matchmaking: usize,
    tournament: usize,
    index: usize,
}

impl<A, B: Borrow<Record>> Iterator for BoundaryIterator<A> where A: Iterator<Item = B> {
    type Item = Boundary;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut output = None;

            match self.iter.as_mut().and_then(|x| x.next()) {
                Some(record) => match record.borrow().mode {
                    Mode::Matchmaking => {
                        if self.tournament != self.index {
                            output = Some(Boundary {
                                mode: Mode::Tournament,
                                start: self.tournament,
                                end: self.index,
                            });
                        }

                        self.index += 1;
                        self.tournament = self.index;
                    },
                    // TODO compare the dates to ensure that it's the same tournament
                    Mode::Tournament => {
                        if self.matchmaking != self.index {
                            output = Some(Boundary {
                                mode: Mode::Matchmaking,
                                start: self.matchmaking,
                                end: self.index,
                            });
                        }

                        self.index += 1;
                        self.matchmaking = self.index;
                    },
                },

                None => {
                    self.iter = None;

                    if self.matchmaking != self.index {
                        output = Some(Boundary {
                            mode: Mode::Matchmaking,
                            start: self.matchmaking,
                            end: self.index,
                        });

                        self.matchmaking = self.index;

                    } else if self.tournament != self.index {
                        output = Some(Boundary {
                            mode: Mode::Tournament,
                            start: self.tournament,
                            end: self.index,
                        });

                        self.tournament = self.index;

                    } else {
                        return None;
                    }
                },
            }

            if output.is_some() {
                return output;
            }
        }
    }
}

// TODO have this return an iterator instead
pub fn boundaries<A: IntoIterator<Item = B>, B: Borrow<Record>>(records: A) -> BoundaryIterator<A::IntoIter> {
    BoundaryIterator {
        iter: Some(records.into_iter()),
        matchmaking: 0,
        tournament: 0,
        index: 0,
    }
}


fn shuffle_records(records: &[Record], mode: Mode) -> Vec<Record> {
    match mode {
        Mode::Matchmaking => {
            let mut records: Vec<Record> = records.iter().filter(|x| x.mode == Mode::Matchmaking).cloned().collect();
            shuffle(&mut records);
            records
        },
        // TODO shuffle this too
        Mode::Tournament => {
            records.to_vec()
        },
    }
}

fn simulate(progress_bar: &indicatif::ProgressBar, mode: Mode, records: &mut [Record]) -> types::BetStrategy {
    let records = shuffle_records(records, mode);

    let settings = genetic::SimulationSettings {
        mode,
        records: &records,
    };

    let mut population: genetic::Population<types::BetStrategy, genetic::SimulationSettings> = genetic::Population::new(POPULATION_SIZE, &settings);

    population.init();
    progress_bar.inc(1);

    for _ in 0..GENERATIONS {
        population.next_generation();
        progress_bar.inc(1);
    }

    // TODO figure out a way to avoid using clone
    population.best().clone()
}

fn test_strategy<A: Strategy>(mode: Mode, records: &mut [Record], strategy: A) -> f64 {
    let records = shuffle_records(records, mode);
    genetic::SimulationSettings { mode, records: &records }.calculate_fitness(strategy)
}


fn run_old_simulation(left: &mut [Record], right: &mut [Record]) -> Result<(), std::io::Error> {
    let matchmaking_strategy: types::BetStrategy = read("../strategies/matchmaking_strategy")?;
    let tournament_strategy: types::BetStrategy = read("../strategies/tournament_strategy")?;
    println!("Matchmaking Old   -> {}   -> {}",
        test_strategy(Mode::Matchmaking, left, matchmaking_strategy.clone()),
        test_strategy(Mode::Matchmaking, right, matchmaking_strategy));
    println!("Tournament Old   -> {}   -> {}\n",
        test_strategy(Mode::Tournament, left, tournament_strategy.clone()),
        test_strategy(Mode::Tournament, right, tournament_strategy));
    Ok(())
}


fn run_strategy<A: Strategy + Copy>(name: &str, left: &mut [Record], right: &mut [Record], strategy: A) {
    println!("Matchmaking {}   -> {}   -> {}", name,
        test_strategy(Mode::Matchmaking, left, strategy),
        test_strategy(Mode::Matchmaking, right, strategy));
    println!("Tournament {}   -> {}   -> {}\n", name,
        test_strategy(Mode::Tournament, left, strategy),
        test_strategy(Mode::Tournament, right, strategy));
}


fn run_bet_strategy(left: &mut [Record], right: &mut [Record]) -> Result<(), std::io::Error> {
    let date = current_time();

    let progress_bar = indicatif::ProgressBar::new((GENERATIONS + 1) * 2);

    let matchmaking = simulate(&progress_bar, Mode::Matchmaking, left);
    let tournament = simulate(&progress_bar, Mode::Tournament, left);

    progress_bar.finish_and_clear();

    let matchmaking_test = test_strategy(Mode::Matchmaking, right, matchmaking.clone());
    let tournament_test = test_strategy(Mode::Tournament, right, tournament.clone());

    println!("Matchmaking Genetic  {} -> {}", matchmaking.fitness, matchmaking_test);
    println!("Tournament Genetic  {} -> {}", tournament.fitness, tournament_test);

    write(&format!("../strategies/{} (matchmaking)", date), &matchmaking)?;
    write(&format!("../strategies/{} (tournament)", date), &tournament)?;

    Ok(())
}


fn run_simulation() -> Result<(), std::io::Error> {
    let records: Vec<Record> = read("../records/SaltyBet Records (2018-08-20T10_33_48.574Z).json")?;
    println!("Read in {} records\n", records.len());

    let (mut left, mut right) = split_records(records);

    run_strategy("Earnings", &mut left, &mut right, EarningsStrategy {
        use_percentages: true,
        expected_profit: true,
        winrate: false,
        bet_difference: false,
        winrate_difference: false,
    });

    run_strategy("Winrate", &mut left, &mut right, EarningsStrategy {
        use_percentages: true,
        expected_profit: false,
        winrate: true,
        bet_difference: false,
        winrate_difference: false,
    });

    run_strategy("AllIn", &mut left, &mut right, AllInStrategy);

    //run_old_simulation(&mut left, &mut right)?;
    run_bet_strategy(&mut left, &mut right)?;

    Ok(())
}

fn main() {
    run_simulation().unwrap();
}
