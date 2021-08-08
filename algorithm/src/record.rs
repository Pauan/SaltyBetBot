//use std;
//use regex;
//use csv;
use serde_json;
use std::cmp::{PartialOrd, Ordering};
use crate::genetic;
use crate::simulation::Bet;


/*fn error<A>(input: Option<A>, message: &str) -> Result<A, Box<std::error::Error>> {
    match input {
        Some(a) => Ok(a),
        None => Err(From::from(format!("invalid odds {}", message))),
    }
}

fn parse_odds(input: &str) -> Result<(f64, f64), Box<std::error::Error>> {
    lazy_static! {
        static ref ODDS_REGEX: regex::Regex = regex::Regex::new(r"^([0-9]+(?:\.[0-9]+)?):([0-9]+(?:\.[0-9]+)?)$").unwrap();
    }

    let capture = error(ODDS_REGEX.captures(input), input)?;
    let left = error(capture.get(1), input)?.as_str();
    let right = error(capture.get(2), input)?.as_str();

    if left == "1" {
        Ok((1.0, right.parse::<f64>()?))

    } else if right == "1" {
        Ok((left.parse::<f64>()?, 1.0))

    } else {
        Err(From::from(format!("invalid odds {}", input)))
    }
}*/


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Winner {
    Left,
    Right,
}

impl Winner {
    fn swap(self) -> Self {
        match self {
            Winner::Left => Winner::Right,
            Winner::Right => Winner::Left,
        }
    }

    /*fn parse(input: &str) -> Result<Winner, Box<std::error::Error>> {
        match input {
            "0" => Ok(Winner::Left),
            "1" => Ok(Winner::Right),
            _ => Err(From::from(format!("invalid winner {}", input))),
        }
    }*/
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    None,
    New,
    P,
    B,
    A,
    S,
    X,
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Tier::None => "None",
            Tier::New => "NEW",
            Tier::P => "P",
            Tier::B => "B",
            Tier::A => "A",
            Tier::S => "S",
            Tier::X => "X",
        })
    }
}

/*impl Tier {
    fn parse(input: &str) -> Result<Tier, Box<std::error::Error>> {
        match input {
            "X" => Ok(Tier::X),
            "S" => Ok(Tier::S),
            "A" => Ok(Tier::A),
            "B" => Ok(Tier::B),
            "P" => Ok(Tier::P),
            _ => Err(From::from(format!("invalid tier {}", input))),
        }
    }
}*/


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Mode {
    Matchmaking,
    Tournament,
    Exhibitions,
}

impl Mode {
    pub fn is_exhibitions(&self) -> bool {
        match self {
            Self::Exhibitions => true,
            Self::Matchmaking | Self::Tournament => false,
        }
    }

    pub fn is_tournament(&self) -> bool {
        match self {
            Self::Tournament => true,
            Self::Matchmaking | Self::Exhibitions => false,
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Mode::Matchmaking => "Matchmaking",
            Mode::Tournament => "Tournament",
            Mode::Exhibitions => "Exhibitions",
        })
    }
}

/*impl Mode {
    fn parse(input: &str) -> Result<Mode, Box<std::error::Error>> {
        match input {
            "m" => Ok(Mode::Matchmaking),
            "t" => Ok(Mode::Tournament),
            _ => Err(From::from(format!("invalid mode {}", input))),
        }
    }
}*/


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Character {
    pub name: String,
    pub bet_amount: f64,
    pub win_streak: f64,

    pub illuminati_bettors: f64,
    pub normal_bettors: f64,

    // This includes self, and also anybody who bets $1
    // TODO default this to something else, like -1 ? Or maybe use Option ?
    #[serde(default)]
    pub ignored_bettors: f64,
}

impl Character {
    pub fn bettors(&self) -> f64 {
        self.illuminati_bettors + self.normal_bettors
    }

    // The bet_amount, illuminati_bettors, and normal_bettors are too unreliable
    fn is_duplicate(&self, other: &Self) -> bool {
        self.name == other.name &&
        self.win_streak == other.win_streak
    }

    pub fn average_bet_amount(&self) -> f64 {
        self.bet_amount / self.bettors()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Profit {
    Loss(f64),
    None,
    Gain(f64),
}

impl Profit {
    pub fn from_old_new(old: f64, new: f64) -> Self {
        let diff = (old - new).abs();

        match old.partial_cmp(&new).unwrap() {
            Ordering::Less => Profit::Gain(diff),
            Ordering::Greater => Profit::Loss(diff),
            Ordering::Equal => Profit::None,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub left: Character,
    pub right: Character,
    pub winner: Winner,
    pub tier: Tier,
    pub mode: Mode,
    pub bet: Bet,
    pub duration: f64,
    pub date: f64,
    #[serde(default = "Record::default_sum")]
    pub sum: f64,
}

impl Record {
    #[inline]
    fn default_sum() -> f64 {
        -1.0
    }

    pub fn shuffle(self) -> Self {
        if genetic::Gene::new() {
            self

        } else {
            Record {
                left: self.right,
                right: self.left,
                winner: self.winner.swap(),
                tier: self.tier,
                mode: self.mode,
                bet: self.bet.swap(),
                duration: self.duration,
                date: self.date,
                sum: self.sum,
            }
        }
    }

    pub fn sort_date(x: &Self, y: &Self) -> Ordering {
        x.date.partial_cmp(&y.date).unwrap()
    }

    // The duration and date are too unreliable
    pub fn is_duplicate(&self, other: &Self) -> bool {
        self.left.is_duplicate(&other.left) &&
        self.right.is_duplicate(&other.right) &&
        self.winner == other.winner &&
        self.tier == other.tier &&
        self.mode == other.mode
    }

    pub fn odds_left(&self, bet_amount: f64) -> f64 {
        self.right.bet_amount / (self.left.bet_amount + bet_amount)
    }

    pub fn odds_right(&self, bet_amount: f64) -> f64 {
        self.left.bet_amount / (self.right.bet_amount + bet_amount)
    }

    pub fn odds(&self, bet: &Bet) -> Option<f64> {
        match bet {
            Bet::Left(amount) => Some(self.odds_left(*amount)),
            Bet::Right(amount) => Some(self.odds_right(*amount)),
            Bet::None => None,
        }
    }

    pub fn odds_winner(&self, bet: &Bet) -> Option<Result<f64, f64>> {
        match bet {
            Bet::Left(amount) => Some(match self.winner {
                Winner::Left => Ok(self.odds_left(*amount)),
                Winner::Right => Err(self.odds_right(*amount)),
            }),
            Bet::Right(amount) => Some(match self.winner {
                Winner::Right => Ok(self.odds_right(*amount)),
                Winner::Left => Err(self.odds_left(*amount)),
            }),
            Bet::None => None,
        }
    }

    pub fn display_odds(&self) -> (f64, f64) {
        let mut left = self.left.bet_amount;
        let mut right = self.right.bet_amount;

        match self.bet {
            Bet::Left(amount) => {
                left += amount;
            },
            Bet::Right(amount) => {
                right += amount;
            },
            Bet::None => {},
        }

        if left < right {
            (1.0, right / left)

        } else if left > right {
            (left / right, 1.0)

        } else {
            (1.0, 1.0)
        }
    }

    // TODO handle tournaments
    pub fn profit(&self, bet: &Bet) -> Profit {
        match bet {
            Bet::Left(amount) => match self.winner {
                Winner::Left => {
                    Profit::Gain((amount * self.odds_left(*amount)).ceil())
                },
                Winner::Right => {
                    Profit::Loss(*amount)
                },
            },
            Bet::Right(amount) => match self.winner {
                Winner::Right => {
                    Profit::Gain((amount * self.odds_right(*amount)).ceil())
                },
                Winner::Left => {
                    Profit::Loss(*amount)
                },
            },
            Bet::None => {
                Profit::None
            },
        }
    }

    pub fn won(&self, bet: &Bet) -> bool {
        match bet {
            Bet::Left(_) => match self.winner {
                Winner::Left => true,
                Winner::Right => false,
            },
            Bet::Right(_) => match self.winner {
                Winner::Right => true,
                Winner::Left => false,
            },
            Bet::None => false,
        }
    }

    // TODO better detection for whether the input matches or not
    pub fn is_winner(&self, input: &str) -> bool {
        match self.winner {
            Winner::Left => self.left.name == input,
            Winner::Right => self.right.name == input,
        }
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn deserialize(str: &str) -> Self {
        serde_json::from_str(str).unwrap()
    }
}


/*fn parse_duration(input: u32) -> f64 {
    (input * 1000) as f64
}*/


/*fn parse_date(input: &str) -> Result<chrono::DateTime<chrono::Utc>, Box<std::error::Error>> {
    log!("{:?}", input);
    Ok(chrono::Utc.datetime_from_str(input, "%d-%m-%Y")?)
}*/


/*pub fn parse_csv(data: &str) -> Result<Vec<Record>, Box<std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
          .has_headers(false)
          .quoting(false)
          .escape(None)
          .comment(None)
          .from_reader(data.as_bytes());

    let mut output: Vec<Record> = vec![];

    for result in reader.deserialize() {
        let (character1, character2, winner, _strategy, _prediction, tier,   mode,   odds,   duration, _crowd_favorite, _illuminati_favorite, _date):
            (String,     String,     String, String,    String,      String, String, String, u32,      String,          String,               String) = result?;

        if tier == "U" {
            continue;
        }

        if mode == "e" {
            continue;
        }

        if odds == "U" {
            continue;
        }

        let (left_odds, right_odds) = parse_odds(&odds)?;

        output.push(Record {
            left: Character {
                name: character1,
                bet_amount: left_odds,
                win_streak: 0.0, // TODO
                illuminati_bettors: 0.0, // TODO
                normal_bettors: 0.0, // TODO
            },
            right: Character {
                name: character2,
                bet_amount: right_odds,
                win_streak: 0.0, // TODO
                illuminati_bettors: 0.0, // TODO
                normal_bettors: 0.0, // TODO
            },
            winner: Winner::parse(&winner)?,
            tier: Tier::parse(&tier)?,
            mode: Mode::parse(&mode)?,
            bet: Bet::None, // TODO
            duration: parse_duration(duration),
            date: 0.0, // TODO
        });
    }

    Ok(output)
}
*/


pub fn serialize_records(records: &[Record]) -> String {
    serde_json::to_string_pretty(&records).unwrap()
}

pub fn deserialize_records(records: &str) -> Vec<Record> {
    serde_json::from_str(records).unwrap()
}
