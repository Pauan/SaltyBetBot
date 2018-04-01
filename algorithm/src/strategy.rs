use record::Tier;
use simulation::{Bet, Simulator, Strategy, lookup};


#[derive(Debug, Clone, Copy)]
pub struct EarningsStrategy;

impl EarningsStrategy {
    // TODO better behavior for this ?
    fn bet_amount<A: Simulator>(&self, simulation: &A, _tier: &Tier, left: &str, right: &str) -> f64 {
        // TODO these f64 conversions are a little bit gross
        let left_len = simulation.matches_len(left) as f64;
        let right_len = simulation.matches_len(right) as f64;
        let percentage = (left_len.min(right_len) / 1000.0).min(0.01);
        simulation.current_money() * percentage
    }

    pub fn expected_profits<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> (f64, f64) {
        let bet_amount = self.bet_amount(simulation, tier, left, right);

        (
            lookup::earnings(simulation.lookup_character(left), left, bet_amount),
            lookup::earnings(simulation.lookup_character(right), right, bet_amount)
        )
    }
}

impl Strategy for EarningsStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let bet_amount = self.bet_amount(simulation, tier, left, right);

        let (left_earnings, right_earnings) = self.expected_profits(simulation, tier, left, right);

        // TODO fuzziness
        if left_earnings > right_earnings {
            Bet::Left(bet_amount)

        // TODO fuzziness
        } else if right_earnings > left_earnings {
            Bet::Right(bet_amount)

        } else {
            Bet::None
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct AllInStrategy;

impl AllInStrategy {
    fn calculate_money<A: Simulator>(&self, simulation: &A, _tier: &Tier, _left: &str, _right: &str) -> f64 {
        simulation.current_money()
    }
}

impl Strategy for AllInStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let left_winrate = lookup::winrate(simulation.lookup_character(left), left);
        let right_winrate = lookup::winrate(simulation.lookup_character(right), right);

        // TODO fuzziness
        if left_winrate > right_winrate {
            Bet::Left(self.calculate_money(simulation, tier, left, right))

        // TODO fuzziness
        } else if right_winrate > left_winrate {
            Bet::Right(self.calculate_money(simulation, tier, right, left))

        } else {
            Bet::None
        }
    }
}
