#![allow(dead_code)]

extern crate indicatif;
extern crate rayon;
extern crate algorithm;
extern crate chrono;
extern crate serde;
extern crate serde_json;

use algorithm::genetic;
use algorithm::genetic::Creature;
use algorithm::types::FitnessResult;
use algorithm::record::{Record, Mode};
use algorithm::simulation::Strategy;
use algorithm::strategy::{CustomStrategy, Permutate};
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

    let mut population: genetic::Population<genetic::FormulaStrategy, genetic::SimulationSettings> = genetic::Population::new(1000, &settings);

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


/*fn read_strategy(filename: &str) -> Result<genetic::FormulaStrategy, std::io::Error> {
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

fn simulate<A>(progress_bar: &indicatif::ProgressBar, mode: Mode, records: &mut [Record]) -> FitnessResult<A>
    where A: Creature + Clone + Send + Sync {

    let records = shuffle_records(records, mode);

    let settings = genetic::SimulationSettings {
        mode,
        records: &records,
    };

    let mut population: genetic::Population<A, _> = genetic::Population::new(POPULATION_SIZE, &settings);

    population.init();
    progress_bar.inc(1);

    for _ in 0..GENERATIONS {
        population.next_generation();
        progress_bar.inc(1);
    }

    // TODO figure out a way to avoid using clone
    population.best().clone()
}

fn test_strategy<A>(mode: Mode, records: &mut [Record], strategy: A) -> FitnessResult<A> where A: Strategy + Clone {
    let records = shuffle_records(records, mode);
    FitnessResult::new(&genetic::SimulationSettings { mode, records: &records }, strategy)
}


/*fn run_old_simulation(left: &mut [Record], right: &mut [Record]) -> Result<(), std::io::Error> {
    let matchmaking_strategy: types::FormulaStrategy = read("../strategies/matchmaking_strategy")?;
    let tournament_strategy: types::FormulaStrategy = read("../strategies/tournament_strategy")?;
    println!("Matchmaking Old   -> {}   -> {}",
        test_strategy(Mode::Matchmaking, left, matchmaking_strategy.clone()),
        test_strategy(Mode::Matchmaking, right, matchmaking_strategy));
    println!("Tournament Old   -> {}   -> {}\n",
        test_strategy(Mode::Tournament, left, tournament_strategy.clone()),
        test_strategy(Mode::Tournament, right, tournament_strategy));
    Ok(())
}*/


fn run_strategy<A: Strategy + Clone>(name: &str, left: &mut [Record], right: &mut [Record], strategy: A) {
    println!("Matchmaking {}   -> {}   -> {}", name,
        test_strategy(Mode::Matchmaking, left, strategy.clone()).fitness,
        test_strategy(Mode::Matchmaking, right, strategy.clone()).fitness);
    /*println!("Tournament {}   -> {}   -> {}\n", name,
        test_strategy(Mode::Tournament, left, strategy.clone()),
        test_strategy(Mode::Tournament, right, strategy));*/
}


fn run_bet_strategy<A>(left: &mut [Record], right: &mut [Record]) -> Result<(), std::io::Error>
    where A: Creature + Clone + Send + Sync + Serialize {

    let date = current_time();

    let progress_bar = indicatif::ProgressBar::new(GENERATIONS + 1);

    let matchmaking = simulate::<A>(&progress_bar, Mode::Matchmaking, left);
    //let tournament = simulate::<A>(&progress_bar, Mode::Tournament, left);

    progress_bar.finish_and_clear();

    let matchmaking_test = test_strategy(Mode::Matchmaking, right, matchmaking.creature.clone());
    //let tournament_test = test_strategy(Mode::Tournament, right, tournament.creature.clone());

    println!("Matchmaking Genetic  {} -> {}", matchmaking.fitness, matchmaking_test.fitness);
    //println!("Tournament Genetic  {} -> {}", tournament.fitness, tournament_test);

    write(&format!("../strategies/{} (matchmaking)", date), &matchmaking)?;
    //write(&format!("../strategies/{} (tournament)", date), &tournament)?;

    Ok(())
}


fn run_simulation() -> Result<(), std::io::Error> {
    let records: Vec<Record> = read("../static/SaltyBet Records.json")?;

    println!("Read in {} records\n", records.len());

    let (mut left, mut right) = split_records(records);

    let mut strategies: Vec<FitnessResult<CustomStrategy>> = vec![];

    Permutate::each(|strategy| {
        strategies.push(test_strategy(Mode::Matchmaking, &mut left, strategy));
    });

    strategies.sort_by(|x, y| x.fitness.partial_cmp(&y.fitness).unwrap());

    println!("{:#?}", &strategies[(strategies.len() - 10)..]);

    //run_strategy("Default (matchmaking)", &mut left, &mut right, MATCHMAKING_STRATEGY);
    //run_strategy("Default (tournament)", &mut left, &mut right, TOURNAMENT_STRATEGY);

    //run_old_simulation(&mut left, &mut right)?;
    //run_bet_strategy::<CustomStrategy>(&mut left, &mut right)?;

    Ok(())
}

fn main() {
    run_simulation().unwrap();
}
