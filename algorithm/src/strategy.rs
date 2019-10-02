use crate::random;
use crate::record::{Tier, Record};
use crate::genetic::NeuralNetwork;
use crate::types::FitnessResult;
use crate::simulation::{Bet, Simulator, Strategy, lookup, SALT_MINE_AMOUNT};


//const MATCHMAKING_STRATEGY: RandomStrategy = RandomStrategy::Left;

pub const MATCHMAKING_STRATEGY: CustomStrategy = CustomStrategy {
    average_sums: false,
    round_to_magnitude: false,
    scale_by_matches: true,
    money: MoneyStrategy::Fixed,
    bet: BetStrategy::UpsetsElo,
};

/*const MATCHMAKING_STRATEGY: EarningsStrategy = EarningsStrategy {
    expected_profit: true,
    winrate: false,
    bet_difference: false,
    winrate_difference: false,
    use_percentages: true,
};*/

pub const TOURNAMENT_STRATEGY: CustomStrategy = CustomStrategy {
    average_sums: false,
    round_to_magnitude: false,
    scale_by_matches: true,
    money: MoneyStrategy::AllIn,
    bet: BetStrategy::UpsetsElo,
};


lazy_static! {
    pub static ref GENETIC_STRATEGY: Box<NeuralNetwork> = {
        let result: FitnessResult<CustomStrategy> = serde_json::from_str(&include_str!("../../strategies/2019-07-30T11.20.23 0 (matchmaking)")).unwrap();
        Box::new(result.creature.bet.unwrap_genetic().clone())
    };
}


pub trait Permutate {
    fn each<F>(f: F) where F: FnMut(Self), Self: Sized;
}

impl Permutate for bool {
    fn each<F>(mut f: F) where F: FnMut(Self) {
        f(true);
        f(false);
    }
}


pub const PERCENTAGE_THRESHOLD: f64 = SALT_MINE_AMOUNT * 100.0;
pub(crate) const FIXED_BET_AMOUNT: f64 = 70_000.0;
const MINIMUM_MATCHES_MATCHMAKING: f64 = 5.0;   // minimum match data before it starts betting
const MAXIMUM_MATCHES_MATCHMAKING: f64 = 50.0;  // maximum match data before it reaches the MAXIMUM_BET_PERCENTAGE
const MAXIMUM_WEIGHT: f64 = 10.0;               // maximum percentage for the weight
const MAXIMUM_BET_PERCENTAGE: f64 = 0.015;      // maximum percentage that it will bet (of current money)
const MINIMUM_BET_AMOUNT: f64 = 50_000.0;       // minimum amount before it will bet
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

fn weight(_percentage: f64, general: f64, _specific: f64) -> f64 {
    general

    /*
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

    general + specific*/
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
            return current_money * (SALT_MINE_AMOUNT / current_money).min(1.0).max(MAXIMUM_BET_PERCENTAGE);

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
        let bet_amount = bet_amount.min(current_money * MAXIMUM_BET_PERCENTAGE);

        /*if current_money > MINIMUM_BET_AMOUNT / MAXIMUM_BET_PERCENTAGE && bet_amount < MINIMUM_BET_AMOUNT {
            0.0

        } else {*/
            bet_amount
        //}
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
    weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::odds(records.into_iter(), name, bet))
}

pub fn needed_odds<A>(simulation: &A, left: &str, right: &str) -> (f64, f64) where A: Simulator {
    weighted(simulation, left, right, 0.0, 0.0, |records, name, _bet| lookup::needed_odds(&records, name))
}

pub fn expected_profits<A>(simulation: &A, left: &str, right: &str, left_bet: f64, right_bet: f64) -> (f64, f64) where A: Simulator {
    weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::earnings(records, name, bet))
}

pub fn bettors<A>(simulation: &A, left: &str, right: &str) -> (f64, f64) where A: Simulator {
    weighted(simulation, left, right, 0.0, 0.0, |records, name, _bet| lookup::bettors(records, name))
}


#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MoneyStrategy {
    ExpectedBetWinner,
    ExpectedBet,
    BetDifference,
    BetDifferenceWinner,
    Percentage,
    Fixed,
    AllIn,
}

impl Permutate for MoneyStrategy {
    fn each<F>(mut f: F) where F: FnMut(Self) {
        f(MoneyStrategy::ExpectedBetWinner);
        f(MoneyStrategy::ExpectedBet);
        f(MoneyStrategy::BetDifference);
        f(MoneyStrategy::BetDifferenceWinner);
        f(MoneyStrategy::Percentage);
        f(MoneyStrategy::Fixed);
        f(MoneyStrategy::AllIn);
    }
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
            MoneyStrategy::ExpectedBetWinner => weighted(simulation, left, right, percentage, percentage, |records, name, bet| simulation.clamp(lookup::expected_bet_winner(&records, name, bet))),
            MoneyStrategy::ExpectedBet => weighted(simulation, left, right, percentage, percentage, |records, name, bet| simulation.clamp(lookup::expected_bet(&records, name, bet))),
            MoneyStrategy::BetDifference => weighted(simulation, left, right, percentage, percentage, |records, name, bet| simulation.clamp(lookup::bet(records, name, bet))),
            MoneyStrategy::BetDifferenceWinner => weighted(simulation, left, right, percentage, percentage, |records, name, bet| simulation.clamp(lookup::winner_bet(records, name, bet))),
            MoneyStrategy::Percentage => (percentage, percentage),
            MoneyStrategy::Fixed => (FIXED_BET_AMOUNT, FIXED_BET_AMOUNT),
            MoneyStrategy::AllIn => (current_money, current_money),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BetStrategy {
    ExpectedBetWinner,
    ExpectedBet,
    ExpectedProfit,
    BetDifference,
    BetDifferenceWinner,
    Odds,
    OddsDifference,
    WinnerOdds,
    Upsets,
    Bettors,
    BettorsRatio,
    IlluminatiBettors,
    NormalBettors,
    BetAmount,
    BetPercentage,
    Wins,
    Losses,
    Left,
    Right,
    Random,
    Elo,
    UpsetsElo,
    Tournament,
    Genetic(Box<NeuralNetwork>),
}

impl Permutate for BetStrategy {
    fn each<F>(mut f: F) where F: FnMut(Self) {
        f(BetStrategy::ExpectedBetWinner);
        f(BetStrategy::ExpectedBet);
        f(BetStrategy::ExpectedProfit);
        f(BetStrategy::BetDifference);
        f(BetStrategy::BetDifferenceWinner);
        f(BetStrategy::Odds);
        f(BetStrategy::OddsDifference);
        f(BetStrategy::WinnerOdds);
        f(BetStrategy::Upsets);
        f(BetStrategy::Bettors);
        f(BetStrategy::BettorsRatio);
        f(BetStrategy::IlluminatiBettors);
        f(BetStrategy::NormalBettors);
        f(BetStrategy::BetAmount);
        f(BetStrategy::BetPercentage);
        f(BetStrategy::Wins);
        f(BetStrategy::Losses);
        f(BetStrategy::Left);
        f(BetStrategy::Right);
        f(BetStrategy::Elo);
        f(BetStrategy::UpsetsElo);
        f(BetStrategy::Tournament);
        //f(BetStrategy::Random);
        f(BetStrategy::Genetic(GENETIC_STRATEGY.clone()));
    }
}

impl BetStrategy {
    fn bet_value<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, left_bet: f64, right_bet: f64, average_sums: bool) -> (f64, f64) {
        let current_money = MoneyStrategy::current_money(simulation, average_sums);
        let percentage = MoneyStrategy::bet_percentage(current_money);

        match self {
            BetStrategy::ExpectedBetWinner => weighted(simulation, left, right, percentage, percentage, |records, name, bet| lookup::expected_bet_winner(&records, name, bet)),
            BetStrategy::ExpectedBet => weighted(simulation, left, right, percentage, percentage, |records, name, bet| lookup::expected_bet(&records, name, bet)),
            BetStrategy::ExpectedProfit => expected_profits(simulation, left, right, left_bet, right_bet),
            BetStrategy::BetDifference => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::bet(records, name, bet)),
            BetStrategy::BetDifferenceWinner => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::winner_bet(records, name, bet)),
            BetStrategy::Odds => average_odds(simulation, left, right, left_bet, right_bet),
            BetStrategy::OddsDifference => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::odds_difference(&records, name, bet)),
            BetStrategy::WinnerOdds => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::winner_odds(records, name, bet)),
            BetStrategy::Upsets => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::upsets(records, name, bet)),
            BetStrategy::Bettors => bettors(simulation, left, right),
            BetStrategy::BettorsRatio => weighted(simulation, left, right, 0.0, 0.0, |records, name, _bet| lookup::bettors_ratio(records, name)),
            BetStrategy::IlluminatiBettors => weighted(simulation, left, right, left_bet, right_bet, |records, name, _bet| lookup::illuminati_bettors(records, name)),
            BetStrategy::NormalBettors => weighted(simulation, left, right, left_bet, right_bet, |records, name, _bet| lookup::normal_bettors(records, name)),
            BetStrategy::BetAmount => weighted(simulation, left, right, left_bet, right_bet, |records, name, _bet| lookup::bet_amount(records, name)),
            BetStrategy::BetPercentage => weighted(simulation, left, right, left_bet, right_bet, |records, name, bet| lookup::bet_percentage(records, name, bet)),
            BetStrategy::Wins => winrates(simulation, left, right),
            BetStrategy::Losses => weighted(simulation, left, right, left_bet, right_bet, |records, name, _bet| lookup::losses(records, name)),
            BetStrategy::Left => (1.0, 0.0),
            BetStrategy::Right => (0.0, 1.0),
            BetStrategy::Random => if random::bool() {
                (1.0, 0.0)
            } else {
                (0.0, 1.0)
            },
            BetStrategy::Elo => (simulation.elo(left).wins.value, simulation.elo(right).wins.value),
            BetStrategy::UpsetsElo => {
                /*{
                    let x: glicko2::GlickoRating = simulation.elo(left).upsets.into();
                    let y: glicko2::GlickoRating = simulation.elo(right).upsets.into();
                    // (simulation.elo(left).upsets.value - simulation.elo(right).upsets.value).abs()
                    console!(log, x.value, y.value, x.deviation, y.deviation, simulation.elo(left).upsets.value, simulation.elo(left).upsets.deviation);
                }*/

                let left = simulation.elo(left).upsets;
                let right = simulation.elo(right).upsets;

                (left.value, right.value)

                /*
                let diff = (left.value - right.value).abs();
                let deviation = left.deviation + right.deviation;

                if diff >= deviation {
                    if left.value > right.value {
                        (1.0, 0.0)

                    } else if right.value > left.value {
                        (0.0, 1.0)

                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                }*/
            },
            BetStrategy::Tournament => {
                let (left_winrate, right_winrate) = winrates(simulation, left, right);

                assert_not_nan(left_winrate);
                assert_not_nan(right_winrate);

                let diff = (left_winrate - right_winrate).abs();

                if !simulation.is_in_mines() && diff < MINIMUM_WINRATE {
                    return (0.0, 0.0);
                }

                if left_winrate > right_winrate {
                    (1.0, 0.0)

                } else if right_winrate > left_winrate {
                    (0.0, 1.0)

                } else {
                    (0.0, 0.0)
                }
            },
            BetStrategy::Genetic(strategy) => {
                let (left, right) = strategy.choose(simulation, tier, left, right, left_bet, right_bet);

                assert!(left >= 0.0 && left <= 1.0);
                assert!(right >= 0.0 && right <= 1.0);

                if left >= 0.5 || right >= 0.5 {
                    (left, right)

                // Don't bet if left and right are less than 0.5
                } else {
                    (0.0, 0.0)
                }
            },
        }
    }

    pub fn unwrap_genetic(&self) -> &NeuralNetwork {
        match self {
            BetStrategy::Genetic(strategy) => strategy,
            _ => unreachable!(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomStrategy {
    pub average_sums: bool,
    pub scale_by_matches: bool,
    pub round_to_magnitude: bool,
    pub money: MoneyStrategy,
    pub bet: BetStrategy,
}

impl Permutate for CustomStrategy {
    fn each<F>(mut f: F) where F: FnMut(Self) {
        Permutate::each(|average_sums| {
            Permutate::each(|scale_by_matches| {
                Permutate::each(|round_to_magnitude| {
                    Permutate::each(|money| {
                        Permutate::each(|bet| {
                            f(Self { average_sums, scale_by_matches, round_to_magnitude, money, bet });
                        });
                    });
                });
            });
        });
    }
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

        // TODO add in a bias so that it will prefer Left unless Right is much greater than Left
        let (left_value, right_value) = self.bet.bet_value(simulation, tier, left, right, left_bet, right_bet, self.average_sums);

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

    // TODO use ELO instead of winrate
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
