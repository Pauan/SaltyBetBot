use std;
use std::collections::{ HashMap };
use record::{ Record, Mode, Winner };
use genetic::{ Gene, gen_rand_index, rand_is_percent, MUTATION_RATE, choose2 };


const MAX_VEC_LEN: f64 = 10000.0;
const SALT_MINE_AMOUNT: f64 = 258.0; // TODO verify that this is correct
const TOURNAMENT_BALANCE: f64 = 1375.0; // TODO


pub enum Bet {
    Left(f64),
    Right(f64),
    None,
}


pub trait Strategy: Sized + std::fmt::Debug {
    fn bet<A, B>(&self, simulation: &Simulation<A, B>, left: &str, right: &str) -> Bet where A: Strategy, B: Strategy;
}


pub trait Calculate<A> {
    fn calculate<'a, 'b, 'c, B, C>(&self, &Simulation<'a, 'b, 'c, B, C>, &'c str, &'c str) -> A
        where B: Strategy,
              C: Strategy;
}


#[derive(Debug, Clone)]
pub enum LookupStatistic {
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
pub enum LookupFilter {
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
pub enum LookupSide {
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
pub enum Lookup {
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
pub struct Simulation<'a, 'b, 'c, A, B> where A: Strategy, A: 'a, B: Strategy, B: 'b {
    pub matchmaking_strategy: Option<&'a A>,
    pub tournament_strategy: Option<&'b B>,
    pub sum: f64,
    tournament_sum: f64,
    in_tournament: bool,
    pub successes: f64,
    pub failures: f64,
    pub max_character_len: usize,
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

        let increase = match winner {
            Bet::Left(bet_amount) => match record.winner {
                Winner::Left => {
                    let odds = record.right.bet_amount / record.left.bet_amount;
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
    }

    pub fn simulate(&mut self, records: &'c Vec<Record>) {
        for record in records.iter() {
            self.calculate(record);
            self.insert_record(record);
        }
    }

    pub fn insert_records(&mut self, records: &'c Vec<Record>) {
        for record in records.iter() {
            self.insert_record(record);
        }
    }
}
