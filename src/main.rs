//#[macro_use]
//extern crate stdweb;
#[macro_use]
extern crate lazy_static;
extern crate csv;
extern crate regex;
extern crate lyon_bezier;
extern crate rand;
extern crate rayon;

/*use std::fs::File;
use std::path::Path;
use std::io::Read;*/

use lyon_bezier::Point;
use lyon_bezier::cubic_bezier::CubicBezierSegment;

mod record;
mod genetic;


const WEB_BUILD: bool = false;


/*fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(&Path::new(path))?;

    let mut s = String::new();

    file.read_to_string(&mut s)?;

    Ok(s)
}*/


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

    let records = {
        let data = include_str!("../../../../Salty Bet/saltyRecordsM--2018-1-6-20.6.txt");
        record::parse_csv(&data).unwrap()
    };

    let settings = genetic::SimulationSettings {
        mode: record::Mode::Tournament,
        records: &records,
    };

    let mut population: genetic::Population<genetic::BetStrategy, genetic::SimulationSettings> = genetic::Population::new(1000, &settings);

    population.init();

    println!("{:#?}", population.populace);

    for _ in 0..100 {
        population.next_generation();
    }

    println!("--------------------------");
    println!("{:#?}", population.populace);

    /*js! {
        console.log(@{format!("{:#?}", records)});
    };*/

    /*js! {
        console.log(@{format!("{:#?}", (2, "hi"))});
    }*/

    //println!("{:#?}", "hi!");

    //stdweb::event_loop();
}
