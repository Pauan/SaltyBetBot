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

mod record;
mod genetic;


/*fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(&Path::new(path))?;

    let mut s = String::new();

    file.read_to_string(&mut s)?;

    Ok(s)
}*/


fn main() {
    //stdweb::initialize();

    let mut records = {
        let data = include_str!("../../../../Salty Bet/saltyRecordsM--2018-1-1-19.39.txt");
        record::parse_csv(&data).unwrap()
    };

    /*js! {
        console.log(@{format!("{:#?}", records)});
    };*/

    let mut population: genetic::Population<genetic::OddsStrategy, Vec<record::Record>> = genetic::Population::new(1000, &mut records);

    population.init();

    println!("{:#?}", population.best());

    for _ in 0..100 {
        population.next_generation();
        println!("{:#?}", population.best());
    }

    /*js! {
        console.log(@{format!("{:#?}", (2, "hi"))});
    }*/

    //println!("{:#?}", "hi!");

    //stdweb::event_loop();
}
