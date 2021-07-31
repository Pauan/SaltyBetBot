#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate algorithm;

use algorithm::types;
use std::io::{BufReader, BufWriter};
use std::fs::File;


fn read_strategy<A>(filename: &str) -> Result<A, std::io::Error> where for<'de> A: serde::Deserialize<'de> {
    let buffer = BufReader::new(File::open(filename)?);
    Ok(serde_json::from_reader(buffer)?)
}

fn write_strategy<A: serde::Serialize>(filename: &str, strategy: &A) -> Result<(), std::io::Error> {
    let buffer = BufWriter::new(File::create(filename)?);
    Ok(serde_json::to_writer_pretty(buffer, strategy)?)
}

fn map_strategy<A, B, F>(filename: &str, f: F) -> Result<(), std::io::Error>
    where for<'de> A: serde::Deserialize<'de>,
          B: serde::Serialize,
          F: FnOnce(A) -> B {
    write_strategy(filename, &f(read_strategy(filename)?))
}


mod epoch1 {
    use algorithm::types;
    use algorithm::types::{Percentage, CubicBezierSegment, Lookup};


    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    enum NumericCalculator<A, B> {
        Base(A),
        Fixed(B),
        Percentage(Percentage),

        Bezier(CubicBezierSegment, Box<NumericCalculator<A, B>>),
        Average(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
        Abs(Box<NumericCalculator<A, B>>),
        Min(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
        Max(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
        Plus(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
        Minus(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
        Multiply(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
        Divide(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),

        // TODO change to use BooleanCalculator<NumericCalculator<A, B>>
        IfThenElse(BooleanCalculator<A>, Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),

        Tier {
            x: Box<NumericCalculator<A, B>>,
            s: Box<NumericCalculator<A, B>>,
            a: Box<NumericCalculator<A, B>>,
            b: Box<NumericCalculator<A, B>>,
            p: Box<NumericCalculator<A, B>>,
        },
    }

    impl<A> Into<types::NumericCalculator<A, f64>> for NumericCalculator<A, f64> {
        fn into(self) -> types::NumericCalculator<A, f64> {
            match self {
                NumericCalculator::Base(a) => types::NumericCalculator::Base(a),
                NumericCalculator::Fixed(a) => types::NumericCalculator::Fixed(a),
                NumericCalculator::Percentage(a) => types::NumericCalculator::Percentage(a),
                NumericCalculator::Bezier(a, b) => types::NumericCalculator::Bezier(a, Box::new((*b).into())),
                NumericCalculator::Average(a, b) => types::NumericCalculator::Average(Box::new((*a).into()), Box::new((*b).into())),
                NumericCalculator::Abs(a) => types::NumericCalculator::Abs(Box::new((*a).into())),
                NumericCalculator::Min(a, b) => types::NumericCalculator::Min(Box::new((*a).into()), Box::new((*b).into())),
                NumericCalculator::Max(a, b) => types::NumericCalculator::Max(Box::new((*a).into()), Box::new((*b).into())),
                NumericCalculator::Plus(a, b) => types::NumericCalculator::Plus(Box::new((*a).into()), Box::new((*b).into())),
                NumericCalculator::Minus(a, b) => types::NumericCalculator::Minus(Box::new((*a).into()), Box::new((*b).into())),
                NumericCalculator::Multiply(a, b) => types::NumericCalculator::Multiply(Box::new((*a).into()), Box::new((*b).into())),
                NumericCalculator::Divide(a, b) => types::NumericCalculator::Divide(Box::new((*a).into()), Box::new((*b).into())),
                NumericCalculator::IfThenElse(a, b, c) => types::NumericCalculator::IfThenElse(a.into(), Box::new((*b).into()), Box::new((*c).into())),
                NumericCalculator::Tier { x, s, a, b, p } => panic!(), /*types::NumericCalculator::Tier {
                    // TODO better value for this
                    new: Box::new(types::NumericCalculator::Fixed(0.0)),
                    x: Box::new((*x).into()),
                    s: Box::new((*s).into()),
                    a: Box::new((*a).into()),
                    b: Box::new((*b).into()),
                    p: Box::new((*p).into()),
                },*/
            }
        }
    }


    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    enum BooleanCalculator<A> {
        True,
        False,
        Greater(A, A),
        GreaterEqual(A, A),
        Lesser(A, A),
        LesserEqual(A, A),
        And(Box<BooleanCalculator<A>>, Box<BooleanCalculator<A>>),
        Or(Box<BooleanCalculator<A>>, Box<BooleanCalculator<A>>),
    }

    impl<A: Into<B>, B> Into<types::BooleanCalculator<B>> for BooleanCalculator<A> {
        fn into(self) -> types::BooleanCalculator<B> {
            match self {
                BooleanCalculator::True => types::BooleanCalculator::True,
                BooleanCalculator::False => types::BooleanCalculator::False,
                BooleanCalculator::Greater(a, b) => types::BooleanCalculator::Greater(a.into(), b.into()),
                BooleanCalculator::GreaterEqual(a, b) => types::BooleanCalculator::GreaterEqual(a.into(), b.into()),
                BooleanCalculator::Lesser(a, b) => types::BooleanCalculator::Lesser(a.into(), b.into()),
                BooleanCalculator::LesserEqual(a, b) => types::BooleanCalculator::LesserEqual(a.into(), b.into()),
                BooleanCalculator::And(a, b) => types::BooleanCalculator::And(Box::new((*a).into()), Box::new((*b).into())),
                BooleanCalculator::Or(a, b) => types::BooleanCalculator::Or(Box::new((*a).into()), Box::new((*b).into())),
            }
        }
    }


    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FormulaStrategy {
        fitness: f64,
        successes: f64,
        failures: f64,
        record_len: f64,
        characters_len: usize,
        max_character_len: usize,
        bet_strategy: BooleanCalculator<NumericCalculator<Lookup, f64>>,
        prediction_strategy: NumericCalculator<Lookup, f64>,
        money_strategy: NumericCalculator<Lookup, f64>,
    }

    impl Into<types::FormulaStrategy> for FormulaStrategy {
        fn into(self) -> types::FormulaStrategy {
            types::FormulaStrategy {
                bet_strategy: self.bet_strategy.into(),
                prediction_strategy: self.prediction_strategy.into(),
                money_strategy: self.money_strategy.into(),
            }
        }
    }
}


fn main() {
    fn migrate(from: epoch1::FormulaStrategy) -> types::FormulaStrategy {
        from.into()
    }

    map_strategy("strategies/matchmaking_strategy", migrate).unwrap();
    map_strategy("strategies/tournament_strategy", migrate).unwrap();
}
