use std;
use std::collections::{ HashMap };
use record::{ Record, Mode, Winner, Tier };
use genetic::{ Gene, gen_rand_index, rand_is_percent, MUTATION_RATE, choose2 };
use types::{Lookup, LookupSide, LookupFilter, LookupStatistic};


// TODO this should take into account the user's real limit
pub const SALT_MINE_AMOUNT: f64 = 200.0 + 550.0 + 1100.0; // TODO verify that this is correct

// TODO this should take into account the user's real limit
pub const TOURNAMENT_BALANCE: f64 = 1000.0 + 550.0 + 1100.0; // TODO verify that this is correct

// The percentage of profit per match that `expected_bet` should try to get
const DESIRED_PERCENTAGE_PROFIT: f64 = 0.10;

// ~7.7 minutes
pub const NORMAL_MATCH_TIME: f64 = 1000.0 * (60.0 + (80.0 * 5.0));

// TODO
const MAX_EXHIBITS_DURATION: f64 = NORMAL_MATCH_TIME * 1.0;

// ~1.92 hours
const MAX_TOURNAMENT_DURATION: f64 = NORMAL_MATCH_TIME * 15.0;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Bet {
    None,
    Left(f64),
    Right(f64),
}

impl Bet {
    pub fn swap(&self) -> Self {
        match *self {
            Bet::Left(a) => Bet::Right(a),
            Bet::Right(a) => Bet::Left(a),
            Bet::None => Bet::None,
        }
    }

    pub fn amount(&self) -> Option<f64> {
        match *self {
            Bet::Left(a) => Some(a),
            Bet::Right(a) => Some(a),
            Bet::None => None,
        }
    }
}


pub trait Simulator {
    fn average_sum(&self) -> f64;
    fn clamp(&self, bet_amount: f64) -> f64;
    fn matches_len(&self, &str) -> usize;
    fn min_matches_len(&self, left: &str, right: &str) -> f64;
    fn current_money(&self) -> f64;
    fn is_in_mines(&self) -> bool;
    fn lookup_character(&self, &str) -> Vec<&Record>;
    fn lookup_specific_character(&self, left: &str, right: &str) -> Vec<&Record>;
}


pub trait Strategy: Sized + std::fmt::Debug {
    fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> (f64, f64);
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet;
}


impl Strategy for () {
    fn bet_amount<A: Simulator>(&self, _simulation: &A, _tier: &Tier, _left: &str, _right: &str) -> (f64, f64) {
        (0.0, 0.0)
    }

    fn bet<A: Simulator>(&self, _simulation: &A, _tier: &Tier, _left: &str, _right: &str) -> Bet {
        Bet::None
    }
}


pub trait Calculate<A> {
    fn calculate<B: Simulator>(&self, &B, &Tier, &str, &str) -> A;

    fn precalculate(&self) -> Option<A> {
        None
    }

    fn optimize(self) -> Self where Self: Sized {
        self
    }
}


pub mod lookup {
    use super::DESIRED_PERCENTAGE_PROFIT;
    use std::f64::INFINITY;
    use std::iter::IntoIterator;
    use record::{Record, Winner};


    fn iterate_percentage<'a, A, B>(iter: A, mut matches: B) -> Option<f64>
        where A: IntoIterator<Item = &'a Record>,
              B: FnMut(&'a Record) -> bool {
        let mut output: f64 = 0.0;
        let mut len: f64 = 0.0;

        for record in iter {
            len += 1.0;

            if matches(record) {
                output += 1.0;
            }
        }

        if len == 0.0 {
            None

        } else {
            Some(output / len)
        }
    }

    fn iterate_average<'a, A, B>(iter: A, mut f: B) -> Option<f64>
        where A: IntoIterator<Item = &'a Record>,
              B: FnMut(&'a Record) -> f64 {
        let mut output: f64 = 0.0;
        let mut len: f64 = 0.0;

        for record in iter {
            len += 1.0;
            output = output + f(record);
        }

        if len == 0.0 {
            None

        } else {
            Some(output / len)
        }
    }

    fn iterate_geometric<'a, A, B>(iter: A, mut f: B) -> Option<f64>
        where A: IntoIterator<Item = &'a Record>,
              B: FnMut(&'a Record) -> f64 {
        // TODO is this correct ?
        let mut output: f64 = 1.0;
        let mut len: f64 = 0.0;

        for record in iter {
            len += 1.0;
            output = output * f(record);
        }

        if len == 0.0 {
            None

        } else {
            // Calculates the nth root
            Some(output.powf(1.0 / len))
        }
    }


    pub fn needed_odds(iter: &[&Record], name: &str) -> f64 {
        let mut wins = 0.0;
        let mut losses = 0.0;

        for record in iter.iter() {
            // TODO what about mirror matches ?
            if record.is_winner(name) {
                wins += 1.0;

            } else {
                losses += 1.0;
            }
        }

        if wins == 0.0 && losses == 0.0 {
            INFINITY

        } else {
            let needed_odds = 1.0 / (wins / losses);
            (needed_odds * (1.0 + DESIRED_PERCENTAGE_PROFIT))
        }
    }

    pub fn expected_bet_winner(iter: &[&Record], name: &str, max_bet: f64) -> f64 {
        let needed_odds = needed_odds(&iter, name);

        let mut sum = 0.0;
        let mut len = 0.0;

        for record in iter.iter() {
            // TODO handle mirror matches
            match record.winner {
                Winner::Left => if record.left.name == name {
                    sum += ((record.right.bet_amount / needed_odds) - record.left.bet_amount).floor().max(0.0).min(max_bet);
                    len += 1.0;
                },

                Winner::Right => if record.right.name == name {
                    sum += ((record.left.bet_amount / needed_odds) - record.right.bet_amount).floor().max(0.0).min(max_bet);
                    len += 1.0;
                },
            }
        }

        if len == 0.0 {
            0.0

        } else {
            (sum / len).floor().max(0.0).min(max_bet)
        }
    }

    pub fn expected_bet(iter: &[&Record], name: &str, max_bet: f64) -> f64 {
        let needed_odds = needed_odds(&iter, name);

        let mut sum = 0.0;
        let mut len = 0.0;

        // TODO use iterate_average
        for record in iter.iter() {
            len += 1.0;

            // TODO handle mirror matches
            if record.left.name == name {
                sum += ((record.right.bet_amount / needed_odds) - record.left.bet_amount).floor().max(0.0).min(max_bet);

            } else if record.right.name == name {
                sum += ((record.left.bet_amount / needed_odds) - record.right.bet_amount).floor().max(0.0).min(max_bet);
            }
        }

        if len == 0.0 {
            0.0

        } else {
            (sum / len).floor().max(0.0).min(max_bet)
        }
    }


    pub fn winner_upsets<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_percentage(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            match record.winner {
                Winner::Left => record.left.name == name && record.right.bet_amount > (record.left.bet_amount + bet_amount),
                Winner::Right => record.right.name == name && record.left.bet_amount > (record.right.bet_amount + bet_amount),
            }
        // TODO is 0.0 or 0.5 better ?
        }).unwrap_or(0.0)
    }

    pub fn upsets<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_percentage(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            if record.left.name == name {
                record.right.bet_amount > (record.left.bet_amount + bet_amount)

            } else {
                record.left.bet_amount > (record.right.bet_amount + bet_amount)
            }
        // TODO is 0.0 or 0.5 better ?
        }).unwrap_or(0.0)
    }

    pub fn favored<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_percentage(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            if record.left.name == name {
                record.right.bet_amount < (record.left.bet_amount + bet_amount)

            } else {
                record.left.bet_amount < (record.right.bet_amount + bet_amount)
            }
        // TODO is 0.0 or 0.5 better ?
        }).unwrap_or(0.0)
    }

    // TODO bet_amount_winner ?
    pub fn bet_amount<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            if record.left.name == name {
                -record.left.bet_amount

            } else {
                -record.right.bet_amount
            }
        }).unwrap_or(0.0)
    }

    pub fn duration<'a, A>(iter: A) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| record.duration as f64).unwrap_or(0.0)
    }

    pub fn bettors<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            let record = if record.left.name == name { &record.left } else { &record.right };
            -(record.illuminati_bettors + record.normal_bettors) as f64
        }).unwrap_or(0.0)
    }

    pub fn illuminati_bettors<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            let record = if record.left.name == name { &record.left } else { &record.right };
            -record.illuminati_bettors as f64
        }).unwrap_or(0.0)
    }

    pub fn normal_bettors<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            let record = if record.left.name == name { &record.left } else { &record.right };
            -record.normal_bettors as f64
        }).unwrap_or(0.0)
    }

    pub fn wins<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        // TODO what about mirror matches ?
        // TODO is 0.0 or 0.5 better ?
        iterate_percentage(iter, |record| record.is_winner(name)).unwrap_or(0.0)
    }

    pub fn losses<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        // TODO what about mirror matches ?
        // TODO is 0.0 or 0.5 better ?
        iterate_percentage(iter, |record| !record.is_winner(name)).unwrap_or(0.0)
    }

    pub fn bet<'a, A>(iter: A, name: &str, max_bet: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            if record.left.name == name {
                (record.right.bet_amount - record.left.bet_amount).max(0.0).min(max_bet)

            } else {
                (record.left.bet_amount - record.right.bet_amount).max(0.0).min(max_bet)
            }
        }).unwrap_or(0.0)
    }

    pub fn winner_bet<'a, A>(iter: A, name: &str, max_bet: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            match record.winner {
                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                Winner::Left => if record.left.name == name {
                    (record.right.bet_amount - record.left.bet_amount).max(0.0).min(max_bet)

                } else {
                    0.0
                },

                // TODO better detection for whether the character matches or not
                Winner::Right => if record.right.name == name {
                    (record.left.bet_amount - record.right.bet_amount).max(0.0).min(max_bet)

                } else {
                    0.0
                },
            }
        }).unwrap_or(0.0)
    }


    // TODO use iterate_geometric ?
    pub fn odds<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            if record.left.name == name {
                record.right.bet_amount / (record.left.bet_amount + bet_amount)

            } else {
                record.left.bet_amount / (record.right.bet_amount + bet_amount)
            }
        // TODO should this return 1.0 instead ?
        }).unwrap_or(0.0)
    }

    pub fn winner_odds<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            match record.winner {
                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                Winner::Left => if record.left.name == name {
                    record.right.bet_amount / (record.left.bet_amount + bet_amount)

                } else {
                    -1.0
                },

                // TODO better detection for whether the character matches or not
                Winner::Right => if record.right.name == name {
                    record.left.bet_amount / (record.right.bet_amount + bet_amount)

                } else {
                    -1.0
                },
            }
        // The `max` is so that it won't bet if they're both negative
        }).unwrap_or(0.0).max(0.0)
    }

    pub fn odds_difference(iter: &[&Record], name: &str, bet_amount: f64) -> f64 {
        let odds = odds(iter.into_iter().map(|x| *x), name, bet_amount);
        let needed = needed_odds(iter, name);
        (odds - needed).max(0.0)
    }

    pub fn earnings<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        let mut earnings: f64 = 0.0;
        let mut len: f64 = 0.0;

        for record in iter {
            // TODO is this correct ?
            if record.left.name != record.right.name {
                len += 1.0;

                match record.winner {
                    // TODO better detection for whether the character matches or not
                    Winner::Left => if record.left.name == name {
                        earnings += (bet_amount * (record.right.bet_amount / (record.left.bet_amount + bet_amount))).ceil();

                    } else {
                        earnings -= bet_amount;
                    },

                    // TODO better detection for whether the character matches or not
                    Winner::Right => if record.right.name == name {
                        earnings += (bet_amount * (record.left.bet_amount / (record.right.bet_amount + bet_amount))).ceil();

                    } else {
                        earnings -= bet_amount;
                    },
                }
            }
        }

        if len == 0.0 {
            earnings

        } else {
            // The `max` is so that it won't bet if they're both negative
            // TODO is this round a good idea ?
            (earnings / len).round().max(0.0)
        }
    }

    pub fn matches_len<'a, A>(iter: A) -> f64
        where A: IntoIterator<Item = &'a Record> {
        let mut len: f64 = 0.0;

        for _ in iter {
            len += 1.0;
        }

        len
    }
}


impl LookupStatistic {
    fn lookup<'a, A>(&self, name: &str, iter: A) -> f64
        where A: Iterator<Item = &'a Record> {
        match *self {
            // TODO this is wrong
            LookupStatistic::Upsets => lookup::upsets(iter, name, 0.0),
            // TODO this is wrong
            LookupStatistic::Favored => lookup::favored(iter, name, 0.0),
            LookupStatistic::Winrate => lookup::wins(iter, name),
            // TODO this is wrong
            LookupStatistic::Earnings => lookup::earnings(iter, name, 0.0),
            // TODO this is wrong
            LookupStatistic::Odds => lookup::odds(iter, name, 0.0),
            LookupStatistic::BetAmount => lookup::bet_amount(iter, name),
            LookupStatistic::Duration => lookup::duration(iter),
            LookupStatistic::MatchesLen => lookup::matches_len(iter),
        }
    }
}

impl Gene for LookupStatistic {
    fn new() -> Self {
        let rand = gen_rand_index(8u32);

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

        } else if rand == 5 {
            LookupStatistic::MatchesLen

        } else if rand == 6 {
            LookupStatistic::BetAmount

        } else {
            LookupStatistic::Duration
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


impl LookupFilter {
    fn lookup(&self, stat: &LookupStatistic, left: &str, right: &str, matches: Vec<&Record>) -> f64 {
        match *self {
            LookupFilter::All => stat.lookup(left, matches.into_iter()),

            LookupFilter::Specific => stat.lookup(left, matches.into_iter().filter(|record|
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


impl Calculate<f64> for Lookup {
    fn calculate<A: Simulator>(&self, simulation: &A, _tier: &Tier, left: &str, right: &str) -> f64 {
        match *self {
            Lookup::Sum => simulation.current_money(),

            Lookup::Character(ref side, ref filter, ref stat) => match *side {
                LookupSide::Left =>
                    filter.lookup(stat, left, right, simulation.lookup_character(left)),

                LookupSide::Right =>
                    filter.lookup(stat, right, left, simulation.lookup_character(right)),
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



#[derive(Debug, Clone)]
pub struct Simulation<A, B> where A: Strategy, B: Strategy {
    pub matchmaking_strategy: Option<A>,
    pub tournament_strategy: Option<B>,
    pub record_len: f64,
    pub sum: f64,
    pub tournament_sum: f64,
    pub tournament_date: Option<f64>,
    pub in_tournament: bool,
    pub successes: f64,
    pub failures: f64,
    pub max_character_len: usize,
    pub sums: Vec<f64>,
    pub characters: HashMap<String, Vec<Record>>,
}

impl<A, B> Simulation<A, B> where A: Strategy, B: Strategy {
    pub fn new() -> Self {
        Self {
            matchmaking_strategy: None,
            tournament_strategy: None,
            record_len: 0.0,
            sum: SALT_MINE_AMOUNT,
            tournament_sum: TOURNAMENT_BALANCE,
            tournament_date: None,
            in_tournament: false,
            successes: 0.0,
            failures: 0.0,
            max_character_len: 0,
            sums: vec![],
            characters: HashMap::new()
        }
    }

    fn insert_match(&mut self, key: String, record: Record) {
        let matches = self.characters.entry(key).or_insert_with(|| vec![]);

        matches.push(record);

        let len = matches.len();

        if len > self.max_character_len {
            self.max_character_len = len;
        }
    }

    // TODO figure out a way to remove the clones
    pub fn insert_record(&mut self, record: &Record) {
        let left = record.left.name.clone();
        let right = record.right.name.clone();

        if left != right {
            self.record_len += 1.0;
            self.insert_match(left, record.clone());
            self.insert_match(right, record.clone());
        }
    }

    fn sum(&self) -> f64 {
        if self.in_tournament {
            self.tournament_sum

        } else {
            self.sum
        }
    }

    // TODO in exhibitions it should always bet $1
    pub fn pick_winner<C>(&self, strategy: &C, tier: &Tier, left: &str, right: &str) -> Bet where C: Strategy {
        if left != right {
            match strategy.bet(self, tier, left, right) {
                Bet::Left(bet_amount) => return Bet::Left(self.clamp(bet_amount)),
                Bet::Right(bet_amount) => return Bet::Right(self.clamp(bet_amount)),
                Bet::None => {},
            }
        }

        if self.is_in_mines() {
            Bet::Left(self.sum())

            // TODO use randomness
            /*if Gene::new() {
                Bet::Left(self.sum())

            } else {
                Bet::Right(self.sum())
            }*/

        } else if self.sum() >= 1.0 {
            // TODO use randomness
            Bet::Left(1.0)

        } else {
            // TODO use randomness
            Bet::Left(self.sum())
        }
    }

    pub fn bet(&self, record: &Record) -> Bet {
        match record.mode {
            Mode::Matchmaking => match self.matchmaking_strategy {
                Some(ref a) => self.pick_winner(a, &record.tier, &record.left.name, &record.right.name),
                None => Bet::None,
            },
            Mode::Tournament => match self.tournament_strategy {
                Some(ref a) => self.pick_winner(a, &record.tier, &record.left.name, &record.right.name),
                None => Bet::None,
            },
        }
    }

    fn is_tournament_boundary(&self, record: &Record) -> bool {
        self.in_tournament && match record.mode {
            Mode::Matchmaking => true,
            Mode::Tournament => self.tournament_date.map(|date| (record.date - date).abs() > MAX_TOURNAMENT_DURATION).unwrap_or(false),
        }
    }

    pub fn tournament_profit(&self, record: &Record) -> Option<f64> {
        if self.is_tournament_boundary(record) {
            Some(self.tournament_sum)

        } else {
            None
        }
    }

    pub fn calculate(&mut self, record: &Record, bet: &Bet) {
        if self.is_tournament_boundary(record) {
            self.sum += self.tournament_sum;
            self.tournament_sum = TOURNAMENT_BALANCE;
        }

        match record.mode {
            Mode::Matchmaking => {
                self.in_tournament = false;
            },
            Mode::Tournament => {
                self.in_tournament = true;
                // TODO use max ?
                self.tournament_date = Some(record.date);
            },
        }

        let increase = match bet {
            Bet::Left(bet_amount) => match record.winner {
                Winner::Left => {
                    let odds = record.right.bet_amount / (record.left.bet_amount + bet_amount);
                    self.successes += 1.0;
                    (bet_amount * odds).ceil()
                },

                Winner::Right => {
                    self.failures += 1.0;
                    -bet_amount
                },
            },

            Bet::Right(bet_amount) => match record.winner {
                Winner::Right => {
                    let odds = record.left.bet_amount / (record.right.bet_amount + bet_amount);
                    self.successes += 1.0;
                    (bet_amount * odds).ceil()
                },

                Winner::Left => {
                    self.failures += 1.0;
                    -bet_amount
                },
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

    pub fn simulate(&mut self, records: Vec<Record>, insert_records: bool) {
        for record in records.into_iter() {
            // TODO make this more efficient
            let record = record.shuffle();

            let bet = self.bet(&record);

            self.calculate(&record, &bet);

            if insert_records {
                self.insert_record(&record);
            }
        }

        // TODO code duplication
        if self.in_tournament {
            self.in_tournament = false;
            self.sum += self.tournament_sum;
            self.tournament_sum = TOURNAMENT_BALANCE;
        }
    }

    pub fn insert_sum(&mut self, sum: f64) {
        self.sums.push(sum);
    }

    pub fn insert_records<'a, C: IntoIterator<Item = &'a Record>>(&mut self, records: C) {
        for record in records {
            self.insert_record(record);
        }
    }

    pub fn winrate(&self, name: &str) -> f64 {
        lookup::wins(self.lookup_character(name), name)
    }

    pub fn specific_matches_len(&self, left: &str, right: &str) -> usize {
        self.lookup_specific_character(left, right).len()
    }
}

fn average<A: Iterator<Item = f64>>(iter: A, default: f64) -> f64 {
    let mut sum = 0.0;
    let mut len = 0.0;

    for x in iter {
        sum += x;
        len += 1.0;
    }

    if len == 0.0 {
        default

    } else {
        sum / len
    }
}

impl<A, B> Simulator for Simulation<A, B> where A: Strategy, B: Strategy {
    fn average_sum(&self) -> f64 {
        average(self.sums.iter().map(|x| *x), self.current_money())

        /*let len = self.sums.len();

        let index = if len <= 20000 {
            0
        } else {
            len - 20000
        };

        let sums = &self.sums[index..];

        average(sums.into_iter().map(|x| *x), self.current_money())*/
    }

    fn clamp(&self, bet_amount: f64) -> f64 {
        let sum = self.sum();

        if self.is_in_mines() {
            sum

        } else {
            let rounded = bet_amount.round();

            if rounded < 1.0 {
                // Bet $1 for maximum exp
                if sum >= 1.0 {
                    1.0

                } else {
                    sum
                }

            } else if rounded > sum {
                sum

            } else {
                rounded
            }
        }
    }

    fn matches_len(&self, name: &str) -> usize {
        self.lookup_character(name).len()
    }

    fn min_matches_len(&self, left: &str, right: &str) -> f64 {
        // TODO these f64 conversions are a little bit gross
        let left_len = self.matches_len(left) as f64;
        let right_len = self.matches_len(right) as f64;
        left_len.min(right_len)
    }

    fn current_money(&self) -> f64 {
        self.sum()
    }

    fn is_in_mines(&self) -> bool {
        if self.in_tournament {
            self.tournament_sum <= TOURNAMENT_BALANCE

        } else {
            self.sum <= SALT_MINE_AMOUNT
        }
    }

    fn lookup_character(&self, name: &str) -> Vec<&Record> {
        // TODO a bit gross that it returns a Vec and not a &[]
        self.characters.get(name).map(|x| x.into_iter().collect()).unwrap_or(vec![])
    }

    fn lookup_specific_character(&self, left: &str, right: &str) -> Vec<&Record> {
        if left == right {
            self.lookup_character(left).into_iter().filter(|record|
                (record.left.name == right) &&
                (record.right.name == right)).collect()

        } else {
            self.lookup_character(left).into_iter().filter(|record|
                (record.left.name == right) ||
                (record.right.name == right)).collect()
        }
    }
}
