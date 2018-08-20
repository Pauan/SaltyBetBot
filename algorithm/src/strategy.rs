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


fn weighted<A: Simulator, F: FnMut(Vec<&Record>, &str) -> f64>(simulation: &A, left: &str, right: &str, mut f: F) -> (f64, f64) {
    let left_earnings = f(simulation.lookup_character(left), left);
    let right_earnings = f(simulation.lookup_character(right), right);

    let specific_matches = simulation.lookup_specific_character(left, right);
    // TODO this f64 conversions is a bit gross
    let specific_matches_len = specific_matches.len() as f64;

    // TODO gross, figure out how to avoid the clone
    let left_specific_earnings = f(specific_matches.clone(), left);
    let right_specific_earnings = f(specific_matches, right);

    // Scales it so that as it collects more matchup-specific data it favors the matchup-specific data more
    (
        weight(specific_matches_len, left_earnings, left_specific_earnings),
        weight(specific_matches_len, right_earnings, right_specific_earnings)
    )
}

pub fn expected_profits<A: Simulator>(simulation: &A, _tier: &Tier, left: &str, right: &str, bet_amount: f64) -> (f64, f64) {
    weighted(simulation, left, right, |records, name| lookup::earnings(records, name, bet_amount))
}

pub fn winrates<A: Simulator>(simulation: &A, _tier: &Tier, left: &str, right: &str) -> (f64, f64) {
    weighted(simulation, left, right, |records, name| lookup::winrate(records, name))
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
    pub fn bet_amount<A: Simulator>(&self, simulation: &A, _tier: &Tier, left: &str, right: &str, minimum_matches: bool) -> f64 {
        let current_money = simulation.current_money();

        if simulation.is_in_mines() {
            current_money

        } else {
            // When at low money, bet high. When at high money, bet at most MAXIMUM_BET_PERCENTAGE of current money
            let bet_amount = (SALT_MINE_AMOUNT / current_money).min(1.0).max(MAXIMUM_BET_PERCENTAGE);

            // Bet high when at low money, to try and get out of mines faster
            if !minimum_matches || current_money < PERCENTAGE_THRESHOLD {
                if self.use_percentages {
                    current_money * bet_amount

                } else {
                    round_to_order_of_magnitude(current_money * bet_amount)
                }

            } else {
                // Scales it so that when it has more match data it bets higher, and when it has less match data it bets lower
                let len = normalize(simulation.min_matches_len(left, right), MINIMUM_MATCHES_MATCHMAKING - 1.0, MAXIMUM_MATCHES_MATCHMAKING);

                // TODO verify that this is correct
                if self.use_percentages {
                    current_money * bet_amount * len

                } else {
                    round_to_order_of_magnitude(current_money * bet_amount) * len
                }
            }
        }
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
