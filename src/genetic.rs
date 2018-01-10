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

const SALT_MINE_AMOUNT: f64 = 258.0; // TODO verify that this is correct
const TOURNAMENT_BALANCE: f64 = 1375.0; // TODO


#[derive(Debug, Clone, Copy)]
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


trait Strategy {
    fn bet(&self, simulation: &Simulation, left: &str, right: &str) -> Bet;
}


pub trait Creature<A>: PartialOrd {
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
    Percentage(Percentage),
    Confidence(CubicBezierSegment),
    Plus(Box<MoneyStrategy>, Box<MoneyStrategy>),
    Minus(Box<MoneyStrategy>, Box<MoneyStrategy>),
    Multiply(Box<MoneyStrategy>, Box<MoneyStrategy>),
    Divide(Box<MoneyStrategy>, Box<MoneyStrategy>),
}

impl MoneyStrategy {
    fn calculate(&self, simulation: &Simulation, left: &str, right: &str) -> f64 {
        match *self {
            MoneyStrategy::AllIn => simulation.sum(),

            MoneyStrategy::Fixed(ref amount) => amount.unwrap(),

            MoneyStrategy::Percentage(Percentage(percentage)) => simulation.sum() * percentage,

            MoneyStrategy::Confidence(bezier) => simulation.sum() * {
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
    // TODO auto-derive this
    fn new() -> Self {
        let rand = gen_rand_index(8u32);

        if rand == 0 {
            MoneyStrategy::AllIn

        } else if rand == 1 {
            MoneyStrategy::Fixed(Gene::new())

        } else if rand == 2 {
            MoneyStrategy::Percentage(Gene::new())

        } else if rand == 3 {
            MoneyStrategy::Confidence(Gene::new())

        } else if rand == 4 {
            MoneyStrategy::Plus(Box::new(Gene::new()), Box::new(Gene::new()))

        } else if rand == 5 {
            MoneyStrategy::Minus(Box::new(Gene::new()), Box::new(Gene::new()))

        } else if rand == 6 {
            MoneyStrategy::Multiply(Box::new(Gene::new()), Box::new(Gene::new()))

        } else {
            MoneyStrategy::Divide(Box::new(Gene::new()), Box::new(Gene::new()))
        }
    }

    // TODO is this correct ?
    // TODO auto-derive this
    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            match *self {
                MoneyStrategy::AllIn => choose2(self, other),
                MoneyStrategy::Fixed(ref father) => match *other {
                    MoneyStrategy::Fixed(ref mother) => MoneyStrategy::Fixed(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Percentage(ref father) => match *other {
                    MoneyStrategy::Percentage(ref mother) => MoneyStrategy::Percentage(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Confidence(father) => match *other {
                    MoneyStrategy::Confidence(mother) => MoneyStrategy::Confidence(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Plus(ref father1, ref father2) => match *other {
                    MoneyStrategy::Plus(ref mother1, ref mother2) => MoneyStrategy::Plus(Box::new(father1.choose(&mother1)), Box::new(father2.choose(&mother2))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Minus(ref father1, ref father2) => match *other {
                    MoneyStrategy::Minus(ref mother1, ref mother2) => MoneyStrategy::Minus(Box::new(father1.choose(&mother1)), Box::new(father2.choose(&mother2))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Multiply(ref father1, ref father2) => match *other {
                    MoneyStrategy::Multiply(ref mother1, ref mother2) => MoneyStrategy::Multiply(Box::new(father1.choose(&mother1)), Box::new(father2.choose(&mother2))),
                    _ => choose2(self, other),
                },
                MoneyStrategy::Divide(ref father1, ref father2) => match *other {
                    MoneyStrategy::Divide(ref mother1, ref mother2) => MoneyStrategy::Divide(Box::new(father1.choose(&mother1)), Box::new(father2.choose(&mother2))),
                    _ => choose2(self, other),
                },
            }
        }
    }
}


#[derive(Debug, Clone)]
enum ChooseStrategy {
    Winrate,
    Earnings(Option<f64>, Option<BetAmount>), // TODO should this use BetAmount ?
    Upset(Option<Percentage>, Option<Percentage>),
    Favored(Option<Percentage>, Option<Percentage>),
}

impl ChooseStrategy {
    fn predict_winner(&self, simulation: &Simulation, left: &str, right: &str) -> Winner {
        match *self {
            ChooseStrategy::Winrate => {
                let Percentage(left_amount) = simulation.get_winrate(left);
                let Percentage(right_amount) = simulation.get_winrate(right);

                if left_amount > right_amount {
                    Winner::Left

                } else if right_amount > left_amount {
                    Winner::Right

                } else {
                    Winner::None
                }
            },

            ChooseStrategy::Earnings(cap, diff) => {
                let left_amount = simulation.get_sum(left);
                let right_amount = simulation.get_sum(right);

                if (left_amount - right_amount).abs() >= diff.unwrap_or_else(|| BetAmount(0.0)).unwrap() {
                    if left_amount > right_amount {
                        match cap {
                            Some(amount) => if left_amount >= amount {
                                return Winner::Left;
                            },
                            None => return Winner::Left,
                        }

                    } else if right_amount > left_amount {
                        match cap {
                            Some(amount) => if right_amount >= amount {
                                return Winner::Right;
                            },
                            None => return Winner::Right,
                        }
                    }
                }

                return Winner::None;
            },

            ChooseStrategy::Upset(cap, diff) => {
                let Percentage(left_amount) = simulation.get_upsets(left);
                let Percentage(right_amount) = simulation.get_upsets(right);

                if (left_amount - right_amount).abs() >= diff.unwrap_or_else(|| Percentage(0.0)).unwrap() {
                    if left_amount > right_amount {
                        if left_amount >= cap.unwrap_or_else(|| Percentage(0.0)).unwrap() {
                            return Winner::Left;
                        }

                    } else if right_amount > left_amount {
                        if right_amount >= cap.unwrap_or_else(|| Percentage(0.0)).unwrap() {
                            return Winner::Right;
                        }
                    }
                }

                return Winner::None;
            },

            ChooseStrategy::Favored(cap, diff) => {
                let Percentage(left_amount) = simulation.get_favored(left);
                let Percentage(right_amount) = simulation.get_favored(right);

                if (left_amount - right_amount).abs() >= diff.unwrap_or_else(|| Percentage(0.0)).unwrap() {
                    if left_amount > right_amount {
                        if left_amount >= cap.unwrap_or_else(|| Percentage(0.0)).unwrap() {
                            return Winner::Left;
                        }

                    } else if right_amount > left_amount {
                        if left_amount >= cap.unwrap_or_else(|| Percentage(0.0)).unwrap() {
                            return Winner::Right;
                        }
                    }
                }

                return Winner::None;
            },
        }
    }
}

impl Gene for ChooseStrategy {
    fn new() -> Self {
        let rand = gen_rand_index(4u32);

        if rand == 0 {
            ChooseStrategy::Winrate

        } else if rand == 1 {
            ChooseStrategy::Earnings(Gene::new(), Gene::new())

        } else if rand == 2 {
            ChooseStrategy::Upset(Gene::new(), Gene::new())

        } else {
            ChooseStrategy::Favored(Gene::new(), Gene::new())
        }
    }

    // TODO is this correct ?
    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            match *self {
                ChooseStrategy::Earnings(ref father1, ref father2) => match *other {
                    ChooseStrategy::Earnings(ref mother1, ref mother2) => ChooseStrategy::Earnings(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                ChooseStrategy::Upset(ref father1, ref father2) => match *other {
                    ChooseStrategy::Upset(ref mother1, ref mother2) => ChooseStrategy::Upset(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                ChooseStrategy::Favored(ref father1, ref father2) => match *other {
                    ChooseStrategy::Favored(ref mother1, ref mother2) => ChooseStrategy::Favored(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                _ => choose2(self, other),
            }
        }
    }
}


#[derive(Debug, Clone, Copy)]
struct BetAmount(f64);

impl BetAmount {
    fn unwrap(self) -> f64 {
        let BetAmount(value) = self;
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
struct Simulation<'a> {
    settings: &'a SimulationSettings<'a>,
    sum: f64,
    tournament_sum: f64,
    in_tournament: bool,
    successes: f64,
    failures: f64,
    max_character_len: usize,
    characters: HashMap<&'a str, Character<'a>>,
}

impl<'a> Simulation<'a> {
    pub fn new(settings: &'a SimulationSettings<'a>) -> Self {
        Self {
            settings: settings,
            sum: SALT_MINE_AMOUNT,
            tournament_sum: TOURNAMENT_BALANCE,
            in_tournament: false,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            characters: HashMap::new()
        }
    }

    fn insert_match(&mut self, key: &'a str, record: &'a Record) {
        let character = self.characters.entry(key).or_insert_with(|| Character::new(key));

        character.matches.push(record);

        let len = character.matches.len();

        if len > self.max_character_len {
            self.max_character_len = len;
        }
    }

    fn insert_record(&mut self, record: &'a Record) {
        self.insert_match(&record.left.name, record);
        self.insert_match(&record.right.name, record);
    }

    fn get_sum(&self, key: &'a str) -> f64 {
        match self.characters.get(key) {
            Some(character) => character.sum(),
            None => 0.0,
        }
    }

    fn get_upsets(&self, key: &'a str) -> Percentage {
        match self.characters.get(key) {
            Some(character) => character.upsets(),
            // TODO is 0.0 or 0.5 better ?
            None => Percentage(0.0),
        }
    }

    fn get_favored(&self, key: &'a str) -> Percentage {
        match self.characters.get(key) {
            Some(character) => character.favored(),
            // TODO is 0.0 or 0.5 better ?
            None => Percentage(0.0),
        }
    }

    fn get_winrate(&self, key: &'a str) -> Percentage {
        match self.characters.get(key) {
            Some(character) => character.winrate(),
            None => Percentage(0.5),
        }
    }

    fn get_len(&self, key: &'a str) -> f32 {
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

    fn pick_winner(&self, strategy: &Strategy, record: &Record) -> Bet {
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

    fn calculate(&mut self, strategy: &Strategy, record: &Record) {
        match record.mode {
            Mode::Matchmaking => {
                if self.in_tournament {
                    self.in_tournament = false;
                    //println!("rollover: {}", self.tournament_sum);
                    self.sum += self.tournament_sum;
                    self.tournament_sum = TOURNAMENT_BALANCE;
                }

                match self.settings.mode {
                    Mode::Tournament => return,
                    Mode::Matchmaking => {},
                }
            },
            Mode::Tournament => {
                self.in_tournament = true;

                match self.settings.mode {
                    Mode::Matchmaking => return,
                    Mode::Tournament => {},
                }
            },
        }

        //println!("tournament: {}", self.in_tournament);

        //println!("sum: {}", self.sum());

        let increase = match self.pick_winner(strategy, record) {
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

    pub fn simulate(&mut self, strategy: &Strategy) {
        // shuffle(records);

        for record in self.settings.records.iter() {
            self.calculate(strategy, record);
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
    choose_strategy: ChooseStrategy,
    money_strategy: MoneyStrategy,
}

impl<'a> BetStrategy {
    /*fn calculate_prediction(simulation: &Simulation, record: &Record) -> Winner {

    }*/

    fn calculate_fitness(mut self, settings: &SimulationSettings<'a>) -> Self {
        let mut simulation = Simulation::new(settings);

        simulation.simulate(&self);

        self.fitness = simulation.sum;
        self.successes = simulation.successes;
        self.failures = simulation.failures;
        self.max_character_len = simulation.max_character_len;

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
            choose_strategy: Gene::new(),
            money_strategy: Gene::new(),
        }.calculate_fitness(settings)
    }

    fn breed(&self, other: &Self, settings: &SimulationSettings<'a>) -> Self {
        Self {
            fitness: 0.0,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            choose_strategy: self.choose_strategy.choose(&other.choose_strategy),
            money_strategy: self.money_strategy.choose(&other.money_strategy),
        }.calculate_fitness(settings)
    }
}

impl Strategy for BetStrategy {
    fn bet(&self, simulation: &Simulation, left: &str, right: &str) -> Bet {
        match self.choose_strategy.predict_winner(simulation, left, right) {
            Winner::Left => Bet::Left(self.money_strategy.calculate(simulation, left, right)),
            Winner::Right => Bet::Right(self.money_strategy.calculate(simulation, left, right)),
            Winner::None => Bet::None,
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
        // TODO is unwrap correct ?
        let index = self.populace.binary_search_by(|a| a.partial_cmp(&creature).unwrap());

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

            if super::WEB_BUILD {
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
        let new_creatures: Vec<A> = if super::WEB_BUILD {
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
