use std;
use std::collections::{ HashMap, HashSet };
use crate::record::{ Record, Mode, Winner, Tier };
use crate::genetic::{ Gene, gen_rand_index, rand_is_percent, MUTATION_RATE, choose2 };
use crate::types::{Lookup, LookupSide, LookupFilter, LookupStatistic};
use crate::strategy::normalize;
use chrono::{Utc, TimeZone, Timelike};


// Number of people using the bot (including self)
pub const NUMBER_OF_BOTS: f64 = 10.0;

// TODO this should take into account the user's real limit
pub const SALT_MINE_AMOUNT: f64 = 4100.0;

// TODO this should take into account the user's real limit
pub const TOURNAMENT_BALANCE: f64 = 4100.0;

// The percentage of profit per match that `expected_bet` should try to get
const DESIRED_PERCENTAGE_PROFIT: f64 = 0.10;

// ~7.7 minutes
pub const NORMAL_MATCH_TIME: f64 = 1000.0 * (60.0 + (80.0 * 5.0));

// TODO
//const MAX_EXHIBITS_DURATION: f64 = NORMAL_MATCH_TIME * 1.0;

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
    fn get_hourly_ratio(&self, date: f64) -> f64;
    fn elo(&self, name: &str, tier: Tier) -> Elo;
    fn average_sum(&self) -> f64;
    fn clamp(&self, bet_amount: f64) -> f64;
    fn matches_len(&self, name: &str, tier: Tier) -> usize;
    fn min_matches_len(&self, left: &str, right: &str, tier: Tier) -> f64;
    fn current_money(&self) -> f64;
    fn is_in_mines(&self) -> bool;
    fn lookup_character(&self, name: &str, tier: Tier) -> Vec<&Record>;
    fn lookup_specific_character(&self, left: &str, right: &str, tier: Tier) -> Vec<&Record>;
}


pub trait Strategy: Sized + std::fmt::Debug {
    fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, date: f64) -> (f64, f64);
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, date: f64) -> Bet;
}


impl Strategy for () {
    fn bet_amount<A: Simulator>(&self, _simulation: &A, _tier: &Tier, _left: &str, _right: &str, _date: f64) -> (f64, f64) {
        (0.0, 0.0)
    }

    fn bet<A: Simulator>(&self, _simulation: &A, _tier: &Tier, _left: &str, _right: &str, _date: f64) -> Bet {
        Bet::None
    }
}


pub trait Calculate<A> {
    fn calculate<B: Simulator>(&self, simulator: &B, tier: &Tier, left: &str, right: &str) -> A;

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
    use crate::record::{Record, Winner, Character};


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
              B: FnMut(&'a Record) -> Option<f64> {
        // TODO is this correct ?
        let mut output: f64 = 1.0;
        let mut len: f64 = 0.0;

        for record in iter {
            if let Some(add) = f(record) {
                len += 1.0;
                output = output * add;
            }
        }

        if len == 0.0 {
            None

        } else {
            // Calculates the nth root
            Some(output.powf(1.0 / len))
        }
    }

    fn choose_map<A, F>(record: &Record, name: &str, mut f: F) -> (A, A) where F: FnMut(&Character) -> A {
        // TODO what about mirror matches ?
        // TODO better detection for whether the character matches or not
        if record.left.name == name {
            (f(&record.left), f(&record.right))
        } else {
            (f(&record.right), f(&record.left))
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

    pub fn bet_percentage<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            let total = record.left.bet_amount + record.right.bet_amount + bet_amount;

            if total == 0.0 {
                0.0

            } else {
                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                let record = if record.left.name == name { &record.left } else { &record.right };

                -((record.bet_amount + bet_amount) / total)
            }
        }).unwrap_or(0.0)
    }

    pub fn bettors<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            let total = record.left.bettors() + record.right.bettors();

            if total == 0.0 {
                0.0

            } else {
                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                let bettors = if record.left.name == name {
                    record.left.bettors()
                } else {
                    record.right.bettors()
                };

                -(bettors / total)
            }
        }).unwrap_or(0.0)
    }

    pub fn bettors_ratio<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_geometric(iter, |record| {
            let (pick, other) = choose_map(&record, name, |x| x.bettors());

            if pick == 0.0 || other == 0.0 {
                None

            } else {
                Some(other / pick)
            }
        }).unwrap_or(0.0)
    }

    pub fn illuminati_bettors<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            let total = record.left.illuminati_bettors + record.right.illuminati_bettors;

            if total == 0.0 {
                0.0

            } else {
                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                let record = if record.left.name == name { &record.left } else { &record.right };

                -(record.illuminati_bettors / total)
            }
        }).unwrap_or(0.0)
    }

    pub fn normal_bettors<'a, A>(iter: A, name: &str) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_average(iter, |record| {
            let total = record.left.normal_bettors + record.right.normal_bettors;

            if total == 0.0 {
                0.0

            } else {
                // TODO what about mirror matches ?
                // TODO better detection for whether the character matches or not
                let record = if record.left.name == name { &record.left } else { &record.right };

                -(record.normal_bettors / total)
            }
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


    pub fn odds<'a, A>(iter: A, name: &str, bet_amount: f64) -> f64
        where A: IntoIterator<Item = &'a Record> {
        iterate_geometric(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            let (pick, other) = if record.left.name == name {
                ((record.left.bet_amount + bet_amount), record.right.bet_amount)

            } else {
                ((record.right.bet_amount + bet_amount), record.left.bet_amount)
            };

            if pick == 0.0 || other == 0.0 {
                None

            } else {
                Some(other / pick)
            }
        }).unwrap_or(0.0)

        /*iterate_average(iter, |record| {
            // TODO what about mirror matches ?
            // TODO better detection for whether the character matches or not
            if record.left.name == name {
                record.right.bet_amount / (record.left.bet_amount + bet_amount)

            } else {
                record.left.bet_amount / (record.right.bet_amount + bet_amount)
            }
        // TODO should this return 1.0 instead ?
        }).unwrap_or(0.0)*/
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
    fn calculate<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> f64 {
        match *self {
            Lookup::Sum => simulation.current_money(),

            Lookup::Character(ref side, ref filter, ref stat) => match *side {
                LookupSide::Left =>
                    filter.lookup(stat, left, right, simulation.lookup_character(left, *tier)),

                LookupSide::Right =>
                    filter.lookup(stat, right, left, simulation.lookup_character(right, *tier)),
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



#[derive(Debug, Clone, Copy)]
pub struct Elo {
    pub wins: glicko2::Glicko2Rating,
    pub upsets: glicko2::Glicko2Rating,
}

impl Elo {
    fn new() -> Self {
        Self {
            wins: glicko2::Glicko2Rating::unrated(),
            upsets: glicko2::Glicko2Rating::unrated(),
        }
    }

    fn update(&mut self, won: bool, opponent: &Elo, left: &crate::record::Character, right: &crate::record::Character) {
        const SYS_CONSTANT: f64 = 0.2;

        self.wins = glicko2::new_rating(
            self.wins,
            &[if won {
                glicko2::GameResult::win(opponent.wins)
            } else {
                glicko2::GameResult::loss(opponent.wins)
            }],
            SYS_CONSTANT,
        );

        self.upsets = glicko2::new_rating(
            self.upsets,
            &[if won {
                if left.average_bet_amount() < right.average_bet_amount() {
                //if left.normal_bettors < right.normal_bettors {
                //if (left.bet_amount + FIXED_BET_AMOUNT) < right.bet_amount {
                    glicko2::GameResult::win(opponent.upsets)
                } else {
                    glicko2::GameResult::draw(opponent.upsets)
                }
            } else {
                if left.average_bet_amount() > right.average_bet_amount() {
                //if left.normal_bettors > right.normal_bettors {
                //if left.bet_amount > (right.bet_amount + FIXED_BET_AMOUNT) {
                    glicko2::GameResult::loss(opponent.upsets)
                } else {
                    glicko2::GameResult::draw(opponent.upsets)
                }
            }],
            SYS_CONSTANT,
        );
    }
}


#[derive(Debug, Clone)]
pub struct CharacterTier {
    characters: HashMap<String, Character>,

}

impl CharacterTier {
    fn new() -> Self {
        Self {
            characters: HashMap::new(),
        }
    }
}


#[derive(Debug, Clone)]
pub struct Character {
    elo: Elo,
    matches: Vec<usize>,
}

impl Character {
    fn new() -> Self {
        Self {
            elo: Elo::new(),
            matches: vec![],
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
    pub upsets: f64,
    pub max_character_len: usize,
    pub sums: Vec<f64>,
    pub records: Vec<Record>,

    // TODO make this faster, e.g. using BTreeMap or an Array ?
    pub tiers: HashMap<Tier, CharacterTier>,

    // TODO is u32 correct ?
    pub bettors_by_hour: [u32; 24],
}

impl<A, B> Simulation<A, B> where A: Strategy, B: Strategy {
    pub fn new(records: Vec<Record>) -> Self {
        let mut this = Self {
            matchmaking_strategy: None,
            tournament_strategy: None,
            record_len: 0.0,
            sum: SALT_MINE_AMOUNT,
            tournament_sum: TOURNAMENT_BALANCE,
            tournament_date: None,
            in_tournament: false,
            successes: 0.0,
            failures: 0.0,
            upsets: 0.0,
            max_character_len: 0,
            sums: vec![],
            records: vec![],
            tiers: HashMap::new(),
            bettors_by_hour: [0; 24],
        };

        for (index, record) in records.iter().enumerate() {
            if let Mode::Matchmaking = record.mode {
                this.insert_sum(record.sum);
            }

            this.insert_record_raw(record, index);
        }

        this.records = records;

        this
    }

    fn insert_match<F>(&mut self, name: String, record: &Record, index: usize, update: F)
        where F: FnOnce(&mut Elo) {

        let tier = self.tiers.entry(record.tier).or_insert_with(CharacterTier::new);

        let character = tier.characters.entry(name).or_insert_with(Character::new);

        update(&mut character.elo);

        character.matches.push(index);

        let len = character.matches.len();

        // TODO this is wrong
        if len > self.max_character_len {
            self.max_character_len = len;
        }
    }

    fn insert_record_raw(&mut self, record: &Record, index: usize) {
        {
            let date = Utc.timestamp_millis(record.date as i64);
            let hour = date.hour() as usize;

            // TODO is this `as u32` correct ?
            self.bettors_by_hour[hour] += (record.left.bettors() + record.right.bettors()) as u32;
        }


        // TODO figure out a way to avoid these clones somehow ?
        let left = record.left.name.clone();
        let right = record.right.name.clone();

        if left != right {
            self.record_len += 1.0;

            let left_elo = self.elo(&left, record.tier);
            let right_elo = self.elo(&right, record.tier);

            self.insert_match(left, &record, index, |elo| {
                elo.update(record.winner == Winner::Left, &right_elo, &record.left, &record.right)
            });

            self.insert_match(right, &record, index, |elo| {
                elo.update(record.winner == Winner::Right, &left_elo, &record.right, &record.left)
            });
        }
    }

    pub fn insert_record(&mut self, record: Record) {
        let index = self.records.len();
        self.insert_record_raw(&record, index);
        self.records.push(record);
    }

    fn sum(&self) -> f64 {
        if self.in_tournament {
            self.tournament_sum

        } else {
            self.sum
        }
    }

    // TODO in exhibitions it should always bet $1
    pub fn pick_winner<C>(&self, strategy: &C, tier: &Tier, left: &str, right: &str, date: f64) -> Bet where C: Strategy {
        if left != right {
            match strategy.bet(self, tier, left, right, date) {
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
                Some(ref a) => self.pick_winner(a, &record.tier, &record.left.name, &record.right.name, record.date),
                None => Bet::None,
            },
            Mode::Tournament => match self.tournament_strategy {
                Some(ref a) => self.pick_winner(a, &record.tier, &record.left.name, &record.right.name, record.date),
                None => Bet::None,
            },
            Mode::Exhibitions => {
                Bet::Left(1.0)
            },
        }
    }

    fn is_tournament_boundary(&self, record: &Record) -> bool {
        self.in_tournament && match record.mode {
            Mode::Matchmaking | Mode::Exhibitions => true,
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

    pub fn skip(&mut self, record: &Record) {
        match record.mode {
            Mode::Matchmaking | Mode::Exhibitions => {
                self.in_tournament = false;
            },
            Mode::Tournament => {
                self.in_tournament = true;
                // TODO use max ?
                self.tournament_date = Some(record.date);
            },
        }
    }

    pub fn calculate(&mut self, record: &Record, bet: &Bet, number_of_bots: f64) {
        if self.is_tournament_boundary(record) {
            self.sum += self.tournament_sum;
            self.tournament_sum = TOURNAMENT_BALANCE;
        }

        self.skip(record);

        let increase = match bet {
            Bet::Left(bet_amount) => match record.winner {
                Winner::Left => {
                    let odds = record.right.bet_amount / (record.left.bet_amount + (bet_amount * number_of_bots));
                    self.successes += 1.0;

                    if odds > 1.0 {
                        self.upsets += 1.0;
                    }

                    (bet_amount * odds).ceil()
                },

                Winner::Right => {
                    self.failures += 1.0;
                    -bet_amount
                },
            },

            Bet::Right(bet_amount) => match record.winner {
                Winner::Right => {
                    let odds = record.left.bet_amount / (record.right.bet_amount + (bet_amount * number_of_bots));
                    self.successes += 1.0;

                    if odds > 1.0 {
                        self.upsets += 1.0;
                    }

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

    pub fn simulate(&mut self, records: Vec<Record>, insert_records: bool, number_of_bots: f64) {
        for record in records.into_iter() {
            // TODO make this more efficient
            let record = record.shuffle();

            let bet = self.bet(&record);

            self.calculate(&record, &bet, number_of_bots);

            if insert_records {
                self.insert_record(record);
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

    pub fn winrate(&self, name: &str, tier: Tier) -> f64 {
        lookup::wins(self.lookup_character(name, tier), name)
    }

    pub fn specific_matches_len(&self, left: &str, right: &str, tier: Tier) -> usize {
        self.lookup_specific_character(left, right, tier).len()
    }

    pub fn characters_len(&self) -> usize {
        let mut seen: HashSet<&str> = HashSet::new();

        for tier in self.tiers.values() {
            for name in tier.characters.keys() {
                seen.insert(name);
            }
        }

        seen.len()
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
    fn get_hourly_ratio(&self, date: f64) -> f64 {
        let date = Utc.timestamp_millis(date as i64);
        let hour = date.hour() as usize;

        let hourly = self.bettors_by_hour[hour];
        // TODO make this more efficient ?
        let min = self.bettors_by_hour.iter().min().unwrap();
        let max = self.bettors_by_hour.iter().max().unwrap();
        normalize(hourly as f64, *min as f64, *max as f64)
    }

    fn elo(&self, name: &str, tier: Tier) -> Elo {
        match self.tiers.get(&tier) {
            Some(x) => match x.characters.get(name) {
                Some(x) => x.elo,
                None => Elo::new()
            },
            None => Elo::new(),
        }
    }

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

    fn matches_len(&self, name: &str, tier: Tier) -> usize {
        self.lookup_character(name, tier).len()
    }

    fn min_matches_len(&self, left: &str, right: &str, tier: Tier) -> f64 {
        // TODO these f64 conversions are a little bit gross
        let left_len = self.matches_len(left, tier) as f64;
        let right_len = self.matches_len(right, tier) as f64;
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

    fn lookup_character(&self, name: &str, tier: Tier) -> Vec<&Record> {
        // TODO a bit gross that it returns a Vec and not a &[]
        self.tiers.get(&tier).and_then(|x| {
            x.characters.get(name).map(|x| {
                x.matches.iter().map(|index| &self.records[*index]).collect()
            })
        }).unwrap_or(vec![])
    }

    fn lookup_specific_character(&self, left: &str, right: &str, tier: Tier) -> Vec<&Record> {
        if left == right {
            self.lookup_character(left, tier).into_iter().filter(|record|
                (record.left.name == right) &&
                (record.right.name == right)).collect()

        } else {
            self.lookup_character(left, tier).into_iter().filter(|record|
                (record.left.name == right) ||
                (record.right.name == right)).collect()
        }
    }
}
