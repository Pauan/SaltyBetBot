use std;
use std::collections::{ HashMap };
use lyon_bezier::Point;
use lyon_bezier::cubic_bezier::CubicBezierSegment;
use rand;
use rand::{ Rng, Closed01 };
use record::{ Record, Mode };
use record::Winner::{ Left, Right };
//use rayon::iter::{ IntoParallelRefIterator, ParallelIterator, FromParallelIterator, IntoParallelIterator };
use rayon::prelude::*;


const MAX_VEC_LEN: f32 = 10000.0;
const MUTATION_RATE: f32 = 0.01;

const SALT_MINE_AMOUNT: f64 = 258.0; // TODO verify that this is correct
const TOURNAMENT_BALANCE: f64 = 1375.0; // TODO


enum Bet {
    Left(f64),
    Right(f64),
    None,
}


#[derive(Debug, Clone, Copy)]
enum MoneyStrategy {
    Fixed(f64),
    Percentage,
}


trait Strategy {
    fn bet(&self, simulation: &Simulation, mode: &Mode, left: &str, right: &str) -> Bet;
}


pub trait Creature<A>: PartialOrd {
    fn new(data: &A) -> Self;

    fn breed(&self, other: &Self, data: &A) -> Self;
}


trait Gene {
    fn new() -> Self;

    fn choose(self, other: Self) -> Self;
}


impl Gene for MoneyStrategy {
    fn new() -> Self {
        if gen_rand_bool(0.5) {
            MoneyStrategy::Fixed(Gene::new())

        } else {
            MoneyStrategy::Percentage
        }
    }

    // TODO is this correct ?
    fn choose(self, other: Self) -> Self {
        // Random mutation
        if gen_rand_bool(MUTATION_RATE) {
            Gene::new()

        // Father
        } else if gen_rand_bool(0.5) {
            self

        // Mother
        } else {
            other
        }
    }
}


impl Gene for bool {
    fn new() -> Self {
        gen_rand_bool(0.5)
    }

    fn choose(self, other: Self) -> Self {
        // Random mutation
        if gen_rand_bool(MUTATION_RATE) {
            Gene::new()

        // Father
        } else if gen_rand_bool(0.5) {
            self

        // Mother
        } else {
            other
        }
    }
}


impl Gene for f64 {
    fn new() -> Self {
        gen_rand_f64()
    }

    fn choose(self, other: Self) -> Self {
        // Random mutation
        if gen_rand_bool(MUTATION_RATE) {
            Gene::new()

        // Father
        } else if gen_rand_bool(0.5) {
            self

        // Mother
        } else {
            other
        }
    }
}


impl Gene for f32 {
    fn new() -> Self {
        gen_rand()
    }

    fn choose(self, other: Self) -> Self {
        // Random mutation
        if gen_rand_bool(MUTATION_RATE) {
            Gene::new()

        // Father
        } else if gen_rand_bool(0.5) {
            self

        // Mother
        } else {
            other
        }
    }
}


impl Gene for Point {
    fn new() -> Self {
        Self::new(Gene::new(), Gene::new())
    }

    fn choose(self, other: Self) -> Self {
        Self::new(self.x.choose(other.x), self.y.choose(other.y))
    }
}


impl Gene for CubicBezierSegment {
    fn new() -> Self {
        Self {
            from: Gene::new(),
            ctrl1: Gene::new(),
            ctrl2: Gene::new(),
            to: Gene::new(),
        }
    }

    fn choose(self, other: Self) -> Self {
        Self {
            from: self.from.choose(other.from),
            ctrl1: self.ctrl1.choose(other.ctrl1),
            ctrl2: self.ctrl2.choose(other.ctrl2),
            to: self.to.choose(other.to),
        }
    }
}


// TODO verify that this is correct
fn gen_rand() -> f32 {
    let Closed01(val) = rand::thread_rng().gen::<Closed01<f32>>();
    val
}


// TODO verify that this is correct
fn gen_rand_f64() -> f64 {
    let Closed01(val) = rand::thread_rng().gen::<Closed01<f64>>();
    val
}


// TODO verify that this is correct
fn gen_rand_bool(percent: f32) -> bool {
    gen_rand() <= percent
}


fn choose<'a, A>(values: &'a [A]) -> Option<&'a A> {
    rand::thread_rng().choose(values)
}


fn shuffle<A>(slice: &mut [A]) {
    rand::thread_rng().shuffle(slice);
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

    fn sum(&self) -> f64 {
        let mut sum: f64 = 0.0;

        for record in self.matches.iter() {
            match record.winner {
                // TODO better detection for whether the character matches or not
                Left(odds) => if record.character_left == self.name {
                    sum += odds;

                } else {
                    sum -= 1.0;
                },

                // TODO better detection for whether the character matches or not
                Right(odds) => if record.character_right == self.name {
                    sum += odds;

                } else {
                    sum -= 1.0;
                }
            }
        }

        sum
    }
}


#[derive(Debug)]
struct Simulation<'a> {
    sum: f64,
    tournament_sum: f64,
    in_tournament: bool,
    successes: f64,
    failures: f64,
    max_character_len: usize,
    characters: HashMap<&'a str, Character<'a>>,
}

impl<'a> Simulation<'a> {
    pub fn new() -> Self {
        Self {
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
        self.insert_match(&record.character_left, record);
        self.insert_match(&record.character_right, record);
    }

    fn get_sum(&self, key: &'a str) -> f64 {
        match self.characters.get(key) {
            Some(character) => character.sum(),
            None => 0.0,
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
        match strategy.bet(self, &record.mode, &record.character_left, &record.character_right) {
            Bet::Left(bet_amount) => Bet::Left(self.clamp(bet_amount)),

            Bet::Right(bet_amount) =>Bet::Right(self.clamp(bet_amount)),

            Bet::None => if self.is_in_mines() {
                if gen_rand_bool(0.5) {
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
            },
            Mode::Tournament => {
                self.in_tournament = true;
            },
        }

        //println!("tournament: {}", self.in_tournament);

        //println!("sum: {}", self.sum());

        let increase = match self.pick_winner(strategy, record) {
            Bet::Left(bet_amount) => {
                //println!("bet: {}", bet_amount);
                match record.winner {
                Left(odds) => {
                    if odds > 1.0 && (bet_amount * odds).ceil() > 100000000.0 {
                        //println!("BIG odds: {}", odds);
                    } else {
                        //println!("odds: {}", odds);
                    }

                    self.successes += 1.0;
                    (bet_amount * odds).ceil()
                },
                Right(_) => {
                    self.failures += 1.0;
                    -bet_amount
                },
            }
        },
            Bet::Right(bet_amount) => {
                //println!("bet: {}", bet_amount);
                match record.winner {
                Right(odds) => {
                    if odds > 1.0 && (bet_amount * odds).ceil() > 100000000.0 {
                        //println!("BIG odds: {}", odds);
                    } else {
                        //println!("odds: {}", odds);
                    }

                    self.successes += 1.0;
                    (bet_amount * odds).ceil()
                },
                Left(_) => {
                    self.failures += 1.0;
                    -bet_amount
                },
            }
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

    pub fn simulate(&mut self, strategy: &Strategy, records: &'a Vec<Record>) {
        // shuffle(records);

        for record in records.iter() {
            self.calculate(strategy, record);
            self.insert_record(record);
        }
    }
}



#[derive(Debug)]
pub struct OddsStrategy {
    fitness: f64,
    successes: f64,
    failures: f64,
    max_character_len: usize,

    // Genes
    matches_len: CubicBezierSegment,
    tournament_confidence: CubicBezierSegment,
    money_strategy: MoneyStrategy,
}

impl OddsStrategy {
    /*fn calculate_prediction(simulation: &Simulation, record: &Record) -> Winner {

    }*/

    fn calculate_fitness(mut self, records: &Vec<Record>) -> Self {
        let mut simulation = Simulation::new();

        simulation.simulate(&self, records);

        self.fitness = average(simulation.sum, records);
        self.successes = simulation.successes;
        self.failures = simulation.failures;
        self.max_character_len = simulation.max_character_len;

        self
    }

    fn calculate_bet(&self, simulation: &Simulation, mode: &Mode, left: &str, right: &str) -> f64 {
        let left_len = simulation.get_len(left);
        let right_len = simulation.get_len(right);

        let bezier = match mode {
            &Mode::Matchmaking => self.matches_len,
            &Mode::Tournament => self.tournament_confidence,
        };

        let confidence = ((bezier.sample_y(left_len) + bezier.sample_y(right_len)) / 2.0) as f64;

        simulation.sum() * confidence
    }
}

impl Creature<Vec<Record>> for OddsStrategy {
    fn new(records: &Vec<Record>) -> Self {
        Self {
            fitness: 0.0,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            matches_len: Gene::new(),
            tournament_confidence: Gene::new(),
            money_strategy: Gene::new(),
        }.calculate_fitness(records)
    }

    fn breed(&self, other: &Self, records: &Vec<Record>) -> Self {
        Self {
            fitness: 0.0,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            matches_len: self.matches_len.choose(other.matches_len),
            tournament_confidence: self.tournament_confidence.choose(other.tournament_confidence),
            money_strategy: self.money_strategy.choose(other.money_strategy),
        }.calculate_fitness(records)
    }
}

impl Strategy for OddsStrategy {
    fn bet(&self, simulation: &Simulation, mode: &Mode, left: &str, right: &str) -> Bet {
        let left_amount = simulation.get_sum(left);
        let right_amount = simulation.get_sum(right);

        if left_amount > right_amount {
            Bet::Left(self.calculate_bet(simulation, mode, left, right))

        } else if right_amount > left_amount {
            Bet::Right(self.calculate_bet(simulation, mode, left, right))

        } else {
            Bet::None
        }
    }
}

impl PartialOrd for OddsStrategy {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.fitness.partial_cmp(&other.fitness)
    }
}

impl PartialEq for OddsStrategy {
    fn eq(&self, other: &Self) -> bool {
        self.fitness == other.fitness
    }
}



#[derive(Debug)]
pub struct Population<'a, A, B> where A: Creature<B>, B: 'a {
    data: &'a B,
    amount: usize,
    populace: Vec<A>,
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
        let mut index: f32 = 0.0;

        let len = (self.populace.len() - 1) as f32;

        // TODO is it guaranteed that retain operates from left-to-right ?
        self.populace.retain(|_| {
            let keep = gen_rand_bool(index / len);
            index += 1.0;
            keep
        });
    }

    fn breed_populace(&mut self) {
        let new_creatures: Vec<A> = (self.populace.len()..self.amount).into_par_iter().map(|_| {
            let father = choose(&self.populace);
            let mother = choose(&self.populace);

            match father {
                Some(father) => match mother {
                    Some(mother) => father.breed(mother, self.data),
                    None => A::new(self.data),
                },
                None => A::new(self.data),
            }
        }).collect();

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn best(&self) -> &A {
        self.populace.last().unwrap()
    }

    pub fn init(&mut self) {
        let new_creatures: Vec<A> = (0..self.amount).into_par_iter().map(|_| A::new(self.data)).collect();

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn next_generation(&mut self) {
        self.kill_populace();
        self.breed_populace();
    }
}
