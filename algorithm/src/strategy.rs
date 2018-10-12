use random;
use record::{Tier, Record};
use simulation::{Bet, Simulator, Strategy, lookup, SALT_MINE_AMOUNT};


pub const PERCENTAGE_THRESHOLD: f64 = SALT_MINE_AMOUNT * 100.0;
const MINIMUM_MATCHES_MATCHMAKING: f64 = 15.0;  // minimum match data before it starts betting
const MAXIMUM_MATCHES_MATCHMAKING: f64 = 60.0;  // maximum match data before it reaches the MAXIMUM_BET_PERCENTAGE
const MAXIMUM_WEIGHT: f64 = 5.0;                // maximum percentage for the weight
const MAXIMUM_BET_PERCENTAGE: f64 = 0.05;       // maximum percentage that it will bet (of current money)
//const MAXIMUM_BET_AMOUNT: f64 = 350000.0;       // maximum amount it will bet
const MINIMUM_WINRATE: f64 = 0.10;              // minimum winrate difference before it will bet


const MAGNITUDE: f64 = 5.0;

// TODO is this optimal ?
// TODO use something like round instead ?
// TODO handle negative numbers correctly (https://stackoverflow.com/a/9204760/449477)
fn round_to_order_of_magnitude(input: f64) -> f64 {
    if MAGNITUDE == 2.0 {
        MAGNITUDE.powf(input.log2().trunc())

    } else if MAGNITUDE == 10.0 {
        MAGNITUDE.powf(input.log10().trunc())

    } else {
        MAGNITUDE.powf(input.log(MAGNITUDE).trunc())
    }
}

fn assert_not_nan(x: f64) {
    assert!(!x.is_nan());
}

fn weight_percentage(len: f64, max: f64) -> f64 {
    (len / max).max(0.0).min(1.0)
}

fn weight(percentage: f64, general: f64, specific: f64) -> f64 {
    // TODO is this correct ?
    let general = if percentage == 1.0 {
        0.0
    } else {
        general * (1.0 - percentage)
    };

    // TODO is this correct ?
    let specific = if percentage == 0.0 {
        0.0
    } else {
        specific * percentage
    };

    general + specific
}

fn normalize(value: f64, min: f64, max: f64) -> f64 {
    // TODO is this correct ?
    if min == max {
        0.0

    } else {
        ((value - min) * (1.0 / (max - min))).max(0.0).min(1.0)
    }
}

fn bet_amount<A: Simulator>(simulation: &A, left: &str, right: &str, bet_amount: f64, round_to_magnitude: bool, scale_by_matches: bool) -> f64 {
    let current_money = simulation.current_money();

    if simulation.is_in_mines() {
        current_money

    } else {
        // Bet high when at low money, to try and get out of mines faster
        // When at low money, bet high. When at high money, bet at most MAXIMUM_BET_PERCENTAGE of current money
        // TODO maybe tweak this
        let bet_amount = if current_money < PERCENTAGE_THRESHOLD {
            current_money * (SALT_MINE_AMOUNT / current_money).min(1.0).max(MAXIMUM_BET_PERCENTAGE)

        } else {
            // TODO verify that this is correct
            if round_to_magnitude {
                round_to_order_of_magnitude(bet_amount)

            } else {
                bet_amount
            }
        };

        // Scales it so that when it has more match data it bets higher, and when it has less match data it bets lower
        let bet_amount = if scale_by_matches {
            bet_amount * normalize(simulation.min_matches_len(left, right), MINIMUM_MATCHES_MATCHMAKING - 1.0, MAXIMUM_MATCHES_MATCHMAKING)

        } else {
            bet_amount
        };

        // TODO is this necessary ?
        bet_amount.min(current_money * MAXIMUM_BET_PERCENTAGE)
    }
}

fn weighted<A, F>(simulation: &A, left: &str, right: &str, left_bet: f64, right_bet: f64, mut f: F) -> (f64, f64)
    where A: Simulator,
          F: FnMut(Vec<&Record>, &str, f64) -> f64 {

    let left_general = f(simulation.lookup_character(left), left, left_bet);
    let right_general = f(simulation.lookup_character(right), right, right_bet);

    let specific_matches = simulation.lookup_specific_character(left, right);
    // TODO this f64 conversions is a bit gross
    let specific_matches_percentage = weight_percentage(specific_matches.len() as f64, MAXIMUM_WEIGHT);

    // TODO gross, figure out how to avoid the clone
    let left_specific = f(specific_matches.clone(), left, left_bet);
    let right_specific = f(specific_matches, right, right_bet);

    // Scales it so that as it collects more matchup-specific data it favors the matchup-specific data more
    (
        weight(specific_matches_percentage, left_general, left_specific),
        weight(specific_matches_percentage, right_general, right_specific),
    )
}

pub fn winrates<A>(simulation: &A, left: &str, right: &str) -> (f64, f64) where A: Simulator {
    weighted(simulation, left, right, 0.0, 0.0, |records, name, _bet| lookup::wins(records, name))
}

pub fn average_odds<A>(simulation: &A, left: &str, right: &str, left_bet: f64, right_bet: f64) -> (f64, f64) where A: Simulator {
    weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::odds(records, name, bet))
}

pub fn needed_odds<A>(simulation: &A, left: &str, right: &str) -> (f64, f64) where A: Simulator {
    weighted(simulation, left, right, 0.0, 0.0, |records, name, _bet| lookup::needed_odds(&records, name))
}


#[derive(Debug, Clone, Copy)]
pub enum MoneyStrategy {
    ExpectedBetWinner,
    ExpectedBet,
    WinnerBet,
    Percentage,
    AllIn,
}

impl MoneyStrategy {
    fn current_money<A: Simulator>(simulation: &A, average_sums: bool) -> f64 {
        let current_money = simulation.current_money();

        if average_sums {
            let average = simulation.average_sum();

            if average > current_money {
                current_money

            } else {
                average
            }

        } else {
            current_money
        }
    }

    fn bet_percentage(current_money: f64) -> f64 {
        current_money * MAXIMUM_BET_PERCENTAGE
    }

    fn bet_amount<A: Simulator>(&self, simulation: &A, left: &str, right: &str, average_sums: bool) -> (f64, f64) {
        let current_money = Self::current_money(simulation, average_sums);
        let percentage = Self::bet_percentage(current_money);

        match self {
            MoneyStrategy::ExpectedBetWinner => weighted(simulation, left, right, percentage, percentage, |records, name, bet| lookup::expected_bet_winner(&records, name, bet)),
            MoneyStrategy::ExpectedBet => weighted(simulation, left, right, percentage, percentage, |records, name, bet| lookup::expected_bet(&records, name, bet)),
            MoneyStrategy::WinnerBet => weighted(simulation, left, right, percentage, percentage, |records, name, bet| lookup::winner_bet(records, name, bet)),
            MoneyStrategy::Percentage => (percentage, percentage),
            MoneyStrategy::AllIn => (current_money, current_money),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub enum BetStrategy {
    ExpectedBetWinner,
    ExpectedBet,
    ExpectedProfit,
    WinnerBet,
    Odds,
    Upsets,
    Wins,
    Losses,
    Left,
    Right,
    Random,
}

impl BetStrategy {
    fn bet_value<A: Simulator>(&self, simulation: &A, left: &str, right: &str, left_bet: f64, right_bet: f64, average_sums: bool) -> (f64, f64) {
        let current_money = MoneyStrategy::current_money(simulation, average_sums);
        let percentage = MoneyStrategy::bet_percentage(current_money);

        match self {
            BetStrategy::ExpectedBetWinner => weighted(simulation, left, right, percentage, percentage, |records, name, bet| lookup::expected_bet_winner(&records, name, bet)),
            BetStrategy::ExpectedBet => weighted(simulation, left, right, percentage, percentage, |records, name, bet| lookup::expected_bet(&records, name, bet)),
            BetStrategy::ExpectedProfit => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::earnings(records, name, bet)),
            BetStrategy::WinnerBet => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::winner_bet(records, name, bet)),
            BetStrategy::Odds => average_odds(simulation, left, right, left_bet, right_bet),
            BetStrategy::Upsets => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::upsets(records, name, bet)),
            BetStrategy::Wins => winrates(simulation, left, right),
            BetStrategy::Losses => weighted(simulation, left, right, left_bet, right_bet, |records, name, _bet| lookup::losses(records, name)),
            BetStrategy::Left => (1.0, 0.0),
            BetStrategy::Right => (0.0, 1.0),
            BetStrategy::Random => if random::bool() {
                (1.0, 0.0)
            } else {
                (0.0, 1.0)
            },
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct CustomStrategy {
    pub average_sums: bool,
    pub scale_by_matches: bool,
    pub round_to_magnitude: bool,
    pub money: MoneyStrategy,
    pub bet: BetStrategy,
}

impl Strategy for CustomStrategy {
    fn bet_amount<A: Simulator>(&self, simulation: &A, _tier: &Tier, left: &str, right: &str) -> (f64, f64) {
        let (left_bet, right_bet) = self.money.bet_amount(simulation, left, right, self.average_sums);

        // TODO are these needed ?
        let left_bet = left_bet.max(0.0);
        let right_bet = right_bet.max(0.0);

        //let left_bet = left_bet.min(MAXIMUM_BET_AMOUNT);
        //let right_bet = right_bet.min(MAXIMUM_BET_AMOUNT);

        (
            simulation.clamp(bet_amount(simulation, left, right, left_bet, self.round_to_magnitude, self.scale_by_matches)),
            simulation.clamp(bet_amount(simulation, left, right, right_bet, self.round_to_magnitude, self.scale_by_matches)),
        )
    }

    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let (left_bet, right_bet) = self.bet_amount(simulation, tier, left, right);

        assert_not_nan(left_bet);
        assert_not_nan(right_bet);

        let (left_value, right_value) = self.bet.bet_value(simulation, left, right, left_bet, right_bet, self.average_sums);

        assert_not_nan(left_value);
        assert_not_nan(right_value);

        // TODO is this a good idea ?
        /*if left_bet <= 1.0 && right_bet > 1.0 {
            Bet::Right(right_bet)

        // TODO is this a good idea ?
        } else if right_bet <= 1.0 && left_bet > 1.0 {
            Bet::Left(left_bet)

        } else {*/
            if left_value > right_value {
                Bet::Left(left_bet)

            } else if right_value > left_value {
                Bet::Right(right_bet)

            } else {
                Bet::Left(1.0)
            }
        //}
    }
}


#[derive(Debug, Clone, Copy)]
pub struct AllInStrategy;

impl Strategy for AllInStrategy {
    fn bet_amount<A: Simulator>(&self, simulation: &A, _tier: &Tier, _left: &str, _right: &str) -> (f64, f64) {
        let bet_amount = simulation.current_money();
        (bet_amount, bet_amount)
    }

    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        // TODO a tiny bit hacky
        let bet_amount = self.bet_amount(simulation, tier, left, right).0;

        let (left_winrate, right_winrate) = winrates(simulation, left, right);

        assert_not_nan(left_winrate);
        assert_not_nan(right_winrate);

        let diff = (left_winrate - right_winrate).abs();

        // TODO should this be moved into the bet_amount method ?
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
