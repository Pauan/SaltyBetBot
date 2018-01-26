#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate lazy_static;
extern crate csv;
extern crate regex;
extern crate rand;
extern crate rayon;

/*use std::fs::File;
use std::path::Path;
use std::io::Read;*/

use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::File;

mod record;
mod genetic;
mod simulation;

mod saltybet {
    pub mod query;
}


/*fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(&Path::new(path))?;

    let mut s = String::new();

    file.read_to_string(&mut s)?;

    Ok(s)
}*/


fn write_file(filename: &str) -> Result<(), std::io::Error> {
    let records = {
        let data = include_str!("../../../../Salty Bet/saltyRecordsM--2018-1-16-14.29.txt");
        record::parse_csv(&data).unwrap()
    };

    let settings = genetic::SimulationSettings {
        mode: record::Mode::Tournament,
        records: &records,
    };

    let mut population: genetic::Population<genetic::BetStrategy, genetic::SimulationSettings> = genetic::Population::new(1000, &settings);

    println!("Initializing...");

    population.init();

    // TODO file an issue for Rust about adding in documentation to File encouraging people to use BufWriter
    let mut buffer = BufWriter::new(File::create(filename)?);

    {
        let best = population.best();
        write!(buffer, "{:#?}\n", population.populace)?;
        write!(buffer, "<<<<<<<<<<<<<<<<<<<<<<<<<<\n")?;
        buffer.flush()?;
        println!("Initialized: {}", best.fitness);
    }

    for i in 0..1000 {
        population.next_generation();

        let best = population.best();
        write!(buffer, "{:#?}\n", best)?;
        buffer.flush()?;
        println!("Generation {}: {}", i + 1, best.fitness);
    }

    write!(buffer, ">>>>>>>>>>>>>>>>>>>>>>>>>>\n")?;
    write!(buffer, "{:#?}\n", population.populace)?;
    buffer.flush()?;

    Ok(())
}


fn run_simulation() {
    use genetic::{ BetStrategy, CubicBezierSegment, Point, Percentage };
    use genetic::BooleanCalculator::*;
    use genetic::NumericCalculator::*;
    use simulation::Lookup::*;
    use simulation::LookupSide::*;
    use simulation::LookupStatistic::*;
    use simulation::LookupFilter::*;

    let records = {
        let data = include_str!("../../../../Salty Bet/saltyRecordsM--2018-1-16-14.29.txt");
        record::parse_csv(&data).unwrap()
    };

    println!("Running...");

    let mut simulation: simulation::Simulation<BetStrategy, BetStrategy> = simulation::Simulation::new();

    simulation.matchmaking_strategy = None;

    simulation.tournament_strategy = None;

    simulation.simulate(&records);

    println!("{} {} {} {} {} {}",
        simulation.sum,
        simulation.successes,
        simulation.failures,
        simulation.record_len,
        simulation.characters.len(),
        simulation.max_character_len
    );
}


#[cfg(any(target_arch = "wasm32", target_arch = "asmjs"))]
fn main() {
    stdweb::initialize();

    /*fn lookup() {
        stdweb::web::set_timeout(lookup, 10000);

        let start: f64 = stdweb::web::Date::now();
        let messages = saltybet::query::get_waifu_messages();
        let end: f64 = stdweb::web::Date::now();

        let start2: f64 = stdweb::web::Date::now();
        println!("{:#?} {:#?}", end - start, messages);
        let end2: f64 = stdweb::web::Date::now();

        println!("{:#?}", end2 - start2);
    }

    lookup();*/

    //run_simulation();

    saltybet::query::observe_changes(|observer| {
        println!("{:#?}", observer);
        std::mem::forget(observer);
    });

    stdweb::event_loop();
}


#[cfg(not(any(target_arch = "wasm32", target_arch = "asmjs")))]
fn main() {
    //stdweb::initialize();

    /*let bezier = CubicBezierSegment {
        from: Point::new(0.83253485,0.018677153),
        ctrl1: Point::new(0.08993364,0.018677153),
        ctrl2: Point::new(0.46272424,0.018678138),
        to: Point::new(0.65694433,0.018677153)
    };

    println!("{:#?}", bezier.sample_y(0.0));
    println!("{:#?}", bezier.sample_y(0.1));
    println!("{:#?}", bezier.sample_y(0.2));
    println!("{:#?}", bezier.sample_y(0.3));
    println!("{:#?}", bezier.sample_y(0.4));
    println!("{:#?}", bezier.sample_y(0.5));
    println!("{:#?}", bezier.sample_y(0.6));
    println!("{:#?}", bezier.sample_y(0.7));
    println!("{:#?}", bezier.sample_y(0.8));
    println!("{:#?}", bezier.sample_y(0.9));
    println!("{:#?}", bezier.sample_y(1.0));*/

    write_file("tmp").unwrap();

    /*js! {
        console.log(@{format!("{:#?}", records)});
    };*/

    /*js! {
        console.log(@{format!("{:#?}", (2, "hi"))});
    }*/

    //println!("{:#?}", "hi!");

    //stdweb::event_loop();
}
