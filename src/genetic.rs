use std;
use std::collections::{ HashMap };
use lyon_bezier::Point;
use lyon_bezier::cubic_bezier::CubicBezierSegment;
use rand;
use rand::{ Rng, Closed01 };
use rand::distributions::{ Range, IndependentSample };
use rand::distributions::normal::{ StandardNormal };
use record::{ Record, Mode, Winner };
use record::Winner::{ Left, Right };
//use rayon::iter::{ IntoParallelRefIterator, ParallelIterator, FromParallelIterator, IntoParallelIterator };
use rayon::prelude::*;


const MAX_VEC_LEN: f32 = 10000.0;
const MAX_BET_AMOUNT: f64 = 1000000.0;
const MUTATION_RATE: Percentage = Percentage(0.10);
const MAX_RECURSION: u32 = 16;

const SALT_MINE_AMOUNT: f64 = 258.0; // TODO verify that this is correct
const TOURNAMENT_BALANCE: f64 = 1375.0; // TODO


trait Diff {
    fn diff(&self, other: &Self) -> Self;
}


impl Diff for f64 {
    fn diff(&self, other: &Self) -> Self {
        (self - other).abs()
    }
}


#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
struct Percentage(f64);

impl Percentage {
    fn unwrap(&self) -> f64 {
        let Percentage(value) = *self;
        value
    }
}

impl Diff for Percentage {
    fn diff(&self, other: &Self) -> Self {
        Percentage(self.unwrap().diff(&other.unwrap()))
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


trait Gene {
    fn new() -> Self;

    fn choose(&self, other: &Self) -> Self;
}


#[derive(Debug, Clone)]
enum MoneyStrategy {
    AllIn,
    Fixed(BetAmount),
    Percentage(Percentage, Box<MoneyStrategy>),
    Confidence(CubicBezierSegment, Box<MoneyStrategy>),
    Plus(Box<MoneyStrategy>, Box<MoneyStrategy>),
    Minus(Box<MoneyStrategy>, Box<MoneyStrategy>),
    Multiply(Box<MoneyStrategy>, Box<MoneyStrategy>),
    Divide(Box<MoneyStrategy>, Box<MoneyStrategy>),
}

impl MoneyStrategy {
    // TODO auto-derive this
    fn _new(depth: u32) -> Self {
        if depth >= MAX_RECURSION {
            let rand = gen_rand_index(2u32);

            if rand == 0 {
                MoneyStrategy::AllIn

            } else {
                MoneyStrategy::Fixed(Gene::new())
            }

        } else {
            let rand = gen_rand_index(8u32);

            if rand == 0 {
                MoneyStrategy::AllIn

            } else if rand == 1 {
                MoneyStrategy::Fixed(Gene::new())

            } else if rand == 2 {
                MoneyStrategy::Percentage(Gene::new(), Box::new(Self::_new(depth + 1)))

            } else if rand == 3 {
                MoneyStrategy::Confidence(Gene::new(), Box::new(Self::_new(depth + 1)))

            } else if rand == 4 {
                MoneyStrategy::Plus(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 5 {
                MoneyStrategy::Minus(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 6 {
                MoneyStrategy::Multiply(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else {
                MoneyStrategy::Divide(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))
            }
        }
    }

    // TODO verify that this cannot exceed MAX_RECURSION
    // TODO auto-derive this
    fn _choose(&self, other: &Self, depth: u32) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Self::_new(depth)

        } else {
            match *self {
                MoneyStrategy::AllIn => choose2(self, other),
                MoneyStrategy::Fixed(ref father) => match *other {
                    MoneyStrategy::Fixed(ref mother) => MoneyStrategy::Fixed(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Percentage(father1, ref father2) => match *other {
                    MoneyStrategy::Percentage(mother1, ref mother2) => MoneyStrategy::Percentage(father1.choose(&mother1), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Confidence(father1, ref father2) => match *other {
                    MoneyStrategy::Confidence(mother1, ref mother2) => MoneyStrategy::Confidence(father1.choose(&mother1), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Plus(ref father1, ref father2) => match *other {
                    MoneyStrategy::Plus(ref mother1, ref mother2) => MoneyStrategy::Plus(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Minus(ref father1, ref father2) => match *other {
                    MoneyStrategy::Minus(ref mother1, ref mother2) => MoneyStrategy::Minus(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Multiply(ref father1, ref father2) => match *other {
                    MoneyStrategy::Multiply(ref mother1, ref mother2) => MoneyStrategy::Multiply(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Divide(ref father1, ref father2) => match *other {
                    MoneyStrategy::Divide(ref mother1, ref mother2) => MoneyStrategy::Divide(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
            }
        }
    }

    fn calculate<A, B>(&self, simulation: &Simulation<A, B>, left: &str, right: &str) -> f64 where A: Strategy, B: Strategy {
        match *self {
            MoneyStrategy::AllIn => simulation.sum(),

            MoneyStrategy::Fixed(amount) => amount.unwrap(),

            MoneyStrategy::Percentage(Percentage(percentage), ref a) => a.calculate(simulation, left, right) * percentage,

            MoneyStrategy::Confidence(bezier, ref a) => a.calculate(simulation, left, right) * {
                let left_len = simulation.get_len(left);
                let right_len = simulation.get_len(right);

                // TODO make this a percentage ?
                ((bezier.sample_y(left_len) + bezier.sample_y(right_len)) / 2.0) as f64
            },

            MoneyStrategy::Plus(ref a, ref b) => a.calculate(simulation, left, right) + b.calculate(simulation, left, right),
            MoneyStrategy::Minus(ref a, ref b) => a.calculate(simulation, left, right) - b.calculate(simulation, left, right),
            MoneyStrategy::Multiply(ref a, ref b) => a.calculate(simulation, left, right) * b.calculate(simulation, left, right),
            MoneyStrategy::Divide(ref a, ref b) => a.calculate(simulation, left, right) / b.calculate(simulation, left, right),
        }
    }
}

impl Gene for MoneyStrategy {
    fn new() -> Self {
        MoneyStrategy::_new(0)
    }

    fn choose(&self, other: &Self) -> Self {
        self._choose(other, 0)
    }
}


#[derive(Debug, Clone)]
enum LimitStrategy<A> {
    True,
    False,
    Greater(A),
    GreaterEqual(A),
    Lesser(A),
    LesserEqual(A),
    And(Box<LimitStrategy<A>>, Box<LimitStrategy<A>>),
    Or(Box<LimitStrategy<A>>, Box<LimitStrategy<A>>),
}

impl<A> LimitStrategy<A> where A: Gene, A: Clone {
    fn _and(self, other: Self) -> Self {
        match self {
            LimitStrategy::True => other,
            LimitStrategy::False => LimitStrategy::False,
            left => match other {
                LimitStrategy::True => left,
                LimitStrategy::False => LimitStrategy::False,
                right => LimitStrategy::And(Box::new(left), Box::new(right)),
            }
        }
    }

    fn _or(self, other: Self) -> Self {
        match self {
            LimitStrategy::True => LimitStrategy::True,
            LimitStrategy::False => other,
            left => match other {
                LimitStrategy::True => LimitStrategy::True,
                LimitStrategy::False => left,
                right => LimitStrategy::Or(Box::new(left), Box::new(right)),
            }
        }
    }

    fn _new(depth: u32) -> Self {
        if depth >= MAX_RECURSION {
            let rand = gen_rand_index(6u32);

            if rand == 0 {
                LimitStrategy::True

            } else if rand == 1 {
                LimitStrategy::False

            } else if rand == 2 {
                LimitStrategy::Greater(Gene::new())

            } else if rand == 3 {
                LimitStrategy::GreaterEqual(Gene::new())

            } else if rand == 4 {
                LimitStrategy::Lesser(Gene::new())

            } else {
                LimitStrategy::LesserEqual(Gene::new())
            }

        } else {
            let rand = gen_rand_index(8u32);

            if rand == 0 {
                LimitStrategy::True

            } else if rand == 1 {
                LimitStrategy::False

            } else if rand == 2 {
                LimitStrategy::Greater(Gene::new())

            } else if rand == 3 {
                LimitStrategy::GreaterEqual(Gene::new())

            } else if rand == 4 {
                LimitStrategy::Lesser(Gene::new())

            } else if rand == 5 {
                LimitStrategy::LesserEqual(Gene::new())

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
                LimitStrategy::Greater(ref father) => match *other {
                    LimitStrategy::Greater(ref mother) => LimitStrategy::Greater(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                LimitStrategy::GreaterEqual(ref father) => match *other {
                    LimitStrategy::GreaterEqual(ref mother) => LimitStrategy::GreaterEqual(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                LimitStrategy::Lesser(ref father) => match *other {
                    LimitStrategy::Lesser(ref mother) => LimitStrategy::Lesser(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                LimitStrategy::LesserEqual(ref father) => match *other {
                    LimitStrategy::LesserEqual(ref mother) => LimitStrategy::LesserEqual(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                LimitStrategy::And(ref father1, ref father2) => match *other {
                    LimitStrategy::And(ref mother1, ref mother2) => father1._choose(&mother1, depth + 1)._and(father2._choose(&mother2, depth + 1)),
                    _ => choose2(self, other),
                },
                LimitStrategy::Or(ref father1, ref father2) => match *other {
                    LimitStrategy::Or(ref mother1, ref mother2) => father1._choose(&mother1, depth + 1)._or(father2._choose(&mother2, depth + 1)),
                    _ => choose2(self, other),
                },
                _ => choose2(self, other),
            }
        }
    }
}

impl<A> LimitStrategy<A> where A: PartialOrd {
    fn is_match(&self, input: &A) -> bool {
        match *self {
            LimitStrategy::True => true,
            LimitStrategy::False => false,
            LimitStrategy::Greater(ref value) => value > input,
            LimitStrategy::GreaterEqual(ref value) => value >= input,
            LimitStrategy::Lesser(ref value) => value < input,
            LimitStrategy::LesserEqual(ref value) => value <= input,
            LimitStrategy::And(ref left, ref right) => left.is_match(input) && right.is_match(input),
            LimitStrategy::Or(ref left, ref right) => left.is_match(input) || right.is_match(input),
        }
    }
}

impl<A> Gene for LimitStrategy<A> where A: Gene, A: Clone {
    fn new() -> Self {
        Self::_new(0)
    }

    fn choose(&self, other: &Self) -> Self {
        self._choose(other, 0)
    }
}


#[derive(Debug, Clone)]
enum PredictionStrategy {
    Winrate(LimitStrategy<Percentage>, LimitStrategy<Percentage>),
    Earnings(LimitStrategy<f64>, LimitStrategy<BetAmount>), // TODO should this use BetAmount ?
    Upset(LimitStrategy<Percentage>, LimitStrategy<Percentage>),
    Favored(LimitStrategy<Percentage>, LimitStrategy<Percentage>),
}

impl PredictionStrategy {
    fn calculate<A>(cap: &LimitStrategy<A>, diff: &LimitStrategy<A>, left_amount: A, right_amount: A) -> Winner
        where A: PartialOrd,
              A: Diff {
        if diff.is_match(&left_amount.diff(&right_amount)) {
            if left_amount > right_amount {
                if cap.is_match(&left_amount) {
                    return Winner::Left;
                }

            } else if right_amount > left_amount {
                if cap.is_match(&right_amount) {
                    return Winner::Right;
                }
            }
        }

        return Winner::None;
    }

    fn predict_winner<A, B>(&self, simulation: &Simulation<A, B>, left: &str, right: &str) -> Winner where A: Strategy, B: Strategy {
        match *self {
            PredictionStrategy::Winrate(ref cap, ref diff) =>
                PredictionStrategy::calculate(cap, diff, simulation.get_winrate(left), simulation.get_winrate(right)),

            PredictionStrategy::Earnings(ref cap, ref diff) => {
                let left_amount = simulation.get_sum(left);
                let right_amount = simulation.get_sum(right);

                // TODO is using BetAmount correct ?
                if diff.is_match(&BetAmount(left_amount.diff(&right_amount))) {
                    if left_amount > right_amount {
                        if cap.is_match(&left_amount) {
                            return Winner::Left;
                        }

                    } else if right_amount > left_amount {
                        if cap.is_match(&right_amount) {
                            return Winner::Right;
                        }
                    }
                }

                return Winner::None;
            },

            PredictionStrategy::Upset(ref cap, ref diff) =>
                PredictionStrategy::calculate(cap, diff, simulation.get_upsets(left), simulation.get_upsets(right)),

            PredictionStrategy::Favored(ref cap, ref diff) =>
                PredictionStrategy::calculate(cap, diff, simulation.get_favored(left), simulation.get_favored(right)),
        }
    }
}

impl Gene for PredictionStrategy {
    fn new() -> Self {
        let rand = gen_rand_index(4u32);

        if rand == 0 {
            PredictionStrategy::Winrate(Gene::new(), Gene::new())

        } else if rand == 1 {
            PredictionStrategy::Earnings(Gene::new(), Gene::new())

        } else if rand == 2 {
            PredictionStrategy::Upset(Gene::new(), Gene::new())

        } else {
            PredictionStrategy::Favored(Gene::new(), Gene::new())
        }
    }

    // TODO is this correct ?
    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            match *self {
                PredictionStrategy::Winrate(ref father1, ref father2) => match *other {
                    PredictionStrategy::Winrate(ref mother1, ref mother2) => PredictionStrategy::Winrate(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                PredictionStrategy::Earnings(ref father1, ref father2) => match *other {
                    PredictionStrategy::Earnings(ref mother1, ref mother2) => PredictionStrategy::Earnings(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                PredictionStrategy::Upset(ref father1, ref father2) => match *other {
                    PredictionStrategy::Upset(ref mother1, ref mother2) => PredictionStrategy::Upset(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                PredictionStrategy::Favored(ref father1, ref father2) => match *other {
                    PredictionStrategy::Favored(ref mother1, ref mother2) => PredictionStrategy::Favored(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
            }
        }
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

impl Diff for BetAmount {
    fn diff(&self, other: &Self) -> Self {
        BetAmount(self.unwrap().diff(&other.unwrap()))
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
        rand::thread_rng().gen::<bool>()
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
        let Closed01(val) = rand::thread_rng().gen::<Closed01<f64>>();
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


#[derive(Debug, Clone)]
struct Uf64(f64);

impl Uf64 {
    fn unwrap(self) -> f64 {
        let Uf64(input) = self;
        input
    }
}

impl Gene for Uf64 {
    fn new() -> Self {
        let Percentage(percentage) = Gene::new();
        Uf64(percentage * std::f64::MAX)
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
        let Closed01(val) = rand::thread_rng().gen::<Closed01<f32>>();
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


fn clamp(mut from: Point, mut ctrl1: Point, mut ctrl2: Point, mut to: Point) -> CubicBezierSegment {
    /*if from.y < to.y {
        let average = average2(from.y, to.y);
        from.y = average;
        to.y = average;
    }

    if from.y < ctrl1.y {
        ctrl1.y = from.y;
    }

    if ctrl2.y < to.y {
        ctrl2.y = to.y;
    }*/

    CubicBezierSegment {
        from: from,
        ctrl1: ctrl1,
        ctrl2: ctrl2,
        to: to,
    }
}

impl Gene for CubicBezierSegment {
    fn new() -> Self {
        clamp(
            Gene::new(),
            Gene::new(),
            Gene::new(),
            Gene::new(),
        )
    }

    fn choose(&self, other: &Self) -> Self {
        clamp(
            self.from.choose(&other.from),
            self.ctrl1.choose(&other.ctrl1),
            self.ctrl2.choose(&other.ctrl2),
            self.to.choose(&other.to),
        )
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
    rand::thread_rng().gen_range(0, index)
}


// TODO verify that this is correct
fn rand_is_percent(Percentage(input): Percentage) -> bool {
    let Percentage(rand) = Gene::new();
    rand <= input
}


fn choose<'a, A>(values: &'a [A]) -> Option<&'a A> {
    rand::thread_rng().choose(values)
}


fn shuffle<A>(slice: &mut [A]) {
    rand::thread_rng().shuffle(slice);
}


fn average2(left: f32, right: f32) -> f32 {
    (left + right) / 2.0
}


fn average<A>(sum: f64, vec: &Vec<A>) -> f64 {
    let len = vec.len();

    if len == 0 {
        0.0

    } else {
        sum / (len as f64)
    }
}


#[derive(Debug)]
pub struct Character<'a> {
    name: &'a str,
    matches: Vec<&'a Record>,
}

impl<'a> Character<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name: name,
            matches: Vec::new()
        }
    }

    fn upsets(&self) -> Percentage {
        let mut upsets: f64 = 0.0;

        let len = self.matches.len();

        if len == 0 {
            // TODO is 0.0 or 0.5 better ?
            Percentage(0.0)

        } else {
            for record in self.matches.iter() {
                // TODO better detection for whether the character matches or not
                if (record.left.name == self.name &&
                    (record.right.bet_amount / record.left.bet_amount) > 1.0) ||

                   (record.right.name == self.name &&
                    (record.left.bet_amount / record.right.bet_amount) > 1.0) {

                    upsets += 1.0;
                }
            }

            Percentage(upsets / (len as f64))
        }
    }

    fn favored(&self) -> Percentage {
        let mut favored: f64 = 0.0;

        let len = self.matches.len();

        if len == 0 {
            // TODO is 0.0 or 0.5 better ?
            Percentage(0.0)

        } else {
            for record in self.matches.iter() {
                // TODO better detection for whether the character matches or not
                if (record.left.name == self.name &&
                    (record.left.bet_amount / record.right.bet_amount) > 1.0) ||

                   (record.right.name == self.name &&
                    (record.right.bet_amount / record.left.bet_amount) > 1.0) {

                    favored += 1.0;
                }
            }

            Percentage(favored / (len as f64))
        }
    }

    fn winrate(&self) -> Percentage {
        let mut wins: f64 = 0.0;

        let len = self.matches.len();

        if len == 0 {
            Percentage(0.5)

        } else {
            for record in self.matches.iter() {
                if record.is_winner(self.name) {
                    wins += 1.0;
                }
            }

            Percentage(wins / (len as f64))
        }
    }

    fn sum(&self) -> f64 {
        let mut sum: f64 = 0.0;

        for record in self.matches.iter() {
            match record.winner {
                // TODO better detection for whether the character matches or not
                Winner::Left => if record.left.name == self.name {
                    sum += record.right.bet_amount / record.left.bet_amount;

                } else {
                    sum -= 1.0;
                },

                // TODO better detection for whether the character matches or not
                Winner::Right => if record.right.name == self.name {
                    sum += record.left.bet_amount / record.right.bet_amount;

                } else {
                    sum -= 1.0;
                },

                Winner::None => {}
            }
        }

        sum
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
    characters: HashMap<&'c str, Character<'c>>,
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
        let character = self.characters.entry(key).or_insert_with(|| Character::new(key));

        character.matches.push(record);

        let len = character.matches.len();

        if len > self.max_character_len {
            self.max_character_len = len;
        }
    }

    fn insert_record(&mut self, record: &'c Record) {
        self.insert_match(&record.left.name, record);
        self.insert_match(&record.right.name, record);
    }

    fn get_sum(&self, key: &'c str) -> f64 {
        match self.characters.get(key) {
            Some(character) => character.sum(),
            None => 0.0,
        }
    }

    fn get_upsets(&self, key: &'c str) -> Percentage {
        match self.characters.get(key) {
            Some(character) => character.upsets(),
            // TODO is 0.0 or 0.5 better ?
            None => Percentage(0.0),
        }
    }

    fn get_favored(&self, key: &'c str) -> Percentage {
        match self.characters.get(key) {
            Some(character) => character.favored(),
            // TODO is 0.0 or 0.5 better ?
            None => Percentage(0.0),
        }
    }

    fn get_winrate(&self, key: &'c str) -> Percentage {
        match self.characters.get(key) {
            Some(character) => character.winrate(),
            None => Percentage(0.5),
        }
    }

    fn get_len(&self, key: &'c str) -> f32 {
        match self.characters.get(key) {
            // TODO what if the len is longer than MAX_VEC_LEN ?
            Some(character) => (character.matches.len() as f32) / MAX_VEC_LEN,
            None => 0.0,
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

    fn pick_winner<C>(&self, strategy: &C, record: &Record) -> Bet where C: Strategy {
        match strategy.bet(self, &record.left.name, &record.right.name) {
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

    fn calculate(&mut self, record: &Record) {
        let winner = match record.mode {
            Mode::Matchmaking => {
                if self.in_tournament {
                    self.in_tournament = false;
                    //println!("rollover: {}", self.tournament_sum);
                    self.sum += self.tournament_sum;
                    self.tournament_sum = TOURNAMENT_BALANCE;
                }

                match self.matchmaking_strategy {
                    Some(a) => self.pick_winner(a, record),
                    None => return,
                }
            },
            Mode::Tournament => {
                self.in_tournament = true;

                match self.tournament_strategy {
                    Some(a) => self.pick_winner(a, record),
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
    fitness: f64,
    successes: f64,
    failures: f64,
    max_character_len: usize,

    // Genes
    prediction_strategy: PredictionStrategy,
    money_strategy: MoneyStrategy,
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
            prediction_strategy: self.prediction_strategy.choose(&other.prediction_strategy),
            money_strategy: self.money_strategy.choose(&other.money_strategy),
        }.calculate_fitness(settings)
    }
}

impl Strategy for BetStrategy {
    fn bet<A, B>(&self, simulation: &Simulation<A, B>, left: &str, right: &str) -> Bet where A: Strategy, B: Strategy {
        match self.prediction_strategy.predict_winner(simulation, left, right) {
            Winner::Left => Bet::Left(self.money_strategy.calculate(simulation, left, right)),
            Winner::Right => Bet::Right(self.money_strategy.calculate(simulation, left, right)),
            Winner::None => Bet::None,
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
            (0..self.amount).into_par_iter().map(|_| A::new(self.data)).collect()
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
