use std;
use regex;
use csv;


#[derive(Debug)]
pub enum Winner {
    Left(f64),
    Right(f64),
}

impl Winner {
    fn parse(input: &str, odds: &Odds) -> Result<Winner, Box<std::error::Error>> {
        match input {
            "0" => Ok(match odds {
                &Odds::Left(odds) => Winner::Left(1.0 / odds),
                &Odds::Right(odds) => Winner::Left(odds),
            }),
            "1" => Ok(match odds {
                &Odds::Right(odds) => Winner::Right(1.0 / odds),
                &Odds::Left(odds) => Winner::Right(odds),
            }),
            _ => Err(From::from(format!("invalid winner {}", input))),
        }
    }
}


#[derive(Debug)]
pub enum Tier {
    X,
    S,
    A,
    B,
    P,
}

impl Tier {
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
}


#[derive(Debug)]
pub enum Mode {
    Matchmaking,
    Tournament,
}

impl Mode {
    fn parse(input: &str) -> Result<Mode, Box<std::error::Error>> {
        match input {
            "m" => Ok(Mode::Matchmaking),
            "t" => Ok(Mode::Tournament),
            _ => Err(From::from(format!("invalid mode {}", input))),
        }
    }
}


#[derive(Debug)]
pub enum Odds {
    Left(f64),
    Right(f64),
}

fn error<A>(input: Option<A>, message: &str) -> Result<A, Box<std::error::Error>> {
    match input {
        Some(a) => Ok(a),
        None => Err(From::from(format!("invalid odds {}", message))),
    }
}

impl Odds {
    fn parse(input: &str) -> Result<Odds, Box<std::error::Error>> {
        lazy_static! {
            static ref ODDS_REGEX: regex::Regex = regex::Regex::new(r"^([0-9]+(?:\.[0-9]+)?):([0-9]+(?:\.[0-9]+)?)$").unwrap();
        }

        let capture = error(ODDS_REGEX.captures(input), input)?;
        let left = error(capture.get(1), input)?.as_str();
        let right = error(capture.get(2), input)?.as_str();

        if left == "1" {
            Ok(Odds::Right(right.parse::<f64>()?))

        } else if right == "1" {
            Ok(Odds::Left(left.parse::<f64>()?))

        } else {
            Err(From::from(format!("invalid odds {}", input)))
        }
    }
}


#[derive(Debug)]
pub struct Record {
    pub character_left: String,
    pub character_right: String,
    pub winner: Winner,
    pub tier: Tier,
    pub mode: Mode,
    pub odds: Odds,
    pub duration: u32,
    //pub date: chrono::DateTime<chrono::Utc>,
}


fn parse_duration(input: u32) -> u32 {
    input * 1000
}


/*fn parse_date(input: &str) -> Result<chrono::DateTime<chrono::Utc>, Box<std::error::Error>> {
    println!("{:?}", input);
    Ok(chrono::Utc.datetime_from_str(input, "%d-%m-%Y")?)
}*/


pub fn parse_csv(data: &str) -> Result<Vec<Record>, Box<std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
          .has_headers(false)
          .quoting(false)
          .escape(None)
          .comment(None)
          .from_reader(data.as_bytes());

    let mut output: Vec<Record> = vec![];

    for result in reader.deserialize() {
        let (character1, character2, winner, _strategy, _prediction, tier,   mode,   odds,   duration, _crowd_favorite, _illuminati_favorite, date):
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

        let odds = Odds::parse(&odds)?;
        let winner = Winner::parse(&winner, &odds)?;

        output.push(Record {
            character_left: character1,
            character_right: character2,
            winner: winner,
            tier: Tier::parse(&tier)?,
            mode: Mode::parse(&mode)?,
            odds: odds,
            duration: parse_duration(duration),
            //date: parse_date(&date)?
        });
    }

    Ok(output)
}
