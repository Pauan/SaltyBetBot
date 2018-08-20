#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LookupStatistic {
    Upsets,
    Favored,
    Winrate,
    Odds,
    Earnings,
    MatchesLen,
    BetAmount,
    Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LookupFilter {
    All,
    Specific,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LookupSide {
    Left,
    Right
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lookup {
    Sum,
    Character(LookupSide, LookupFilter, LookupStatistic),
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Percentage(pub f64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CubicBezierSegment {
    pub from: Point,
    pub ctrl1: Point,
    pub ctrl2: Point,
    pub to: Point,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NumericCalculator<A, B> {
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

    /*Tier {
        new: Box<NumericCalculator<A, B>>,
        x: Box<NumericCalculator<A, B>>,
        s: Box<NumericCalculator<A, B>>,
        a: Box<NumericCalculator<A, B>>,
        b: Box<NumericCalculator<A, B>>,
        p: Box<NumericCalculator<A, B>>,
    },*/
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
