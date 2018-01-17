use std;
use std::collections::{ HashMap };
use lyon_bezier::Point;
use lyon_bezier::cubic_bezier::CubicBezierSegment;
use rand;
use rand::{ Rng, Closed01 };
use record::{ Record, Mode, Winner };
//use rayon::iter::{ IntoParallelRefIterator, ParallelIterator, FromParallelIterator, IntoParallelIterator };
use rayon::prelude::*;


const MAX_VEC_LEN: f64 = 10000.0;
const MAX_BET_AMOUNT: f64 = 1000000.0;
const MUTATION_RATE: Percentage = Percentage(0.10);
const MAX_RECURSION_DEPTH: u32 = 6; // 64 maximum nodes

const SALT_MINE_AMOUNT: f64 = 258.0; // TODO verify that this is correct
const TOURNAMENT_BALANCE: f64 = 1375.0; // TODO


trait Calculate<A> {
    fn calculate<'a, 'b, 'c, B, C>(&self, &Simulation<'a, 'b, 'c, B, C>, &'c str, &'c str) -> A
        where B: Strategy,
              C: Strategy;
}


#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
struct Percentage(f64);

impl Percentage {
    fn unwrap(&self) -> f64 {
        let Percentage(value) = *self;
        value
    }
}


enum Bet {
    Left(f64),
    Right(f64),
    None,
}


trait Strategy: Sized + std::fmt::Debug {
    fn bet<A, B>(&self, simulation: &Simulation<A, B>, left: &str, right: &str) -> Bet where A: Strategy, B: Strategy;
}


pub trait Creature<A>: Ord {
    fn new(data: &A) -> Self;

    fn breed(&self, other: &Self, data: &A) -> Self;
}


pub trait Gene {
    fn new() -> Self;

    fn choose(&self, other: &Self) -> Self;
}


#[derive(Debug, Clone)]
enum NumericCalculator<A, B> where A: Calculate<B> {
    Base(A),
    Fixed(B),
    Percentage(Percentage, Box<NumericCalculator<A, B>>),
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
    IfThenElse(Box<BooleanCalculator<A>>, Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
}

impl<A> NumericCalculator<A, f64>
    where A: Calculate<f64> + Gene + Clone {
    fn _binary<B, C>(left: Self, right: Self, make: B, reduce: C) -> Self
        where B: FnOnce(Box<Self>, Box<Self>) -> Self,
              C: FnOnce(f64, f64) -> f64 {
        match left {
            NumericCalculator::Fixed(a) => match right {
                NumericCalculator::Fixed(b) => NumericCalculator::Fixed(reduce(a, b)),
                _ => make(Box::new(left), Box::new(right)),
            },
            _ => make(Box::new(left), Box::new(right)),
        }
    }

    fn _percentage(left: Self, percentage: Percentage) -> Self {
        match left {
            NumericCalculator::Fixed(a) => NumericCalculator::Fixed(a * percentage.unwrap()),
            NumericCalculator::Percentage(a, b) => NumericCalculator::Percentage(Percentage(a.unwrap() * percentage.unwrap()), b),
            _ => NumericCalculator::Percentage(percentage, Box::new(left)),
        }
    }

    // TODO what about nested Bezier ?
    fn _bezier(left: Self, bezier: CubicBezierSegment) -> Self {
        match left {
            // TODO f64 version of Bezier curves
            NumericCalculator::Fixed(a) => NumericCalculator::Fixed(bezier.sample_y(a as f32) as f64),
            _ => NumericCalculator::Bezier(bezier, Box::new(left)),
        }
    }

    fn _abs(left: Self) -> Self {
        match left {
            NumericCalculator::Fixed(a) => NumericCalculator::Fixed(a.abs()),
            NumericCalculator::Abs(_) => left,
            _ => NumericCalculator::Abs(Box::new(left)),
        }
    }

    fn _if_then_else(test: BooleanCalculator<A>, yes: Self, no: Self) -> Self {
        match test {
            BooleanCalculator::True => yes,
            BooleanCalculator::False => no,
            _ => NumericCalculator::IfThenElse(Box::new(test), Box::new(yes), Box::new(no))
        }
    }

    // TODO auto-derive this
    fn _new(depth: u32) -> Self {
        if depth >= MAX_RECURSION_DEPTH {
            let rand = gen_rand_index(2u32);

            if rand == 0 {
                NumericCalculator::Base(Gene::new())

            } else {
                NumericCalculator::Fixed(Gene::new())
            }

        } else {
            let rand = gen_rand_index(13u32);

            if rand == 0 {
                NumericCalculator::Base(Gene::new())

            } else if rand == 1 {
                NumericCalculator::Fixed(Gene::new())

            } else if rand == 2 {
                Self::_percentage(Self::_new(depth + 1), Gene::new())

            } else if rand == 3 {
                Self::_bezier(Self::_new(depth + 1), Gene::new())

            } else if rand == 4 {
                Self::_abs(Self::_new(depth + 1))

            } else if rand == 5 {
                Self::_binary(Self::_new(depth + 1), Self::_new(depth + 1), NumericCalculator::Average, |a, b| (a + b) / 2.0)

            } else if rand == 6 {
                Self::_binary(Self::_new(depth + 1), Self::_new(depth + 1), NumericCalculator::Min, |a, b| a.min(b))

            } else if rand == 7 {
                Self::_binary(Self::_new(depth + 1), Self::_new(depth + 1), NumericCalculator::Max, |a, b| a.max(b))

            } else if rand == 8 {
                Self::_binary(Self::_new(depth + 1), Self::_new(depth + 1), NumericCalculator::Plus, |a, b| a + b)

            } else if rand == 9 {
                Self::_binary(Self::_new(depth + 1), Self::_new(depth + 1), NumericCalculator::Minus, |a, b| a - b)

            } else if rand == 10 {
                Self::_binary(Self::_new(depth + 1), Self::_new(depth + 1), NumericCalculator::Multiply, |a, b| a * b)

            } else if rand == 11 {
                Self::_binary(Self::_new(depth + 1), Self::_new(depth + 1), NumericCalculator::Divide, |a, b| a / b)

            } else {
                Self::_if_then_else(Gene::new(), Self::_new(depth + 1), Self::_new(depth + 1))
            }
        }
    }

    // TODO verify that this cannot exceed MAX_RECURSION_DEPTH
    // TODO auto-derive this
    fn _choose(&self, other: &Self, depth: u32) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Self::_new(depth)

        } else {
            match *self {
                NumericCalculator::Base(ref father) => match *other {
                    NumericCalculator::Base(ref mother) => NumericCalculator::Base(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Fixed(ref father) => match *other {
                    NumericCalculator::Fixed(ref mother) => NumericCalculator::Fixed(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Percentage(father1, ref father2) => match *other {
                    NumericCalculator::Percentage(mother1, ref mother2) => Self::_percentage(father2._choose(&mother2, depth + 1), father1.choose(&mother1)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Bezier(father1, ref father2) => match *other {
                    NumericCalculator::Bezier(mother1, ref mother2) => Self::_bezier(father2._choose(&mother2, depth + 1), father1.choose(&mother1)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Abs(ref father) => match *other {
                    NumericCalculator::Abs(ref mother) => Self::_abs(father._choose(&mother, depth + 1)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Average(ref father1, ref father2) => match *other {
                    NumericCalculator::Average(ref mother1, ref mother2) => Self::_binary(father1._choose(&mother1, depth + 1), father2._choose(&mother2, depth + 1), NumericCalculator::Average, |a, b| (a + b) / 2.0),
                    _ => choose2(self, other),
                },
                NumericCalculator::Min(ref father1, ref father2) => match *other {
                    NumericCalculator::Min(ref mother1, ref mother2) => Self::_binary(father1._choose(&mother1, depth + 1), father2._choose(&mother2, depth + 1), NumericCalculator::Min, |a, b| a.min(b)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Max(ref father1, ref father2) => match *other {
                    NumericCalculator::Max(ref mother1, ref mother2) => Self::_binary(father1._choose(&mother1, depth + 1), father2._choose(&mother2, depth + 1), NumericCalculator::Max, |a, b| a.max(b)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Plus(ref father1, ref father2) => match *other {
                    NumericCalculator::Plus(ref mother1, ref mother2) => Self::_binary(father1._choose(&mother1, depth + 1), father2._choose(&mother2, depth + 1), NumericCalculator::Plus, |a, b| a + b),
                    _ => choose2(self, other),
                },
                NumericCalculator::Minus(ref father1, ref father2) => match *other {
                    NumericCalculator::Minus(ref mother1, ref mother2) => Self::_binary(father1._choose(&mother1, depth + 1), father2._choose(&mother2, depth + 1), NumericCalculator::Minus, |a, b| a - b),
                    _ => choose2(self, other),
                },
                NumericCalculator::Multiply(ref father1, ref father2) => match *other {
                    NumericCalculator::Multiply(ref mother1, ref mother2) => Self::_binary(father1._choose(&mother1, depth + 1), father2._choose(&mother2, depth + 1), NumericCalculator::Multiply, |a, b| a * b),
                    _ => choose2(self, other),
                },
                NumericCalculator::Divide(ref father1, ref father2) => match *other {
                    NumericCalculator::Divide(ref mother1, ref mother2) => Self::_binary(father1._choose(&mother1, depth + 1), father2._choose(&mother2, depth + 1), NumericCalculator::Divide, |a, b| a / b),
                    _ => choose2(self, other),
                },
                NumericCalculator::IfThenElse(ref father1, ref father2, ref father3) => match *other {
                    // TODO should this pass the depth somehow to father1 and mother1 ?
                    NumericCalculator::IfThenElse(ref mother1, ref mother2, ref mother3) => Self::_if_then_else(father1.choose(&mother1), father2._choose(&mother2, depth + 1), father3._choose(&mother3, depth + 1)),
                    _ => choose2(self, other),
                },
            }
        }
    }
}

impl<A> Calculate<f64> for NumericCalculator<A, f64>
    where A: Calculate<f64> {
    fn calculate<'a, 'b, 'c, C, D>(&self, simulation: &Simulation<'a, 'b, 'c, C, D>, left: &'c str, right: &'c str) -> f64
        where C: Strategy,
              D: Strategy {
        match *self {
            NumericCalculator::Base(ref a) => a.calculate(simulation, left, right),

            NumericCalculator::Fixed(a) => a,

            NumericCalculator::Percentage(Percentage(percentage), ref a) => a.calculate(simulation, left, right) * percentage,

            // TODO f64 version of Bezier curves
            NumericCalculator::Bezier(bezier, ref a) => bezier.sample_y(a.calculate(simulation, left, right) as f32) as f64,

            NumericCalculator::Average(ref a, ref b) => (a.calculate(simulation, left, right) + b.calculate(simulation, left, right)) / 2.0,
            NumericCalculator::Abs(ref a) => a.calculate(simulation, left, right).abs(),
            NumericCalculator::Min(ref a, ref b) => a.calculate(simulation, left, right).min(b.calculate(simulation, left, right)),
            NumericCalculator::Max(ref a, ref b) => a.calculate(simulation, left, right).max(b.calculate(simulation, left, right)),
            NumericCalculator::Plus(ref a, ref b) => a.calculate(simulation, left, right) + b.calculate(simulation, left, right),
            NumericCalculator::Minus(ref a, ref b) => a.calculate(simulation, left, right) - b.calculate(simulation, left, right),
            NumericCalculator::Multiply(ref a, ref b) => a.calculate(simulation, left, right) * b.calculate(simulation, left, right),
            NumericCalculator::Divide(ref a, ref b) => a.calculate(simulation, left, right) / b.calculate(simulation, left, right),

            NumericCalculator::IfThenElse(ref a, ref b, ref c) => if a.calculate(simulation, left, right) {
                b.calculate(simulation, left, right)
            } else {
                c.calculate(simulation, left, right)
            }
        }
    }
}

impl<A> Gene for NumericCalculator<A, f64>
    where A: Calculate<f64> + Gene + Clone {
    fn new() -> Self {
        NumericCalculator::_new(0)
    }

    fn choose(&self, other: &Self) -> Self {
        self._choose(other, 0)
    }
}


#[derive(Debug, Clone)]
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

impl<A> BooleanCalculator<A> where A: Gene, A: Clone {
    fn _and(self, other: Self) -> Self {
        match self {
            BooleanCalculator::True => other,
            BooleanCalculator::False => self,
            _ => match other {
                BooleanCalculator::True => self,
                BooleanCalculator::False => other,
                _ => BooleanCalculator::And(Box::new(self), Box::new(other)),
            }
        }
    }

    fn _or(self, other: Self) -> Self {
        match self {
            BooleanCalculator::True => self,
            BooleanCalculator::False => other,
            _ => match other {
                BooleanCalculator::True => other,
                BooleanCalculator::False => self,
                _ => BooleanCalculator::Or(Box::new(self), Box::new(other)),
            }
        }
    }

    fn _new(depth: u32) -> Self {
        if depth >= MAX_RECURSION_DEPTH {
            let rand = gen_rand_index(6u32);

            if rand == 0 {
                BooleanCalculator::True

            } else if rand == 1 {
                BooleanCalculator::False

            } else if rand == 2 {
                BooleanCalculator::Greater(Gene::new(), Gene::new())

            } else if rand == 3 {
                BooleanCalculator::GreaterEqual(Gene::new(), Gene::new())

            } else if rand == 4 {
                BooleanCalculator::Lesser(Gene::new(), Gene::new())

            } else {
                BooleanCalculator::LesserEqual(Gene::new(), Gene::new())
            }

        } else {
            let rand = gen_rand_index(8u32);

            if rand == 0 {
                BooleanCalculator::True

            } else if rand == 1 {
                BooleanCalculator::False

            } else if rand == 2 {
                BooleanCalculator::Greater(Gene::new(), Gene::new())

            } else if rand == 3 {
                BooleanCalculator::GreaterEqual(Gene::new(), Gene::new())

            } else if rand == 4 {
                BooleanCalculator::Lesser(Gene::new(), Gene::new())

            } else if rand == 5 {
                BooleanCalculator::LesserEqual(Gene::new(), Gene::new())

            } else if rand == 6 {
                Self::_new(depth + 1)._and(Self::_new(depth + 1))

            } else {
                Self::_new(depth + 1)._or(Self::_new(depth + 1))
            }
        }
    }

    // TODO is this correct ?
    fn _choose(&self, other: &Self, depth: u32) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Self::_new(depth)

        } else {
            match *self {
                BooleanCalculator::Greater(ref father1, ref father2) => match *other {
                    BooleanCalculator::Greater(ref mother1, ref mother2) => BooleanCalculator::Greater(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::GreaterEqual(ref father1, ref father2) => match *other {
                    BooleanCalculator::GreaterEqual(ref mother1, ref mother2) => BooleanCalculator::GreaterEqual(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::Lesser(ref father1, ref father2) => match *other {
                    BooleanCalculator::Lesser(ref mother1, ref mother2) => BooleanCalculator::Lesser(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::LesserEqual(ref father1, ref father2) => match *other {
                    BooleanCalculator::LesserEqual(ref mother1, ref mother2) => BooleanCalculator::LesserEqual(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::And(ref father1, ref father2) => match *other {
                    BooleanCalculator::And(ref mother1, ref mother2) => father1._choose(&mother1, depth + 1)._and(father2._choose(&mother2, depth + 1)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::Or(ref father1, ref father2) => match *other {
                    BooleanCalculator::Or(ref mother1, ref mother2) => father1._choose(&mother1, depth + 1)._or(father2._choose(&mother2, depth + 1)),
                    _ => choose2(self, other),
                },
                _ => choose2(self, other),
            }
        }
    }
}

impl<A> Calculate<bool> for BooleanCalculator<A>
    where A: Calculate<f64> {
    fn calculate<'a, 'b, 'c, C, D>(&self, simulation: &Simulation<'a, 'b, 'c, C, D>, left: &'c str, right: &'c str) -> bool
        where C: Strategy,
              D: Strategy {
        match *self {
            BooleanCalculator::True => true,
            BooleanCalculator::False => false,
            BooleanCalculator::Greater(ref a, ref b) => a.calculate(simulation, left, right) > b.calculate(simulation, left, right),
            BooleanCalculator::GreaterEqual(ref a, ref b) => a.calculate(simulation, left, right) >= b.calculate(simulation, left, right),
            BooleanCalculator::Lesser(ref a, ref b) => a.calculate(simulation, left, right) < b.calculate(simulation, left, right),
            BooleanCalculator::LesserEqual(ref a, ref b) => a.calculate(simulation, left, right) <= b.calculate(simulation, left, right),
            BooleanCalculator::And(ref a, ref b) => a.calculate(simulation, left, right) && b.calculate(simulation, left, right),
            BooleanCalculator::Or(ref a, ref b) => a.calculate(simulation, left, right) || b.calculate(simulation, left, right),
        }
    }
}

impl<A> Gene for BooleanCalculator<A> where A: Gene, A: Clone {
    fn new() -> Self {
        Self::_new(0)
    }

    fn choose(&self, other: &Self) -> Self {
        self._choose(other, 0)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct BetAmount(f64);

impl BetAmount {
    fn unwrap(&self) -> f64 {
        let BetAmount(value) = *self;
        value
    }
}

impl Gene for BetAmount {
    fn new() -> Self {
        let Percentage(percent) = Gene::new();

        BetAmount(MAX_BET_AMOUNT * percent)
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


impl Gene for bool {
    fn new() -> Self {
        rand::weak_rng().gen::<bool>()
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


impl<A> Gene for Option<A> where A: Gene, A: Clone {
    fn new() -> Self {
        if Gene::new() {
            None

        } else {
            Some(A::new())
        }
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            match *self {
                Some(ref father) => match *other {
                    Some(ref mother) => Some(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                _ => choose2(self, other),
            }
        }
    }
}


impl Gene for Percentage {
    fn new() -> Self {
        let Closed01(val) = rand::weak_rng().gen::<Closed01<f64>>();
        Percentage(val)
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


impl Gene for f64 {
    // TODO verify that this is correct
    fn new() -> Self {
        let Percentage(percent) = Gene::new();

        MAX_BET_AMOUNT * ((percent * 2.0) - 1.0)
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


impl Gene for f32 {
    // TODO verify that this is correct
    fn new() -> Self {
        let Closed01(val) = rand::weak_rng().gen::<Closed01<f32>>();
        val
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


impl Gene for Point {
    fn new() -> Self {
        Self::new(Gene::new(), Gene::new())
    }

    fn choose(&self, other: &Self) -> Self {
        Self::new(self.x.choose(&other.x), self.y.choose(&other.y))
    }
}


impl Gene for CubicBezierSegment {
    fn new() -> Self {
        CubicBezierSegment {
            from: Gene::new(),
            ctrl1: Gene::new(),
            ctrl2: Gene::new(),
            to: Gene::new(),
        }
    }

    fn choose(&self, other: &Self) -> Self {
        CubicBezierSegment {
            from: self.from.choose(&other.from),
            ctrl1: self.ctrl1.choose(&other.ctrl1),
            ctrl2: self.ctrl2.choose(&other.ctrl2),
            to: self.to.choose(&other.to),
        }
    }
}


fn choose2<A>(left: &A, right: &A) -> A where A: Clone {
    if Gene::new() {
        left.clone()

    } else {
        right.clone()
    }
}


fn gen_rand_index(index: u32) -> u32 {
    rand::weak_rng().gen_range(0, index)
}


// TODO verify that this is correct
fn rand_is_percent(Percentage(input): Percentage) -> bool {
    let Percentage(rand) = Gene::new();
    rand <= input
}


fn choose<'a, A>(values: &'a [A]) -> Option<&'a A> {
    rand::weak_rng().choose(values)
}


#[derive(Debug, Clone)]
enum LookupStatistic {
    Upsets,
    Favored,
    Winrate,
    Odds,
    Earnings,
    MatchesLen,
}

impl LookupStatistic {
    fn iterate_percentage<'a, A, B, C>(iter: A, default: B, matches: C) -> f64
        where A: Iterator<Item = &'a Record>,
              B: FnOnce() -> f64,
              C: Fn(&'a Record) -> bool {
        let mut output: f64 = 0.0;

        let mut len: f64 = 0.0;

        for record in iter {
            len += 1.0;

            if matches(record) {
                output += 1.0;
            }
        }

        if len == 0.0 {
            default()

        } else {
            output / len
        }
    }

    fn upsets<'a, A>(iter: A, name: &'a str) -> f64
        where A: Iterator<Item = &'a Record> {
        // TODO is 0.0 or 0.5 better ?
        LookupStatistic::iterate_percentage(iter, || 0.0, |record|
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            (record.left.name == name &&
             (record.right.bet_amount / record.left.bet_amount) > 1.0) ||

            (record.right.name == name &&
             (record.left.bet_amount / record.right.bet_amount) > 1.0))
    }

    fn favored<'a, A>(iter: A, name: &'a str) -> f64
        where A: Iterator<Item = &'a Record> {
        // TODO is 0.0 or 0.5 better ?
        LookupStatistic::iterate_percentage(iter, || 0.0, |record|
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            (record.left.name == name &&
             (record.left.bet_amount / record.right.bet_amount) > 1.0) ||

            (record.right.name == name &&
             (record.right.bet_amount / record.left.bet_amount) > 1.0))
    }

    fn winrate<'a, A>(iter: A, name: &'a str) -> f64
        where A: Iterator<Item = &'a Record> {
        // TODO what about mirror matches ?
        LookupStatistic::iterate_percentage(iter, || 0.5, |record| record.is_winner(name))
    }

    fn odds<'a, A>(iter: A, name: &'a str) -> f64
        where A: Iterator<Item = &'a Record> {
        let mut len: f64 = 0.0;

        let mut odds: f64 = 0.0;

        for record in iter {
            len += 1.0;

            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            if record.left.name == name {
                odds += record.right.bet_amount / record.left.bet_amount;

            } else {
                odds += record.left.bet_amount / record.right.bet_amount;
            }
        }

        if len == 0.0 {
            // TODO is this correct ?
            0.0

        } else {
            odds / len
        }
    }

    fn earnings<'a, A>(iter: A, name: &'a str) -> f64
        where A: Iterator<Item = &'a Record> {
        let mut earnings: f64 = 0.0;

        for record in iter {
            match record.winner {
                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                Winner::Left => if record.left.name == name {
                    earnings += record.right.bet_amount / record.left.bet_amount;

                } else {
                    earnings -= 1.0;
                },

                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                Winner::Right => if record.right.name == name {
                    earnings += record.left.bet_amount / record.right.bet_amount;

                } else {
                    earnings -= 1.0;
                },

                Winner::None => {}
            }
        }

        earnings
    }

    fn matches_len<'a, A>(iter: A) -> f64
        where A: Iterator<Item = &'a Record> {
        let mut len: f64 = 0.0;

        for _ in iter {
            len += 1.0;
        }

        // TODO what if the len is longer than MAX_VEC_LEN ?
        len / MAX_VEC_LEN
    }

    fn lookup<'a, A>(&self, name: &'a str, iter: A) -> f64
        where A: Iterator<Item = &'a Record> {
        match *self {
            LookupStatistic::Upsets => LookupStatistic::upsets(iter, name),
            LookupStatistic::Favored => LookupStatistic::favored(iter, name),
            LookupStatistic::Winrate => LookupStatistic::winrate(iter, name),
            LookupStatistic::Earnings => LookupStatistic::earnings(iter, name),
            LookupStatistic::Odds => LookupStatistic::odds(iter, name),
            LookupStatistic::MatchesLen => LookupStatistic::matches_len(iter),
        }
    }
}

impl Gene for LookupStatistic {
    fn new() -> Self {
        let rand = gen_rand_index(6u32);

        if rand == 0 {
            LookupStatistic::Upsets

        } else if rand == 1 {
            LookupStatistic::Favored

        } else if rand == 2 {
            LookupStatistic::Winrate

        } else if rand == 3 {
            LookupStatistic::Odds

        } else if rand == 4 {
            LookupStatistic::Earnings

        } else {
            LookupStatistic::MatchesLen
        }
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


#[derive(Debug, Clone)]
enum LookupFilter {
    All,
    Specific,
}

impl LookupFilter {
    fn lookup<'a>(&self, stat: &LookupStatistic, left: &'a str, right: &'a str, matches: &Vec<&'a Record>) -> f64 {
        match *self {
            LookupFilter::All => stat.lookup(left, matches.into_iter().map(|x| *x)),

            LookupFilter::Specific => stat.lookup(left, matches.into_iter().map(|x| *x).filter(|record|
                (record.left.name == right) ||
                (record.right.name == right))),
        }
    }
}

impl Gene for LookupFilter {
    fn new() -> Self {
        let rand = gen_rand_index(2u32);

        if rand == 0 {
            LookupFilter::All

        } else {
            LookupFilter::Specific
        }
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


#[derive(Debug, Clone)]
enum LookupSide {
    Left,
    Right
}

impl Gene for LookupSide {
    fn new() -> Self {
        let rand = gen_rand_index(2u32);

        if rand == 0 {
            LookupSide::Left

        } else {
            LookupSide::Right
        }
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}


#[derive(Debug, Clone)]
enum Lookup {
    Sum,
    Character(LookupSide, LookupFilter, LookupStatistic),
}

impl Calculate<f64> for Lookup {
    fn calculate<'a, 'b, 'c, A, B>(&self, simulation: &Simulation<'a, 'b, 'c, A, B>, left: &'c str, right: &'c str) -> f64
        where A: Strategy,
              B: Strategy {
        match *self {
            Lookup::Sum => simulation.sum(),

            Lookup::Character(ref side, ref filter, ref stat) => match *side {
                LookupSide::Left =>
                    filter.lookup(stat, left, right, simulation.characters.get(left).unwrap_or(&vec![])),

                LookupSide::Right =>
                    filter.lookup(stat, right, left, simulation.characters.get(right).unwrap_or(&vec![])),
            },
        }
    }
}

impl Gene for Lookup {
    fn new() -> Self {
        let rand = gen_rand_index(2u32);

        if rand == 0 {
            Lookup::Sum

        } else {
            Lookup::Character(Gene::new(), Gene::new(), Gene::new())
        }
    }

    // TODO is this correct ?
    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            match *self {
                Lookup::Character(ref father1, ref father2, ref father3) => match *other {
                    Lookup::Character(ref mother1, ref mother2, ref mother3) => Lookup::Character(father1.choose(&mother1), father2.choose(&mother2), father3.choose(&mother3)),
                    _ => choose2(self, other),
                },
                _ => choose2(self, other),
            }
        }
    }
}


#[derive(Debug)]
pub struct SimulationSettings<'a> {
    pub records: &'a Vec<Record>,
    pub mode: Mode,
}


#[derive(Debug)]
struct Simulation<'a, 'b, 'c, A, B> where A: Strategy, A: 'a, B: Strategy, B: 'b {
    matchmaking_strategy: Option<&'a A>,
    tournament_strategy: Option<&'b B>,
    sum: f64,
    tournament_sum: f64,
    in_tournament: bool,
    successes: f64,
    failures: f64,
    max_character_len: usize,
    characters: HashMap<&'c str, Vec<&'c Record>>,
}

impl<'a, 'b, 'c, A, B> Simulation<'a, 'b, 'c, A, B> where A: Strategy, B: Strategy {
    pub fn new() -> Self {
        Self {
            matchmaking_strategy: None,
            tournament_strategy: None,
            sum: SALT_MINE_AMOUNT,
            tournament_sum: TOURNAMENT_BALANCE,
            in_tournament: false,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            characters: HashMap::new()
        }
    }

    fn insert_match(&mut self, key: &'c str, record: &'c Record) {
        let matches = self.characters.entry(key).or_insert_with(|| vec![]);

        matches.push(record);

        let len = matches.len();

        if len > self.max_character_len {
            self.max_character_len = len;
        }
    }

    fn insert_record(&mut self, record: &'c Record) {
        if record.left.name != record.right.name {
            self.insert_match(&record.left.name, record);
            self.insert_match(&record.right.name, record);
        }
    }

    pub fn sum(&self) -> f64 {
        if self.in_tournament {
            self.tournament_sum

        } else {
            self.sum
        }
    }

    fn is_in_mines(&self) -> bool {
        if self.in_tournament {
            self.tournament_sum <= TOURNAMENT_BALANCE

        } else {
            self.sum <= SALT_MINE_AMOUNT
        }
    }

    fn clamp(&self, bet_amount: f64) -> f64 {
        let sum = self.sum();

        if self.is_in_mines() {
            sum

        } else {
            let rounded = bet_amount.round();

            if rounded < 1.0 {
                1.0

            } else if rounded > sum {
                sum

            } else {
                rounded
            }
        }
    }

    fn pick_winner<C>(&self, strategy: &C, record: &'c Record) -> Bet where C: Strategy {
        let bet = if record.left.name == record.right.name {
            Bet::None

        } else {
            strategy.bet(self, &record.left.name, &record.right.name)
        };

        match bet {
            Bet::Left(bet_amount) => Bet::Left(self.clamp(bet_amount)),

            Bet::Right(bet_amount) => Bet::Right(self.clamp(bet_amount)),

            Bet::None => if self.is_in_mines() {
                if Gene::new() {
                    Bet::Left(self.sum())

                } else {
                    Bet::Right(self.sum())
                }

            } else {
                Bet::None
            },
        }
    }

    fn calculate(&mut self, record: &'c Record) {
        // TODO make this more efficient
        let record = record.clone().shuffle();

        let winner = match record.mode {
            Mode::Matchmaking => {
                if self.in_tournament {
                    self.in_tournament = false;
                    //println!("rollover: {}", self.tournament_sum);
                    self.sum += self.tournament_sum;
                    self.tournament_sum = TOURNAMENT_BALANCE;
                }

                match self.matchmaking_strategy {
                    Some(a) => self.pick_winner(a, &record),
                    None => return,
                }
            },
            Mode::Tournament => {
                self.in_tournament = true;

                match self.tournament_strategy {
                    Some(a) => self.pick_winner(a, &record),
                    None => return,
                }
            },
        };

        //println!("tournament: {}", self.in_tournament);

        //println!("sum: {}", self.sum());

        let increase = match winner {
            Bet::Left(bet_amount) => match record.winner {
                Winner::Left => {
                    let odds = record.right.bet_amount / record.left.bet_amount;

                    if odds > 1.0 && (bet_amount * odds).ceil() > 100000000.0 {
                        //println!("BIG odds: {}", odds);
                    } else {
                        //println!("odds: {}", odds);
                    }

                    self.successes += 1.0;
                    (bet_amount * odds).ceil()
                },

                Winner::Right => {
                    self.failures += 1.0;
                    -bet_amount
                },

                Winner::None => 0.0,
            },

            Bet::Right(bet_amount) => match record.winner {
                Winner::Right => {
                    let odds = record.left.bet_amount / record.right.bet_amount;

                    if odds > 1.0 && (bet_amount * odds).ceil() > 100000000.0 {
                        //println!("BIG odds: {}", odds);
                    } else {
                        //println!("odds: {}", odds);
                    }

                    self.successes += 1.0;
                    (bet_amount * odds).ceil()
                },

                Winner::Left => {
                    self.failures += 1.0;
                    -bet_amount
                },

                Winner::None => 0.0,
            },

            Bet::None => 0.0,
        };

        //println!("increase: {}", increase);

        if self.in_tournament {
            self.tournament_sum += increase;

            if self.tournament_sum <= 0.0 {
                self.tournament_sum = TOURNAMENT_BALANCE;
            }

        } else {
            self.sum += increase;

            if self.sum <= 0.0 {
                self.sum = SALT_MINE_AMOUNT;
            }
        }

        //println!("sum: {}", self.sum());
        //println!("----------------------");
    }

    pub fn simulate(&mut self, records: &'c Vec<Record>) {
        // shuffle(records);

        for record in records.iter() {
            self.calculate(record);
            self.insert_record(record);
        }
    }

    pub fn insert_records(&mut self, records: &'c Vec<Record>) {
        // shuffle(records);

        for record in records.iter() {
            self.insert_record(record);
        }
    }
}



#[derive(Debug)]
pub struct BetStrategy {
    pub fitness: f64,
    successes: f64,
    failures: f64,
    max_character_len: usize,

    // Genes
    bet_strategy: BooleanCalculator<NumericCalculator<Lookup, f64>>,
    prediction_strategy: NumericCalculator<Lookup, f64>,
    money_strategy: NumericCalculator<Lookup, f64>,
}

impl<'a> BetStrategy {
    /*fn calculate_prediction(simulation: &Simulation, record: &Record) -> Winner {

    }*/

    fn calculate_fitness(mut self, settings: &SimulationSettings<'a>) -> Self {
        let (sum, successes, failures, max_character_len) = {
            let mut simulation = Simulation::new();

            match settings.mode {
                Mode::Matchmaking => simulation.matchmaking_strategy = Some(&self),
                Mode::Tournament => simulation.tournament_strategy = Some(&self),
            }

            simulation.simulate(settings.records);

            (
                simulation.sum,
                simulation.successes,
                simulation.failures,
                simulation.max_character_len
            )
        };

        self.fitness = sum;
        self.successes = successes;
        self.failures = failures;
        self.max_character_len = max_character_len;

        self
    }
}

impl<'a> Creature<SimulationSettings<'a>> for BetStrategy {
    fn new(settings: &SimulationSettings<'a>) -> Self {
        Self {
            fitness: 0.0,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            bet_strategy: Gene::new(),
            prediction_strategy: Gene::new(),
            money_strategy: Gene::new(),
        }.calculate_fitness(settings)
    }

    fn breed(&self, other: &Self, settings: &SimulationSettings<'a>) -> Self {
        Self {
            fitness: 0.0,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            bet_strategy: self.bet_strategy.choose(&other.bet_strategy),
            prediction_strategy: self.prediction_strategy.choose(&other.prediction_strategy),
            money_strategy: self.money_strategy.choose(&other.money_strategy),
        }.calculate_fitness(settings)
    }
}

impl Strategy for BetStrategy {
    fn bet<A, B>(&self, simulation: &Simulation<A, B>, left: &str, right: &str) -> Bet where A: Strategy, B: Strategy {
        if self.bet_strategy.calculate(simulation, left, right) {
            let p_left = self.prediction_strategy.calculate(simulation, left, right);
            let p_right = self.prediction_strategy.calculate(simulation, right, left);

            if p_left > p_right {
                Bet::Left(self.money_strategy.calculate(simulation, left, right))

            } else if p_right > p_left {
                Bet::Right(self.money_strategy.calculate(simulation, right, left))

            } else {
                Bet::None
            }

        } else {
            Bet::None
        }
    }
}

impl Ord for BetStrategy {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.fitness.is_nan() {
            if other.fitness.is_nan() {
                std::cmp::Ordering::Equal

            } else {
                std::cmp::Ordering::Less
            }

        } else if other.fitness.is_nan() {
            std::cmp::Ordering::Greater

        } else {
            self.partial_cmp(other).unwrap()
        }
    }
}

impl PartialOrd for BetStrategy {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.fitness.partial_cmp(&other.fitness)
    }
}

impl PartialEq for BetStrategy {
    fn eq(&self, other: &Self) -> bool {
        self.fitness == other.fitness
    }
}

impl Eq for BetStrategy {}



#[derive(Debug)]
pub struct Population<'a, A, B> where A: Creature<B>, B: 'a {
    data: &'a B,
    amount: usize,
    pub populace: Vec<A>,
}

impl<'a, A, B> Population<'a, A, B> where A: Creature<B> + Send + Sync, B: 'a + Sync {
    pub fn new(amount: usize, data: &'a B) -> Self {
        Self {
            data: data,
            amount: amount,
            populace: Vec::with_capacity(amount),
        }
    }

    fn insert_creature(&mut self, creature: A) {
        let index = self.populace.binary_search(&creature);

        match index {
            Ok(index) => self.populace.insert(index, creature),
            Err(index) => self.populace.insert(index, creature),
        }
    }

    fn kill_populace(&mut self) {
        let mut index: f64 = 0.0;

        let len = (self.populace.len() - 1) as f64;

        // TODO is it guaranteed that retain operates from left-to-right ?
        self.populace.retain(|_| {
            let keep = rand_is_percent(Percentage(index / len));
            index += 1.0;
            keep
        });
    }

    fn breed_populace(&mut self) {
        let new_creatures: Vec<A> = {
            let closure = |_| {
                let father = choose(&self.populace);
                let mother = choose(&self.populace);

                match father {
                    Some(father) => match mother {
                        Some(mother) => father.breed(mother, self.data),
                        None => A::new(self.data),
                    },
                    None => A::new(self.data),
                }
            };

            if cfg!(target_arch = "wasm32") {
                (self.populace.len()..self.amount).map(closure).collect()

            } else {
                (self.populace.len()..self.amount).into_par_iter().map(closure).collect()
            }
        };

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn best(&self) -> &A {
        self.populace.last().unwrap()
    }

    pub fn init(&mut self) {
        // TODO code duplication
        let new_creatures: Vec<A> = if cfg!(target_arch = "wasm32") {
            (0..self.amount).map(|_| A::new(self.data)).collect()

        } else {
            (0..self.amount).into_par_iter().map(|_|A::new(self.data)).collect()
        };

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn next_generation(&mut self) {
        self.kill_populace();
        self.breed_populace();
    }
}
