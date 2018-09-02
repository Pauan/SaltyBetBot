use record::{Tier, Record};
use simulation::{Bet, Simulator, Strategy, lookup, SALT_MINE_AMOUNT};


pub const PERCENTAGE_THRESHOLD: f64 = SALT_MINE_AMOUNT * 100.0;
const MINIMUM_MATCHES_MATCHMAKING: f64 = 10.0;  // minimum match data before it starts betting
const MAXIMUM_MATCHES_MATCHMAKING: f64 = 10.0;  // maximum match data before it reaches the MAXIMUM_BET_PERCENTAGE
const MAXIMUM_WEIGHT: f64 = 10.0;               // maximum percentage for the weight
const MAXIMUM_BET_PERCENTAGE: f64 = 0.01;       // maximum percentage that it will bet (of current money)
const MINIMUM_WINRATE: f64 = 0.10;              // minimum winrate difference before it will bet


const MAGNITUDE: f64 = 10.0;

// TODO is this optimal ?
// TODO use something like round instead ?
fn round_to_order_of_magnitude(input: f64) -> f64 {
    MAGNITUDE.powf(input.log10().trunc())
}

fn weight(len: f64, general: f64, specific: f64) -> f64 {
    let weight = (len / MAXIMUM_WEIGHT).max(0.0).min(1.0);

    (general * (1.0 - weight)) + (specific * weight)
}

fn normalize(value: f64, min: f64, max: f64) -> f64 {
    // TODO is this correct ?
    if min == max {
        0.0

    } else {
        ((value - min) * (1.0 / (max - min))).max(0.0).min(1.0)
    }
}


fn weighted<A, F, G>(simulation: &A, left: &str, right: &str, mut general: F, mut specific: G) -> (f64, f64)
    where A: Simulator,
          F: FnMut(Vec<&Record>, &str) -> f64,
          G: FnMut(Vec<&Record>, &str) -> f64 {

    let left_general = general(simulation.lookup_character(left), left);
    let right_general = general(simulation.lookup_character(right), right);

    let specific_matches = simulation.lookup_specific_character(left, right);
    // TODO this f64 conversions is a bit gross
    let specific_matches_len = specific_matches.len() as f64;

    // TODO gross, figure out how to avoid the clone
    let left_specific = specific(specific_matches.clone(), left);
    let right_specific = specific(specific_matches, right);

    // Scales it so that as it collects more matchup-specific data it favors the matchup-specific data more
    (
        weight(specific_matches_len, left_general, left_specific),
        weight(specific_matches_len, right_general, right_specific)
    )
}

pub fn expected_profits<A: Simulator>(simulation: &A, _tier: &Tier, left: &str, right: &str, bet_amount: f64) -> (f64, f64) {
    weighted(simulation, left, right,
        |records, name| lookup::earnings(records, name, bet_amount),
        |records, name| lookup::earnings(records, name, bet_amount))
}

pub fn winrates<A: Simulator>(simulation: &A, _tier: &Tier, left: &str, right: &str) -> (f64, f64) {
    weighted(simulation, left, right,
        |records, name| lookup::winrate(records, name),
        |records, name| lookup::winrate(records, name))
}

pub fn upsets<A: Simulator>(simulation: &A, _tier: &Tier, left: &str, right: &str, bet_amount: f64) -> (f64, f64) {
    weighted(simulation, left, right,
        |records, name| lookup::upsets(records, name, bet_amount),
        |records, name| lookup::upsets(records, name, bet_amount))
}

pub fn odds<A: Simulator>(simulation: &A, _tier: &Tier, left: &str, right: &str, bet_amount: f64) -> (f64, f64) {
    weighted(simulation, left, right,
        |records, name| lookup::odds(records, name, bet_amount),
        |records, name| lookup::odds(records, name, bet_amount))
}


#[derive(Debug, Clone, Copy)]
pub struct HybridStrategy;

impl HybridStrategy {
    pub fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, minimum_matches: bool) -> f64 {
        EarningsStrategy::_bet_amount(simulation, tier, left, right, true, minimum_matches)
    }
}

impl Strategy for HybridStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let bet_amount = self.bet_amount(simulation, tier, left, right, true);

        let (left, right) = weighted(simulation, left, right,
            |records, name| lookup::upsets(records, name, bet_amount),
            |records, name| lookup::upsets(records, name, bet_amount));

        if left > right && left > 0.5 {
            Bet::Left(bet_amount)

        } else if right > left && right > 0.5 {
            Bet::Right(bet_amount)

        } else {
            Bet::Left(1.0)
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub enum UpsetStrategy {
    Percentage,
    Odds,
}

impl UpsetStrategy {
    pub fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, minimum_matches: bool) -> f64 {
        EarningsStrategy::_bet_amount(simulation, tier, left, right, true, minimum_matches)
    }
}

impl Strategy for UpsetStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let bet_amount = self.bet_amount(simulation, tier, left, right, true);

        let (left, right) = match self {
            UpsetStrategy::Percentage => upsets(simulation, tier, left, right, bet_amount),
            UpsetStrategy::Odds => odds(simulation, tier, left, right, bet_amount),
        };

        if left > right {
            Bet::Left(bet_amount)

        } else if right > left {
            Bet::Right(bet_amount)

        } else {
            Bet::Left(1.0)
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub enum WinrateStrategy {
    High,
    Low,
}

impl WinrateStrategy {
    pub fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, minimum_matches: bool) -> f64 {
        EarningsStrategy::_bet_amount(simulation, tier, left, right, true, minimum_matches)
    }
}

impl Strategy for WinrateStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let bet_amount = self.bet_amount(simulation, tier, left, right, true);

        let (left_winrate, right_winrate) = winrates(simulation, tier, left, right);

        match self {
            WinrateStrategy::High => if left_winrate > right_winrate {
                Bet::Left(bet_amount)

            } else if right_winrate > left_winrate {
                Bet::Right(bet_amount)

            } else {
                Bet::Left(1.0)
            },

            WinrateStrategy::Low => if left_winrate < right_winrate {
                Bet::Left(bet_amount)

            } else if right_winrate < left_winrate {
                Bet::Right(bet_amount)

            } else {
                Bet::Left(1.0)
            },
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub enum RandomStrategy {
    Left,
    Right,
}

impl RandomStrategy {
    pub fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, minimum_matches: bool) -> f64 {
        EarningsStrategy::_bet_amount(simulation, tier, left, right, true, minimum_matches)
    }
}

impl Strategy for RandomStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let bet_amount = self.bet_amount(simulation, tier, left, right, true);

        match self {
            RandomStrategy::Left => Bet::Left(bet_amount),
            RandomStrategy::Right => Bet::Right(bet_amount),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct EarningsStrategy {
    pub use_percentages: bool,
    pub expected_profit: bool,
    pub winrate: bool,
    pub winrate_difference: bool,
    pub bet_difference: bool,
}

impl EarningsStrategy {
    // TODO better behavior for this ?
    fn _bet_amount<A: Simulator>(simulation: &A, _tier: &Tier, left: &str, right: &str, use_percentages: bool, minimum_matches: bool) -> f64 {
        let current_money = simulation.current_money();

        if simulation.is_in_mines() {
            current_money

        } else {
            // When at low money, bet high. When at high money, bet at most MAXIMUM_BET_PERCENTAGE of current money
            let bet_amount = (SALT_MINE_AMOUNT / current_money).min(1.0).max(MAXIMUM_BET_PERCENTAGE);

            // Bet high when at low money, to try and get out of mines faster
            let len = if !minimum_matches || current_money < PERCENTAGE_THRESHOLD {
                1.0

            } else {
                // Scales it so that when it has more match data it bets higher, and when it has less match data it bets lower
                normalize(simulation.min_matches_len(left, right), MINIMUM_MATCHES_MATCHMAKING - 1.0, MAXIMUM_MATCHES_MATCHMAKING)
            };

            // TODO verify that this is correct
            if use_percentages {
                current_money * bet_amount * len

            } else {
                round_to_order_of_magnitude(current_money * bet_amount) * len
            }
        }
    }

    pub fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, minimum_matches: bool) -> f64 {
        Self::_bet_amount(simulation, tier, left, right, self.use_percentages, minimum_matches)
    }
}

impl Strategy for EarningsStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let bet_amount = self.bet_amount(simulation, tier, left, right, true);

        let (left_earnings, right_earnings) = expected_profits(simulation, tier, left, right, bet_amount);
        let (left_winrate, right_winrate) = winrates(simulation, tier, left, right);

        let diff = (left_winrate - right_winrate).abs();

        let bet_amount = if simulation.is_in_mines() {
            simulation.current_money()

        } else if self.winrate_difference && diff < MINIMUM_WINRATE {
            0.0

        } else if self.bet_difference {
            // TODO use the fixed point of expected_profits(diff) for this ?
            (left_earnings - right_earnings).abs().min(bet_amount)

        } else {
            bet_amount
        };

        // Bet $1 for maximum exp
        let bet_amount = bet_amount.max(1.0);

        if (if self.expected_profit { left_earnings > right_earnings } else { true }) &&
           (if self.winrate { left_winrate > right_winrate } else { true }) {
            Bet::Left(bet_amount)

        } else if (if self.expected_profit { right_earnings > left_earnings } else { true }) &&
                  (if self.winrate { right_winrate > left_winrate } else { true }) {
            Bet::Right(bet_amount)

        } else {
            Bet::Left(bet_amount)
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct AllInStrategy;

impl Strategy for AllInStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let bet_amount = simulation.current_money();

        let (left_winrate, right_winrate) = winrates(simulation, tier, left, right);

        let diff = (left_winrate - right_winrate).abs();

        let bet_amount = if simulation.is_in_mines() {
            bet_amount

        } else if diff < MINIMUM_WINRATE {
            0.0

        } else {
            bet_amount
        };

        // Bet $1 for maximum exp
        let bet_amount = bet_amount.max(1.0);

        //let diff = (left_winrate - right_winrate).abs();

        /*if !simulation.is_in_mines() {
            bet_amount = bet_amount * normalize(diff, 0.0, 0.50);
        }*/

        if left_winrate > right_winrate {
            Bet::Left(bet_amount)

        } else if right_winrate > left_winrate {
            Bet::Right(bet_amount)

        } else {
            Bet::Left(bet_amount)
        }
    }
}
