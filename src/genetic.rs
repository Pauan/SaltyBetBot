use std;
use rand;
use record::{ Record, Mode, Tier };
//use rayon::prelude::*;
use simulation::{ Simulation, Strategy, Bet, Lookup, Calculate, Simulator };


const MAX_BET_AMOUNT: f64 = 1000000.0;
pub const MUTATION_RATE: Percentage = Percentage(0.10);
const MAX_RECURSION_DEPTH: u32 = 6; // 64 maximum nodes


pub trait Creature<A>: Ord {
    fn new(data: &A) -> Self;

    fn breed(&self, other: &Self, data: &A) -> Self;
}


pub trait Gene {
    fn new() -> Self;

    fn choose(&self, other: &Self) -> Self;
}


impl Gene for bool {
    fn new() -> Self {
        rand::bool()
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
        let Percentage(percent) = Gene::new();

        MAX_BET_AMOUNT * ((percent * 2.0) - 1.0)
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
        let Closed01(val) = rand::weak_rng().gen::<Closed01<f32>>();
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
    rand::between_exclusive(0, index)
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
        Some(&values[rand::between_exclusive(0, values.len() as u32) as usize])
    }
}


#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Percentage(pub f64);

impl Gene for Percentage {
    fn new() -> Self {
        Percentage(rand::percentage())
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


#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
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


#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CubicBezierSegment {
    pub from: Point,
    pub ctrl1: Point,
    pub ctrl2: Point,
    pub to: Point,
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


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NumericCalculator<A, B> where A: Calculate<B> {
    Base(A),
    Fixed(B),
    Percentage(Percentage),

    Bezier(CubicBezierSegment, Box<NumericCalculator<A, B>>),
    Average(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
    Abs(Box<NumericCalculator<A, B>>),
    Min(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
    Max(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
    Plus(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
    Minus(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
    Multiply(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),
    Divide(Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),

    // TODO change to use BooleanCalculator<NumericCalculator<A, B>>
    IfThenElse(BooleanCalculator<A>, Box<NumericCalculator<A, B>>, Box<NumericCalculator<A, B>>),

    Tier {
        new: Box<NumericCalculator<A, B>>,
        x: Box<NumericCalculator<A, B>>,
        s: Box<NumericCalculator<A, B>>,
        a: Box<NumericCalculator<A, B>>,
        b: Box<NumericCalculator<A, B>>,
        p: Box<NumericCalculator<A, B>>,
    },
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
            let rand = gen_rand_index(14u32);

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

            } else if rand == 12 {
                NumericCalculator::IfThenElse(Gene::new(), Box::new(Self::_new(depth + 1)), Box::new(Self::_new(depth + 1)))

            } else {
                NumericCalculator::Tier {
                    new: Box::new(Self::_new(depth + 1)),
                    x: Box::new(Self::_new(depth + 1)),
                    s: Box::new(Self::_new(depth + 1)),
                    a: Box::new(Self::_new(depth + 1)),
                    b: Box::new(Self::_new(depth + 1)),
                    p: Box::new(Self::_new(depth + 1)),
                }
            }
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
                NumericCalculator::Tier { new: ref father_new, x: ref father_x, s: ref father_s, a: ref father_a, b: ref father_b, p: ref father_p } => match *other {
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
                },
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

                    NumericCalculator::Tier { new, x, s, a, b, p } => match *tier {
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
                    },
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

            NumericCalculator::Tier { ref new, ref x, ref s, ref a, ref b, ref p } =>
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
                    })))))),
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

            NumericCalculator::Tier { ref new, ref x, ref s, ref a, ref b, ref p } => match *tier {
                Tier::New => new.calculate(simulation, tier, left, right),
                Tier::X => x.calculate(simulation, tier, left, right),
                Tier::S => s.calculate(simulation, tier, left, right),
                Tier::A => a.calculate(simulation, tier, left, right),
                Tier::B => b.calculate(simulation, tier, left, right),
                Tier::P => p.calculate(simulation, tier, left, right),
            },
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


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BooleanCalculator<A> {
    True,
    False,
    Greater(A, A),
    GreaterEqual(A, A),
    Lesser(A, A),
    LesserEqual(A, A),
    And(Box<BooleanCalculator<A>>, Box<BooleanCalculator<A>>),
    Or(Box<BooleanCalculator<A>>, Box<BooleanCalculator<A>>),
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


#[derive(Debug)]
pub struct SimulationSettings<'a> {
    pub records: &'a Vec<Record>,
    pub mode: Mode,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetStrategy {
    pub fitness: f64,
    pub successes: f64,
    pub failures: f64,
    pub record_len: f64,
    pub characters_len: usize,
    pub max_character_len: usize,

    // Genes
    pub bet_strategy: BooleanCalculator<NumericCalculator<Lookup, f64>>,
    pub prediction_strategy: NumericCalculator<Lookup, f64>,
    pub money_strategy: NumericCalculator<Lookup, f64>,
}

impl<'a> BetStrategy {
    // TODO figure out a way to avoid the clones
    fn calculate_fitness(mut self, settings: &SimulationSettings<'a>) -> Self {
        let (sum, successes, failures, record_len, characters_len, max_character_len) = {
            let mut simulation = Simulation::new();

            match settings.mode {
                Mode::Matchmaking => simulation.matchmaking_strategy = Some(self.clone()),
                Mode::Tournament => simulation.tournament_strategy = Some(self.clone()),
            }

            simulation.simulate(settings.records.clone());

            (
                simulation.sum,
                simulation.successes,
                simulation.failures,
                simulation.record_len,
                simulation.characters.len(),
                simulation.max_character_len,
            )
        };

        self.fitness = sum;
        self.successes = successes;
        self.failures = failures;
        self.record_len = record_len;
        self.characters_len = characters_len;
        self.max_character_len = max_character_len;

        self
    }
}

impl<'a> Creature<SimulationSettings<'a>> for BetStrategy {
    fn new(settings: &SimulationSettings<'a>) -> Self {
        Self {
            fitness: 0.0,
            successes: 0.0,
            failures: 0.0,
            record_len: 0.0,
            characters_len: 0,
            max_character_len: 0,
            bet_strategy: Gene::new(),
            prediction_strategy: Gene::new(),
            money_strategy: Gene::new(),
        }.calculate_fitness(settings)
    }

    fn breed(&self, other: &Self, settings: &SimulationSettings<'a>) -> Self {
        Self {
            fitness: 0.0,
            successes: 0.0,
            failures: 0.0,
            record_len: 0.0,
            characters_len: 0,
            max_character_len: 0,
            bet_strategy: self.bet_strategy.choose(&other.bet_strategy),
            prediction_strategy: self.prediction_strategy.choose(&other.prediction_strategy),
            money_strategy: self.money_strategy.choose(&other.money_strategy),
        }.calculate_fitness(settings)
    }
}

impl Strategy for BetStrategy {
    fn bet<A: Simulator>(&self, simulation: &A, tier: &Tier, left: &str, right: &str) -> Bet {
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

impl Ord for BetStrategy {
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

impl PartialOrd for BetStrategy {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.fitness.partial_cmp(&other.fitness)
    }
}

impl PartialEq for BetStrategy {
    fn eq(&self, other: &Self) -> bool {
        self.fitness == other.fitness
    }
}

impl Eq for BetStrategy {}



#[derive(Debug)]
pub struct Population<'a, A, B> where A: Creature<B>, B: 'a {
    data: &'a B,
    amount: usize,
    pub populace: Vec<A>,
}

impl<'a, A, B> Population<'a, A, B> where A: Creature<B> + Send + Sync, B: 'a + Sync {
    pub fn new(amount: usize, data: &'a B) -> Self {
        Self {
            data: data,
            amount: amount,
            populace: Vec::with_capacity(amount),
        }
    }

    fn insert_creature(&mut self, creature: A) {
        let index = self.populace.binary_search(&creature);

        match index {
            Ok(index) => self.populace.insert(index, creature),
            Err(index) => self.populace.insert(index, creature),
        }
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
        let new_creatures: Vec<A> = {
            let closure = |_| {
                let father = choose(&self.populace);
                let mother = choose(&self.populace);

                match father {
                    Some(father) => match mother {
                        Some(mother) => father.breed(mother, self.data),
                        None => A::new(self.data),
                    },
                    None => A::new(self.data),
                }
            };

            //if cfg!(any(target_arch = "wasm32", target_arch = "asmjs")) {
                (self.populace.len()..self.amount).map(closure).collect()

            /*} else {
                (self.populace.len()..self.amount).into_par_iter().map(closure).collect()
            }*/
        };

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn best(&self) -> &A {
        self.populace.last().unwrap()
    }

    pub fn init(&mut self) {
        // TODO code duplication
        let new_creatures: Vec<A> = //if cfg!(any(target_arch = "wasm32", target_arch = "asmjs")) {
            (0..self.amount).map(|_| A::new(self.data)).collect()

        /*} else {
            (0..self.amount).into_par_iter().map(|_|A::new(self.data)).collect()
        }*/;

        for creature in new_creatures {
            self.insert_creature(creature);
        }
    }

    pub fn next_generation(&mut self) {
        self.kill_populace();
        self.breed_populace();
    }
}
