pub enum Outcome {
    Loss,
    Draw,
    Win,
}

pub struct Match {
    pub opponent: Glicko,
    pub outcome: Outcome,
}


#[derive(Clone, Copy, Debug)]
pub struct Glicko {
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
}

impl Glicko {
    pub fn new() -> Self {
        Self {
            rating: 1500.0,
            deviation: 350.0,
            volatility: 0.06,
        }
    }

    fn rating2(&self) -> f64 {
        (self.rating - 1500.0) / 173.7178
    }

    fn deviation2(&self) -> f64 {
        self.deviation / 173.7178
    }

    pub fn expected_winrate(&self, other: &Self) -> f64 {
        fn g(rd: f64) -> f64 {
            use std::f64::consts::PI;
            let q = 10.0f64.ln() / 400.0;
            (1.0 + (3.0 * q * q) * (rd * rd) / (PI * PI)).sqrt().recip()
        }

        let ld = self.deviation * self.deviation;
        let rd = other.deviation * other.deviation;
        (1.0 + 10.0f64.powf(-(g((ld + rd).sqrt()) * ((self.rating - other.rating) / 400.0)))).recip()
    }

    pub fn rating_interval(&self) -> (f64, f64) {
        (
            self.rating - (1.96 * self.deviation),
            self.rating + (1.96 * self.deviation),
        )
    }

    fn new_deviation(&self, other: &Self, uncertainty: f64) -> f64 {
        350.0.min(((self.deviation * self.deviation) + (uncertainty * uncertainty)).sqrt())
    }


    pub fn new_rating(&self, matches: &[Match], uncertainty: f64) -> Self {

    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expected() {
        let mut x = Glicko::new();
        x.rating = 1400.0;
        x.deviation = 80.0;

        let mut y = Glicko::new();
        y.rating = 1500.0;
        y.deviation = 150.0;

        assert_eq!(x.expected_winrate(y), 0.3759876557136924);
    }
}
