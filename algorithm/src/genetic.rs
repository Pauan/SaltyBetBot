use std;
use arrayvec::ArrayVec;
use rayon::prelude::*;
use crate::random;
use crate::record::{Record, Mode, Tier};
use crate::simulation::{Simulation, Strategy, Bet, Calculate, Simulator, lookup};
use crate::strategy::{CustomStrategy, BetStrategy, MATCHMAKING_STRATEGY};
use crate::types::{FitnessResult, BooleanCalculator, FormulaStrategy, Percentage, NumericCalculator, CubicBezierSegment, Point, Lookup};


//const MAX_BET_AMOUNT: f64 = 1000000.0;
pub const MUTATION_RATE: Percentage = Percentage(0.10);
const MAX_RECURSION_DEPTH: u32 = 2; // 4 maximum nodes


macro_rules! choose_vec {
    ($left:expr, $right:expr) => {
        $left.into_iter().zip($right.into_iter()).map(|(x, y)| x.choose(&y)).collect()
    }
}

/*fn choose_vec<A>(left: &[A], right: &[A]) -> Vec<A> where A: Gene {
    left.into_iter().zip(right.into_iter()).map(|(x, y)| x.choose(&y)).collect()
}*/


const INPUTS: usize = 6;
const LAYERS: usize = 3;

// https://en.wikipedia.org/wiki/Sigmoid_function
#[inline]
fn sigmoid(x: f64) -> f64 {
    //x
    //(0.5 * (x / (1.0 + x.abs()))) + 0.5
    1.0 / (1.0 + (-x).exp())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Node {
    bias: f64,
    weights: ArrayVec<[Percentage; INPUTS]>,
}

impl Node {
    fn calculate(&self, layers: &[Layer], input: &[f64]) -> f64 {
        let sum: f64 = match layers.split_last() {
            Some((layer, rest)) => {
                self.weights.iter().zip(layer.nodes.iter()).map(|(weight, node)| node.calculate(rest, input) * weight.0).sum()
            },
            None => {
                self.weights.iter().zip(input.into_iter()).map(|(weight, input)| input * weight.0).sum()
            },
        };

        sigmoid(sum + self.bias)
    }
}

impl Gene for Node {
    fn new() -> Self {
        Self {
            bias: Gene::new(),
            weights: (0..INPUTS).map(|_| Gene::new()).collect(),
        }
    }

    fn choose(&self, other: &Self) -> Self {
        Self {
            bias: self.bias.choose(&other.bias),
            weights: choose_vec!(&self.weights, &other.weights),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Layer {
    nodes: ArrayVec<[Node; INPUTS]>,
}

impl Gene for Layer {
    fn new() -> Self {
        Self {
            nodes: (0..INPUTS).map(|_| Gene::new()).collect(),
        }
    }

    fn choose(&self, other: &Self) -> Self {
        Self {
            nodes: choose_vec!(&self.nodes, &other.nodes),
        }
    }
}

// TODO regularization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuralNetwork {
    hidden_layers: ArrayVec<[Layer; LAYERS]>,
    output_layer: Node,
}

impl NeuralNetwork {
    fn new() -> Self {
        Self {
            hidden_layers: (0..LAYERS).map(|_| Gene::new()).collect(),
            output_layer: Gene::new(),
        }
    }

    fn breed(&self, other: &Self) -> Self {
        Self {
            hidden_layers: choose_vec!(&self.hidden_layers, &other.hidden_layers),
            output_layer: self.output_layer.choose(&other.output_layer),
        }
    }

    fn calculate(&self, input: &[f64]) -> f64 {
        self.output_layer.calculate(&self.hidden_layers, input)
    }

    pub fn choose<A: Simulator>(&self, simulation: &A, _tier: &Tier, left: &str, right: &str, left_bet: f64, right_bet: f64) -> (f64, f64) {
        let left_matches = simulation.lookup_character(left);
        let right_matches = simulation.lookup_character(right);

        // NeededOdds
        // Odds
        // Duration
        // Bettors
        // Winrate
        // MatchLen
        let left_value = self.calculate(&[
            lookup::needed_odds(&left_matches, left),
            lookup::odds(left_matches.iter().map(|x| *x), left, left_bet),
            lookup::duration(left_matches.iter().map(|x| *x)),
            lookup::bettors(left_matches.iter().map(|x| *x), left),
            lookup::wins(left_matches.iter().map(|x| *x), left),
            left_matches.len() as f64,
        ]);

        let right_value = self.calculate(&[
            lookup::needed_odds(&right_matches, right),
            lookup::odds(right_matches.iter().map(|x| *x), right, right_bet),
            lookup::duration(right_matches.iter().map(|x| *x)),
            lookup::bettors(right_matches.iter().map(|x| *x), right),
            lookup::wins(right_matches.iter().map(|x| *x), right),
            right_matches.len() as f64,
        ]);

        (left_value, right_value)
    }
}

/*impl Strategy for NeuralNetwork {
    fn bet_amount<A: Simulator>(&self, _simulation: &A, _tier: &Tier, _left: &str, _right: &str) -> (f64, f64) {
        (SALT_MINE_AMOUNT, SALT_MINE_AMOUNT)
    }

    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
        let (left_bet, right_bet) = self.bet_amount(simulation, tier, left, right);
        let (left_value, right_value) = self.choose(simulation, tier, left, right, left_bet, right_bet);

        if left_value > right_value {
            Bet::Left(left_bet)

        } else if right_value > left_value {
            Bet::Right(right_bet)

        } else {
            // TODO is this correct ?
            Bet::None
        }
    }
}*/


impl Creature for CustomStrategy {
    fn new() -> Self {
        let mut this = MATCHMAKING_STRATEGY.clone();
        this.bet = BetStrategy::Genetic(Box::new(NeuralNetwork::new()));
        this
    }

    fn breed(&self, other: &Self) -> Self {
        let mut this = MATCHMAKING_STRATEGY.clone();

        // TODO super hacky
        this.bet = BetStrategy::Genetic(Box::new(self.bet.unwrap_genetic().breed(other.bet.unwrap_genetic())));

        this
    }
}


pub trait Creature: Strategy {
    fn new() -> Self;

    fn breed(&self, other: &Self) -> Self;
}


pub trait Gene {
    fn new() -> Self;

    fn choose(&self, other: &Self) -> Self;
}


impl Gene for bool {
    fn new() -> Self {
        random::bool()
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


impl Gene for f64 {
    // TODO verify that this is correct
    fn new() -> Self {
        random::gaussian()

        /*let Percentage(percent) = Gene::new();

        MAX_BET_AMOUNT * ((percent * 2.0) - 1.0)*/
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


/*impl Gene for f32 {
    // TODO verify that this is correct
    fn new() -> Self {
        let Closed01(val) = random::weak_rng().gen::<Closed01<f32>>();
        val
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            choose2(self, other)
        }
    }
}*/


pub fn choose2<A>(left: &A, right: &A) -> A where A: Clone {
    if Gene::new() {
        left.clone()

    } else {
        right.clone()
    }
}


pub fn gen_rand_index(index: u32) -> u32 {
    random::between_exclusive(0, index)
}


// TODO verify that this is correct
pub fn rand_is_percent(Percentage(input): Percentage) -> bool {
    let Percentage(rand) = Gene::new();
    rand <= input
}


pub fn choose<'a, A>(values: &'a [A]) -> Option<&'a A> {
    if values.is_empty() {
        None
    } else {
        Some(&values[random::between_exclusive(0, values.len() as u32) as usize])
    }
}


impl Gene for Percentage {
    fn new() -> Self {
        Percentage(random::percentage())
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


impl Point {
    fn new(x: f64, y: f64) -> Point {
        Point { x: x, y: y }
    }
}

impl Gene for Point {
    fn new() -> Self {
        Self::new(Gene::new(), Gene::new())
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            Self::new(self.x.choose(&other.x), self.y.choose(&other.y))
        }
    }
}


impl CubicBezierSegment {
    // https://docs.rs/lyon_bezier/0.8.5/src/lyon_bezier/cubic_bezier.rs.html#51-61
    // TODO verify that this is correct
    pub fn sample_y(&self, t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let one_t3 = one_t2 * one_t;
        return self.from.y * one_t3 +
            self.ctrl1.y * 3.0 * one_t2 * t +
            self.ctrl2.y * 3.0 * one_t * t2 +
            self.to.y * t3;
    }
}

impl Gene for CubicBezierSegment {
    fn new() -> Self {
        CubicBezierSegment {
            from: Gene::new(),
            ctrl1: Gene::new(),
            ctrl2: Gene::new(),
            to: Gene::new(),
        }
    }

    fn choose(&self, other: &Self) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Gene::new()

        } else {
            CubicBezierSegment {
                from: self.from.choose(&other.from),
                ctrl1: self.ctrl1.choose(&other.ctrl1),
                ctrl2: self.ctrl2.choose(&other.ctrl2),
                to: self.to.choose(&other.to),
            }
        }
    }
}


impl<A> NumericCalculator<A, f64>
    where A: Calculate<f64> + Gene + Clone + PartialEq {
    // TODO auto-derive this
    fn _new(depth: u32) -> Self {
        if depth >= MAX_RECURSION_DEPTH {
            let rand = gen_rand_index(3u32);

            if rand == 0 {
                NumericCalculator::Base(Gene::new())

            } else if rand == 1 {
                NumericCalculator::Fixed(Gene::new())

            } else {
                NumericCalculator::Percentage(Gene::new())
            }

        } else {
            let rand = gen_rand_index(13u32);

            if rand == 0 {
                NumericCalculator::Base(Gene::new())

            } else if rand == 1 {
                NumericCalculator::Fixed(Gene::new())

            } else if rand == 2 {
                NumericCalculator::Percentage(Gene::new())

            } else if rand == 3 {
                NumericCalculator::Bezier(Gene::new(), Box::new(Self::_new(depth + 1)))

            } else if rand == 4 {
                NumericCalculator::Abs(Box::new(Self::_new(depth + 1)))

            } else if rand == 5 {
                NumericCalculator::Average(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 6 {
                NumericCalculator::Min(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 7 {
                NumericCalculator::Max(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 8 {
                NumericCalculator::Plus(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 9 {
                NumericCalculator::Minus(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 10 {
                NumericCalculator::Multiply(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else if rand == 11 {
                NumericCalculator::Divide(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else {
                NumericCalculator::IfThenElse(Gene::new(), Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            }/* else {
                NumericCalculator::Tier {
                    new: Box::new(Self::_new(depth + 1)),
                    x: Box::new(Self::_new(depth + 1)),
                    s: Box::new(Self::_new(depth + 1)),
                    a: Box::new(Self::_new(depth + 1)),
                    b: Box::new(Self::_new(depth + 1)),
                    p: Box::new(Self::_new(depth + 1)),
                }
            }*/
        }
    }

    // TODO verify that this cannot exceed MAX_RECURSION_DEPTH
    // TODO auto-derive this
    fn _choose(&self, other: &Self, depth: u32) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Self::_new(depth)

        } else {
            match *self {
                NumericCalculator::Base(ref father) => match *other {
                    NumericCalculator::Base(ref mother) =>
                        NumericCalculator::Base(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Fixed(ref father) => match *other {
                    NumericCalculator::Fixed(ref mother) =>
                        NumericCalculator::Fixed(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Percentage(father) => match *other {
                    NumericCalculator::Percentage(mother) =>
                        NumericCalculator::Percentage(father.choose(&mother)),
                    _ => choose2(self, other),
                },
                NumericCalculator::Bezier(father1, ref father2) => match *other {
                    NumericCalculator::Bezier(mother1, ref mother2) =>
                        NumericCalculator::Bezier(father1.choose(&mother1), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Abs(ref father) => match *other {
                    NumericCalculator::Abs(ref mother) =>
                        NumericCalculator::Abs(Box::new(father._choose(&mother, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Average(ref father1, ref father2) => match *other {
                    NumericCalculator::Average(ref mother1, ref mother2) =>
                        NumericCalculator::Average(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Min(ref father1, ref father2) => match *other {
                    NumericCalculator::Min(ref mother1, ref mother2) =>
                        NumericCalculator::Min(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Max(ref father1, ref father2) => match *other {
                    NumericCalculator::Max(ref mother1, ref mother2) =>
                        NumericCalculator::Max(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Plus(ref father1, ref father2) => match *other {
                    NumericCalculator::Plus(ref mother1, ref mother2) =>
                        NumericCalculator::Plus(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Minus(ref father1, ref father2) => match *other {
                    NumericCalculator::Minus(ref mother1, ref mother2) =>
                        NumericCalculator::Minus(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Multiply(ref father1, ref father2) => match *other {
                    NumericCalculator::Multiply(ref mother1, ref mother2) =>
                        NumericCalculator::Multiply(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::Divide(ref father1, ref father2) => match *other {
                    NumericCalculator::Divide(ref mother1, ref mother2) =>
                        NumericCalculator::Divide(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                NumericCalculator::IfThenElse(ref father1, ref father2, ref father3) => match *other {
                    // TODO should this pass the depth somehow to father1 and mother1 ?
                    NumericCalculator::IfThenElse(ref mother1, ref mother2, ref mother3) =>
                        NumericCalculator::IfThenElse(father1.choose(&mother1), Box::new(father2._choose(&mother2, depth + 1)), Box::new(father3._choose(&mother3, depth + 1))),
                    _ => choose2(self, other),
                },
                /*NumericCalculator::Tier { new: ref father_new, x: ref father_x, s: ref father_s, a: ref father_a, b: ref father_b, p: ref father_p } => match *other {
                    NumericCalculator::Tier { new: ref mother_new, x: ref mother_x, s: ref mother_s, a: ref mother_a, b: ref mother_b, p: ref mother_p } =>
                        NumericCalculator::Tier {
                            new: Box::new(father_new._choose(&mother_new, depth + 1)),
                            x: Box::new(father_x._choose(&mother_x, depth + 1)),
                            s: Box::new(father_s._choose(&mother_s, depth + 1)),
                            a: Box::new(father_a._choose(&mother_a, depth + 1)),
                            b: Box::new(father_b._choose(&mother_b, depth + 1)),
                            p: Box::new(father_p._choose(&mother_p, depth + 1)),
                        },
                    _ => choose2(self, other),
                },*/
            }
        }
    }

    fn _optimize(self, tier: &Option<Tier>) -> Self {
        if let NumericCalculator::Percentage(_) = self {
            self

        } else {
            match self.precalculate() {
                Some(a) => NumericCalculator::Fixed(a),
                None => match self {
                    NumericCalculator::Base(a) => NumericCalculator::Base(a.optimize()),
                    NumericCalculator::Fixed(_) => self,
                    NumericCalculator::Percentage(_) => self,

                    NumericCalculator::Bezier(a, b) => NumericCalculator::Bezier(a, Box::new(b._optimize(tier))),

                    // TODO Abs can probably be optimized further
                    NumericCalculator::Abs(a) => NumericCalculator::Abs(Box::new(a._optimize(tier))),

                    NumericCalculator::Average(a, b) => NumericCalculator::Average(Box::new(a._optimize(tier)), Box::new(b._optimize(tier))),

                    NumericCalculator::Min(a, b) => {
                        let a = a._optimize(tier);
                        let b = b._optimize(tier);

                        if a == b {
                            a

                        } else {
                            NumericCalculator::Min(Box::new(a), Box::new(b))
                        }
                    },

                    NumericCalculator::Max(a, b) => {
                        let a = a._optimize(tier);
                        let b = b._optimize(tier);

                        if a == b {
                            a

                        } else {
                            NumericCalculator::Max(Box::new(a), Box::new(b))
                        }
                    },

                    // TODO maybe optimize into a * 2.0 when a == b
                    NumericCalculator::Plus(a, b) => NumericCalculator::Plus(Box::new(a._optimize(tier)), Box::new(b._optimize(tier))),

                    NumericCalculator::Minus(a, b) => {
                        let a = a._optimize(tier);
                        let b = b._optimize(tier);

                        // TODO move this into precalculate
                        if a == b {
                            // TODO is this correct ?
                            NumericCalculator::Fixed(0.0)

                        } else {
                            NumericCalculator::Minus(Box::new(a), Box::new(b))
                        }
                    },

                    NumericCalculator::Multiply(a, b) => NumericCalculator::Multiply(Box::new(a._optimize(tier)), Box::new(b._optimize(tier))),
                    NumericCalculator::Divide(a, b) => NumericCalculator::Divide(Box::new(a._optimize(tier)), Box::new(b._optimize(tier))),

                    NumericCalculator::IfThenElse(a, b, c) => match a.precalculate() {
                        Some(a) => if a {
                            b._optimize(tier)

                        } else {
                            c._optimize(tier)
                        },

                        None => {
                            let b = b._optimize(tier);
                            let c = c._optimize(tier);

                            if b == c {
                                b

                            } else {
                                NumericCalculator::IfThenElse(a.optimize(), Box::new(b), Box::new(c))
                            }
                        },
                    },

                    /*NumericCalculator::Tier { new, x, s, a, b, p } => match *tier {
                        Some(Tier::New) => new._optimize(tier),
                        Some(Tier::X) => x._optimize(tier),
                        Some(Tier::S) => s._optimize(tier),
                        Some(Tier::A) => a._optimize(tier),
                        Some(Tier::B) => b._optimize(tier),
                        Some(Tier::P) => p._optimize(tier),

                        None => {
                            let new = new._optimize(&Some(Tier::New));
                            let x = x._optimize(&Some(Tier::X));
                            let s = s._optimize(&Some(Tier::S));
                            let a = a._optimize(&Some(Tier::A));
                            let b = b._optimize(&Some(Tier::B));
                            let p = p._optimize(&Some(Tier::P));

                            if new == x && new == s && new == a && new == b && new == p {
                                new

                            } else {
                                NumericCalculator::Tier {
                                    new: Box::new(new),
                                    x: Box::new(x),
                                    s: Box::new(s),
                                    a: Box::new(a),
                                    b: Box::new(b),
                                    p: Box::new(p),
                                }
                            }
                        }
                    },*/
                },
            }
        }
    }
}

impl<A> Calculate<f64> for NumericCalculator<A, f64>
    where A: Calculate<f64> + Gene + Clone + PartialEq {
    fn optimize(self) -> Self {
        self._optimize(&None)
    }

    fn precalculate(&self) -> Option<f64> {
        match *self {
            NumericCalculator::Base(ref a) => a.precalculate(),

            NumericCalculator::Fixed(a) => Some(a),

            NumericCalculator::Percentage(Percentage(percentage)) => Some(percentage),

            NumericCalculator::Bezier(bezier, ref a) => a.precalculate().map(|a| bezier.sample_y(a)),

            NumericCalculator::Average(ref a, ref b) => a.precalculate().and_then(|a| b.precalculate().map(|b| (a + b) / 2.0)),

            NumericCalculator::Abs(ref a) => a.precalculate().map(|a| a.abs()),
            NumericCalculator::Min(ref a, ref b) => a.precalculate().and_then(|a| b.precalculate().map(|b| a.min(b))),
            NumericCalculator::Max(ref a, ref b) => a.precalculate().and_then(|a| b.precalculate().map(|b| a.max(b))),

            // TODO optimize for certain things, like 0 or 1
            NumericCalculator::Plus(ref a, ref b) => a.precalculate().and_then(|a| b.precalculate().map(|b| a + b)),
            NumericCalculator::Minus(ref a, ref b) => a.precalculate().and_then(|a| b.precalculate().map(|b| a - b)),
            NumericCalculator::Multiply(ref a, ref b) => a.precalculate().and_then(|a| b.precalculate().map(|b| a * b)),
            NumericCalculator::Divide(ref a, ref b) => a.precalculate().and_then(|a| b.precalculate().map(|b| a / b)),

            NumericCalculator::IfThenElse(ref a, ref b, ref c) => match a.precalculate() {
                Some(a) => if a {
                    b.precalculate()

                } else {
                    c.precalculate()
                },

                None => b.precalculate().and_then(|b| c.precalculate().and_then(|c| if b == c {
                    Some(b)

                } else {
                    None
                }))
            },

            /*NumericCalculator::Tier { ref new, ref x, ref s, ref a, ref b, ref p } =>
                new.precalculate().and_then(|new|
                x.precalculate().and_then(|x|
                s.precalculate().and_then(|s|
                a.precalculate().and_then(|a|
                b.precalculate().and_then(|b|
                p.precalculate().and_then(|p|
                    if new == x && new == s && new == a && new == b && new == p {
                        Some(x)

                    } else {
                        None
                    })))))),*/
        }
    }

    fn calculate<B: Simulator>(&self, simulation: &B, tier: &Tier, left: &str, right: &str) -> f64 {
        match *self {
            NumericCalculator::Base(ref a) => a.calculate(simulation, tier, left, right),

            NumericCalculator::Fixed(a) => a,

            NumericCalculator::Percentage(Percentage(percentage)) => percentage,

            NumericCalculator::Bezier(bezier, ref a) => bezier.sample_y(a.calculate(simulation, tier, left, right)),

            NumericCalculator::Abs(ref a) => a.calculate(simulation, tier, left, right).abs(),
            NumericCalculator::Average(ref a, ref b) => (a.calculate(simulation, tier, left, right) + b.calculate(simulation, tier, left, right)) / 2.0,
            NumericCalculator::Min(ref a, ref b) => a.calculate(simulation, tier, left, right).min(b.calculate(simulation, tier, left, right)),
            NumericCalculator::Max(ref a, ref b) => a.calculate(simulation, tier, left, right).max(b.calculate(simulation, tier, left, right)),
            NumericCalculator::Plus(ref a, ref b) => a.calculate(simulation, tier, left, right) + b.calculate(simulation, tier, left, right),
            NumericCalculator::Minus(ref a, ref b) => a.calculate(simulation, tier, left, right) - b.calculate(simulation, tier, left, right),
            NumericCalculator::Multiply(ref a, ref b) => a.calculate(simulation, tier, left, right) * b.calculate(simulation, tier, left, right),
            NumericCalculator::Divide(ref a, ref b) => a.calculate(simulation, tier, left, right) / b.calculate(simulation, tier, left, right),

            NumericCalculator::IfThenElse(ref a, ref b, ref c) => if a.calculate(simulation, tier, left, right) {
                b.calculate(simulation, tier, left, right)
            } else {
                c.calculate(simulation, tier, left, right)
            },

            /*NumericCalculator::Tier { ref new, ref x, ref s, ref a, ref b, ref p } => match *tier {
                Tier::New => new.calculate(simulation, tier, left, right),
                Tier::X => x.calculate(simulation, tier, left, right),
                Tier::S => s.calculate(simulation, tier, left, right),
                Tier::A => a.calculate(simulation, tier, left, right),
                Tier::B => b.calculate(simulation, tier, left, right),
                Tier::P => p.calculate(simulation, tier, left, right),
            },*/
        }
    }
}

impl<A> Gene for NumericCalculator<A, f64>
    where A: Calculate<f64> + Gene + Clone + PartialEq {
    fn new() -> Self {
        NumericCalculator::_new(0).optimize()
    }

    fn choose(&self, other: &Self) -> Self {
        self._choose(other, 0).optimize()
    }
}


impl<A> BooleanCalculator<A> where A: Gene + Clone {
    fn _new(depth: u32) -> Self {
        if depth >= MAX_RECURSION_DEPTH {
            let rand = gen_rand_index(6u32);

            if rand == 0 {
                BooleanCalculator::True

            } else if rand == 1 {
                BooleanCalculator::False

            } else if rand == 2 {
                BooleanCalculator::Greater(Gene::new(), Gene::new())

            } else if rand == 3 {
                BooleanCalculator::GreaterEqual(Gene::new(), Gene::new())

            } else if rand == 4 {
                BooleanCalculator::Lesser(Gene::new(), Gene::new())

            } else {
                BooleanCalculator::LesserEqual(Gene::new(), Gene::new())
            }

        } else {
            let rand = gen_rand_index(8u32);

            if rand == 0 {
                BooleanCalculator::True

            } else if rand == 1 {
                BooleanCalculator::False

            } else if rand == 2 {
                BooleanCalculator::Greater(Gene::new(), Gene::new())

            } else if rand == 3 {
                BooleanCalculator::GreaterEqual(Gene::new(), Gene::new())

            } else if rand == 4 {
                BooleanCalculator::Lesser(Gene::new(), Gene::new())

            } else if rand == 5 {
                BooleanCalculator::LesserEqual(Gene::new(), Gene::new())

            } else if rand == 6 {
                BooleanCalculator::And(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else {
                BooleanCalculator::Or(Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))
            }
        }
    }

    // TODO is this correct ?
    fn _choose(&self, other: &Self, depth: u32) -> Self {
        // Random mutation
        if rand_is_percent(MUTATION_RATE) {
            Self::_new(depth)

        } else {
            match *self {
                BooleanCalculator::Greater(ref father1, ref father2) => match *other {
                    BooleanCalculator::Greater(ref mother1, ref mother2) =>
                        BooleanCalculator::Greater(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::GreaterEqual(ref father1, ref father2) => match *other {
                    BooleanCalculator::GreaterEqual(ref mother1, ref mother2) =>
                        BooleanCalculator::GreaterEqual(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::Lesser(ref father1, ref father2) => match *other {
                    BooleanCalculator::Lesser(ref mother1, ref mother2) =>
                        BooleanCalculator::Lesser(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::LesserEqual(ref father1, ref father2) => match *other {
                    BooleanCalculator::LesserEqual(ref mother1, ref mother2) =>
                        BooleanCalculator::LesserEqual(father1.choose(&mother1), father2.choose(&mother2)),
                    _ => choose2(self, other),
                },
                BooleanCalculator::And(ref father1, ref father2) => match *other {
                    BooleanCalculator::And(ref mother1, ref mother2) =>
                        BooleanCalculator::And(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                BooleanCalculator::Or(ref father1, ref father2) => match *other {
                    BooleanCalculator::Or(ref mother1, ref mother2) =>
                        BooleanCalculator::Or(Box::new(father1._choose(&mother1, depth + 1)), Box::new(father2._choose(&mother2, depth + 1))),
                    _ => choose2(self, other),
                },
                _ => choose2(self, other),
            }
        }
    }
}

impl<A> Calculate<bool> for BooleanCalculator<A>
    where A: Calculate<f64> + PartialEq {
    fn optimize(self) -> Self {
        match self.precalculate() {
            Some(a) => if a {
                BooleanCalculator::True

            } else {
                BooleanCalculator::False
            },

            None => match self {
                BooleanCalculator::True => self,
                BooleanCalculator::False => self,

                BooleanCalculator::Greater(a, b) => BooleanCalculator::Greater(a.optimize(), b.optimize()),
                BooleanCalculator::GreaterEqual(a, b) => BooleanCalculator::GreaterEqual(a.optimize(), b.optimize()),
                BooleanCalculator::Lesser(a, b) => BooleanCalculator::Lesser(a.optimize(), b.optimize()),
                BooleanCalculator::LesserEqual(a, b) => BooleanCalculator::LesserEqual(a.optimize(), b.optimize()),

                BooleanCalculator::And(a, b) => match a.precalculate() {
                    Some(a) => if a {
                        b.optimize()

                    } else {
                        BooleanCalculator::False
                    },

                    None => match b.precalculate() {
                        Some(b) => if b {
                            a.optimize()

                        } else {
                            BooleanCalculator::False
                        },

                        None => BooleanCalculator::And(Box::new(a.optimize()), Box::new(b.optimize())),
                    },
                },

                BooleanCalculator::Or(a, b) => match a.precalculate() {
                    Some(a) => if a {
                        BooleanCalculator::True

                    } else {
                        b.optimize()
                    },

                    None => match b.precalculate() {
                        Some(b) => if b {
                            BooleanCalculator::True

                        } else {
                            a.optimize()
                        },

                        None => BooleanCalculator::Or(Box::new(a.optimize()), Box::new(b.optimize())),
                    },
                }
            },
        }
    }

    fn precalculate(&self) -> Option<bool> {
        match *self {
            BooleanCalculator::True => Some(true),
            BooleanCalculator::False => Some(false),

            BooleanCalculator::Greater(ref a, ref b) => if a == b {
                Some(false)
            } else {
                a.precalculate().and_then(|a| b.precalculate().map(|b| a > b))
            },

            BooleanCalculator::GreaterEqual(ref a, ref b) => if a == b {
                Some(true)
            } else {
                a.precalculate().and_then(|a| b.precalculate().map(|b| a >= b))
            },

            BooleanCalculator::Lesser(ref a, ref b) => if a == b {
                Some(false)
            } else {
                a.precalculate().and_then(|a| b.precalculate().map(|b| a < b))
            },

            BooleanCalculator::LesserEqual(ref a, ref b) => if a == b {
                Some(true)
            } else {
                a.precalculate().and_then(|a| b.precalculate().map(|b| a <= b))
            },

            BooleanCalculator::And(ref a, ref b) => match a.precalculate() {
                Some(a) => if a {
                    b.precalculate()

                } else {
                    Some(false)
                },

                None => b.precalculate().and_then(|b| if b {
                    None

                } else {
                    Some(false)
                }),
            },

            BooleanCalculator::Or(ref a, ref b) => match a.precalculate() {
                Some(a) => if a {
                    Some(true)

                } else {
                    b.precalculate()
                },

                None => b.precalculate().and_then(|b| if b {
                    Some(true)

                } else {
                    None
                }),
            },
        }
    }

    fn calculate<B: Simulator>(&self, simulation: &B, tier: &Tier, left: &str, right: &str) -> bool {
        match *self {
            BooleanCalculator::True => true,
            BooleanCalculator::False => false,
            BooleanCalculator::Greater(ref a, ref b) => a.calculate(simulation, tier, left, right) > b.calculate(simulation, tier, left, right),
            BooleanCalculator::GreaterEqual(ref a, ref b) => a.calculate(simulation, tier, left, right) >= b.calculate(simulation, tier, left, right),
            BooleanCalculator::Lesser(ref a, ref b) => a.calculate(simulation, tier, left, right) < b.calculate(simulation, tier, left, right),
            BooleanCalculator::LesserEqual(ref a, ref b) => a.calculate(simulation, tier, left, right) <= b.calculate(simulation, tier, left, right),
            BooleanCalculator::And(ref a, ref b) => a.calculate(simulation, tier, left, right) && b.calculate(simulation, tier, left, right),
            BooleanCalculator::Or(ref a, ref b) => a.calculate(simulation, tier, left, right) || b.calculate(simulation, tier, left, right),
        }
    }
}

impl<A> Gene for BooleanCalculator<A> where A: Calculate<f64> + Gene + Clone + PartialEq {
    fn new() -> Self {
        Self::_new(0).optimize()
    }

    fn choose(&self, other: &Self) -> Self {
        self._choose(other, 0).optimize()
    }
}


impl Creature for FormulaStrategy {
    fn new() -> Self {
        Self {
            bet_strategy: Gene::new(),
            prediction_strategy: Gene::new(),
            money_strategy: NumericCalculator::Multiply(Box::new(NumericCalculator::Base(Lookup::Sum)), Box::new(NumericCalculator::Percentage(Percentage(0.01)))), //Gene::new(),
        }
    }

    fn breed(&self, other: &Self) -> Self {
        Self {
            bet_strategy: self.bet_strategy.choose(&other.bet_strategy),
            prediction_strategy: self.prediction_strategy.choose(&other.prediction_strategy),
            money_strategy: NumericCalculator::Multiply(Box::new(NumericCalculator::Base(Lookup::Sum)), Box::new(NumericCalculator::Percentage(Percentage(0.01)))), //self.money_strategy.choose(&other.money_strategy),
        }
    }
}

impl Strategy for FormulaStrategy {
    fn bet_amount<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, _date: f64) -> (f64, f64) {
        (
            self.money_strategy.calculate(simulation, tier, left, right),
            self.money_strategy.calculate(simulation, tier, right, left),
        )
    }

    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str, _date: f64) -> Bet {
        if self.bet_strategy.calculate(simulation, tier, left, right) {
            let p_left = self.prediction_strategy.calculate(simulation, tier, left, right);
            let p_right = self.prediction_strategy.calculate(simulation, tier, right, left);

            if p_left > p_right {
                Bet::Left(self.money_strategy.calculate(simulation, tier, left, right))

            } else if p_right > p_left {
                Bet::Right(self.money_strategy.calculate(simulation, tier, right, left))

            } else {
                Bet::None
            }

        } else {
            Bet::None
        }
    }
}


#[derive(Debug)]
pub struct SimulationSettings<'a> {
    pub records: &'a [Record],
    pub mode: Mode,
}

impl<A> FitnessResult<A> where A: Strategy + Clone {
    // TODO figure out a way to avoid the clone and to_vec
    pub fn new<'a>(settings: &SimulationSettings<'a>, creature: A) -> Self {
        let (fitness, successes, failures, record_len, characters_len, max_character_len) = {
            let mut simulation = Simulation::<A, A>::new();

            // TODO is this correct ?
            simulation.sum = 10_000_000.0;

            match settings.mode {
                Mode::Matchmaking => simulation.matchmaking_strategy = Some(creature.clone()),
                Mode::Tournament => simulation.tournament_strategy = Some(creature.clone()),
            }

            // TODO is this correct ?
            /*for record in settings.records {
                simulation.insert_record(record);
            }*/

            /*let mut successes = 0.0;
            let mut failures = 0.0;

            let mut len = 0.0;
            let mut sum = 0.0;

            for record in settings.records {
                if let Some(odds) = record.odds_winner(&simulation.pick_winner(&creature, &record.tier, &record.left.name, &record.right.name, record.date)) {
                    len += 1.0;

                    match odds {
                        Ok(odds) => {
                            successes += 1.0;
                            sum += odds;
                        },
                        Err(_) => {
                            failures += 1.0;
                            sum -= 1.0;
                        },
                    }
                }
            }*/

            simulation.simulate(settings.records.to_vec(), true);

            (
                if simulation.record_len == 0.0 { 0.0 } else { simulation.upsets / simulation.record_len }, // simulation.sum,
                simulation.successes,
                simulation.failures,
                simulation.record_len,
                simulation.characters.len(),
                simulation.max_character_len,
            )
        };

        FitnessResult {
            fitness: fitness,
            successes: successes,
            failures: failures,
            record_len: record_len,
            characters_len: characters_len,
            max_character_len: max_character_len,
            creature,
        }
    }
}

impl<A> Ord for FitnessResult<A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.fitness.is_nan() {
            if other.fitness.is_nan() {
                std::cmp::Ordering::Equal

            } else {
                std::cmp::Ordering::Less
            }

        } else if other.fitness.is_nan() {
            std::cmp::Ordering::Greater

        } else {
            self.partial_cmp(other).unwrap()
        }
    }
}

impl<A> PartialOrd for FitnessResult<A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.fitness.partial_cmp(&other.fitness)
    }
}

impl<A> PartialEq for FitnessResult<A> {
    // TODO handle NaN ?
    fn eq(&self, other: &Self) -> bool {
        self.fitness == other.fitness
    }
}

impl<A> Eq for FitnessResult<A> {}


#[derive(Debug)]
pub struct Population<'a, A, B> where A: Creature, B: 'a {
    data: &'a B,
    size: usize,
    // TODO is it faster to use a Box ?
    // TODO use ArrayVec ?
    pub populace: Vec<Box<FitnessResult<A>>>,
}

impl<'a, A, B> Population<'a, A, B> where A: Creature, B: 'a {
    pub fn new(size: usize, data: &'a B) -> Self {
        Self {
            data,
            size,
            populace: Vec::with_capacity(size),
        }
    }
}

impl<'a, A> Population<'a, A, SimulationSettings<'a>> where A: Creature + Clone + Send + Sync {
    fn insert_creature(&mut self, result: FitnessResult<A>) {
        let index = self.populace.binary_search_by(|value| (**value).cmp(&result));

        let index = match index {
            Ok(index) => index,
            Err(index) => index,
        };

        self.populace.insert(index, Box::new(result));
    }

    fn kill_populace(&mut self) {
        let mut index: f64 = 0.0;

        let len = (self.populace.len() - 1) as f64;

        // TODO is it guaranteed that retain operates from left-to-right ?
        self.populace.retain(|_| {
            let keep = rand_is_percent(Percentage(index / len));
            index += 1.0;
            keep
        });
    }

    fn breed_populace(&mut self) {
        let new_creatures: Vec<FitnessResult<A>> = {
            let closure = |_| {
                let father = choose(&self.populace);
                let mother = choose(&self.populace);

                FitnessResult::new(self.data, match father {
                    Some(father) => match mother {
                        Some(mother) => father.creature.breed(&mother.creature),
                        None => A::new(),
                    },
                    None => A::new(),
                })
            };

            (self.populace.len()..self.size).into_par_iter().map(closure).collect()
        };

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn best(&self) -> &FitnessResult<A> {
        self.populace.last().unwrap()
    }

    pub fn init(&mut self) {
        // TODO code duplication
        let new_creatures: Vec<FitnessResult<A>> = (0..self.size).into_par_iter().map(|_| FitnessResult::new(self.data, A::new())).collect();

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn next_generation(&mut self) {
        self.kill_populace();
        self.breed_populace();
    }
}
