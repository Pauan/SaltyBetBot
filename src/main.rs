#[macro_use]
extern crate salty_bet_bot;
//extern crate serde;
//extern crate serde_json;
#[macro_use]
extern crate stdweb;

/*use std::fs::File;
use std::path::Path;
use std::io::Read;*/

use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::fs::File;


use salty_bet_bot::{genetic, record, simulation};


/*fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(&Path::new(path))?;

    let mut s = String::new();

    file.read_to_string(&mut s)?;

    Ok(s)
}*/


/*fn write_file(filename: &str) -> Result<(), std::io::Error> {
    let records = {
        let data = include_str!("../records/saltyRecordsM--2018-1-16-14.29.txt");
        record::parse_csv(&data).unwrap()
    };

    let settings = genetic::SimulationSettings {
        mode: record::Mode::Tournament,
        records: &records,
    };

    let mut population: genetic::Population<genetic::BetStrategy, genetic::SimulationSettings> = genetic::Population::new(1000, &settings);

    log!("Initializing...");

    population.init();

    // TODO file an issue for Rust about adding in documentation to File encouraging people to use BufWriter
    let mut buffer = BufWriter::new(File::create(filename)?);

    {
        let best = population.best();
        write!(buffer, "{:#?}\n", population.populace)?;
        write!(buffer, "<<<<<<<<<<<<<<<<<<<<<<<<<<\n")?;
        buffer.flush()?;
        log!("Initialized: {}", best.fitness);
    }

    for i in 0..1000 {
        population.next_generation();

        let best = population.best();
        write!(buffer, "{:#?}\n", best)?;
        buffer.flush()?;
        log!("Generation {}: {}", i + 1, best.fitness);
    }

    write!(buffer, ">>>>>>>>>>>>>>>>>>>>>>>>>>\n")?;
    write!(buffer, "{:#?}\n", population.populace)?;
    buffer.flush()?;

    Ok(())
}*/


/*fn read_strategy(filename: &str) -> Result<genetic::BetStrategy, std::io::Error> {
    let buffer = BufReader::new(File::open(filename)?);
    Ok(serde_json::from_reader(buffer)?)
}

fn write_strategy<A: simulation::Strategy + serde::Serialize>(filename: &str, strategy: &A) -> Result<(), std::io::Error> {
    let buffer = BufWriter::new(File::create(filename)?);
    Ok(serde_json::to_writer_pretty(buffer, strategy)?)
}*/


/*fn run_simulation() -> Result<(), std::io::Error> {
    use genetic::{ BetStrategy, CubicBezierSegment, Point };
    use genetic::BooleanCalculator::*;
    use genetic::NumericCalculator::*;
    use simulation::Calculate;
    use simulation::Lookup::*;
    use simulation::LookupSide::*;
    use simulation::LookupStatistic::*;
    use simulation::LookupFilter::*;


    let matchmaking_strategy2 = read_strategy("strategies/matchmaking_strategy")?;
    let tournament_strategy2 = read_strategy("strategies/tournament_strategy")?;





    let matchmaking_strategy = BetStrategy {
        fitness: 6757872108009969000000000000000000000000000000000000000000000000000000000000000000000000.0,
        successes: 33021.0,
        failures: 31413.0,
        record_len: 91468.0,
        characters_len: 7972,
        max_character_len: 86,
        bet_strategy: True,
        prediction_strategy: Tier {
            x: Box::new(Base(
                Character(
                    Left,
                    All,
                    Earnings
                )
            )),
            s: Box::new(Divide(
                Box::new(IfThenElse(
                    Lesser(
                        Character(
                            Right,
                            All,
                            Favored
                        ),
                        Character(
                            Left,
                            All,
                            Odds
                        )
                    ),
                    Box::new(Plus(
                        Box::new(Max(
                            Box::new(Tier {
                                x: Box::new(Base(
                                    Sum
                                )),
                                s: Box::new(Percentage(
                                    genetic::Percentage(
                                        0.19595607148216068
                                    )
                                )),
                                a: Box::new(Fixed(
                                    -267653.2112371593
                                )),
                                b: Box::new(Percentage(
                                    genetic::Percentage(
                                        0.09628313098722964
                                    )
                                )),
                                p: Box::new(Base(
                                    Character(
                                        Right,
                                        All,
                                        BetAmount
                                    )
                                ))
                            }),
                            Box::new(Percentage(
                                genetic::Percentage(
                                    0.7023460766950024
                                )
                            ))
                        )),
                        Box::new(Base(
                            Character(
                                Left,
                                All,
                                Winrate
                            )
                        ))
                    )),
                    Box::new(Tier {
                        x: Box::new(Fixed(
                            231022.7920343284
                        )),
                        s: Box::new(Fixed(
                            -0.000001200472291490774
                        )),
                        a: Box::new(Fixed(
                            494138.7814781472
                        )),
                        b: Box::new(Fixed(
                            0.6689264926148141
                        )),
                        p: Box::new(Base(
                            Character(
                                Left,
                                Specific,
                                Duration
                            )
                        ))
                    })
                )),
                Box::new(Minus(
                    Box::new(Average(
                        Box::new(Abs(
                            Box::new(Multiply(
                                Box::new(Base(
                                    Character(
                                        Left,
                                        All,
                                        Upsets
                                    )
                                )),
                                Box::new(Fixed(
                                    267896.41408951214
                                ))
                            ))
                        )),
                        Box::new(Fixed(
                            -410297.8759880543
                        ))
                    )),
                    Box::new(Abs(
                        Box::new(Plus(
                            Box::new(Fixed(
                                -786859.6889044034
                            )),
                            Box::new(Multiply(
                                Box::new(Base(
                                    Character(
                                        Left,
                                        All,
                                        BetAmount
                                    )
                                )),
                                Box::new(Fixed(
                                    352426.5091200507
                                ))
                            ))
                        ))
                    ))
                ))
            )),
            a: Box::new(Divide(
                Box::new(Minus(
                    Box::new(Multiply(
                        Box::new(Percentage(
                            genetic::Percentage(
                                0.16832453254782823
                            )
                        )),
                        Box::new(Multiply(
                            Box::new(Multiply(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.4321325338376472
                                    )
                                )),
                                Box::new(Base(
                                    Sum
                                ))
                            )),
                            Box::new(Max(
                                Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        Earnings
                                    )
                                )),
                                Box::new(Base(
                                    Character(
                                        Right,
                                        All,
                                        BetAmount
                                    )
                                ))
                            ))
                        ))
                    )),
                    Box::new(Divide(
                        Box::new(Fixed(
                            -5813382621729153000000.0
                        )),
                        Box::new(Multiply(
                            Box::new(Fixed(
                                -211908.67953557768
                            )),
                            Box::new(Bezier(
                                CubicBezierSegment {
                                    from: Point {
                                        x: -694681.5374679107,
                                        y: 667659.4760902412
                                    },
                                    ctrl1: Point {
                                        x: -36294.6823591227,
                                        y: -412806.52791049855
                                    },
                                    ctrl2: Point {
                                        x: 164938.4540434069,
                                        y: -116210.92582819992
                                    },
                                    to: Point {
                                        x: 104125.63211500259,
                                        y: 696402.9257636211
                                    }
                                },
                                Box::new(Base(
                                    Character(
                                        Left,
                                        All,
                                        Odds
                                    )
                                ))
                            ))
                        ))
                    ))
                )),
                Box::new(Min(
                    Box::new(Fixed(
                        38299070476265000000000.0
                    )),
                    Box::new(Average(
                        Box::new(Abs(
                            Box::new(Base(
                                Sum
                            ))
                        )),
                        Box::new(Tier {
                            x: Box::new(Fixed(
                                470963.0152231348
                            )),
                            s: Box::new(Fixed(
                                109761.06362234429
                            )),
                            a: Box::new(Base(
                                Character(
                                    Left,
                                    All,
                                    Earnings
                                )
                            )),
                            b: Box::new(Percentage(
                                genetic::Percentage(
                                    0.31348641682633654
                                )
                            )),
                            p: Box::new(Fixed(
                                530521.6647710928
                            ))
                        })
                    ))
                ))
            )),
            b: Box::new(Minus(
                Box::new(Divide(
                    Box::new(Average(
                        Box::new(Base(
                            Character(
                                Left,
                                All,
                                MatchesLen
                            )
                        )),
                        Box::new(Multiply(
                            Box::new(Average(
                                Box::new(Base(
                                    Sum
                                )),
                                Box::new(Base(
                                    Character(
                                        Right,
                                        Specific,
                                        Odds
                                    )
                                ))
                            )),
                            Box::new(Base(
                                Character(
                                    Left,
                                    All,
                                    Favored
                                )
                            ))
                        ))
                    )),
                    Box::new(Bezier(
                        CubicBezierSegment {
                            from: Point {
                                x: -440233.90696612885,
                                y: -637781.5049788693
                            },
                            ctrl1: Point {
                                x: 258825.0895854711,
                                y: 832875.8638675795
                            },
                            ctrl2: Point {
                                x: -68826.79917777501,
                                y: 782336.1448990782
                            },
                            to: Point {
                                x: 518193.19357426494,
                                y: -856032.6090341177
                            }
                        },
                        Box::new(Bezier(
                            CubicBezierSegment {
                                from: Point {
                                    x: 352973.7013758145,
                                    y: -784343.6024992085
                                },
                                ctrl1: Point {
                                    x: 682335.7581625234,
                                    y: -752972.1411953743
                                },
                                ctrl2: Point {
                                    x: -496486.2555522641,
                                    y: -568599.8390616294
                                },
                                to: Point {
                                    x: -845737.7290152488,
                                    y: 810799.043459532
                                }
                            },
                            Box::new(Plus(
                                Box::new(Base(
                                    Sum
                                )),
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.9377245903357799
                                    )
                                ))
                            ))
                        ))
                    ))
                )),
                Box::new(Plus(
                    Box::new(Base(
                        Character(
                            Right,
                            Specific,
                            Duration
                        )
                    )),
                    Box::new(Plus(
                        Box::new(Tier {
                            x: Box::new(Percentage(
                                genetic::Percentage(
                                    0.9089624396437214
                                )
                            )),
                            s: Box::new(Average(
                                Box::new(Base(
                                    Sum
                                )),
                                Box::new(Base(
                                    Character(
                                        Left,
                                        All,
                                        Duration
                                    )
                                ))
                            )),
                            a: Box::new(Multiply(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.47585101863597196
                                    )
                                )),
                                Box::new(Base(
                                    Sum
                                ))
                            )),
                            b: Box::new(Max(
                                Box::new(Base(
                                    Character(
                                        Right,
                                        All,
                                        Earnings
                                    )
                                )),
                                Box::new(Base(
                                    Character(
                                        Right,
                                        Specific,
                                        Duration
                                    )
                                ))
                            )),
                            p: Box::new(Minus(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.5245639573330924
                                    )
                                )),
                                Box::new(Base(
                                    Sum
                                ))
                            ))
                        }),
                        Box::new(Fixed(
                            -692388.9453011274
                        ))
                    ))
                ))
            )),
            p: Box::new(Fixed(
                750907.8101147992
            ))
        }.optimize(),
        money_strategy: Min(
            Box::new(Minus(
                Box::new(Max(
                    Box::new(Multiply(
                        Box::new(Divide(
                            Box::new(Base(
                                Sum
                            )),
                            Box::new(IfThenElse(
                                And(
                                    Box::new(True),
                                    Box::new(GreaterEqual(
                                        Character(
                                            Left,
                                            All,
                                            BetAmount
                                        ),
                                        Character(
                                            Left,
                                            All,
                                            Winrate
                                        )
                                    ))
                                ),
                                Box::new(Base(
                                    Sum
                                )),
                                Box::new(Base(
                                    Character(
                                        Left,
                                        All,
                                        Winrate
                                    )
                                ))
                            ))
                        )),
                        Box::new(Fixed(
                            0.532508946804858
                        ))
                    )),
                    Box::new(Plus(
                        Box::new(Multiply(
                            Box::new(Percentage(
                                genetic::Percentage(
                                    0.0414460491421409
                                )
                            )),
                            Box::new(Base(
                                Sum
                            ))
                        )),
                        Box::new(Base(
                            Sum
                        ))
                    ))
                )),
                Box::new(Max(
                    Box::new(Base(
                        Sum
                    )),
                    Box::new(Min(
                        Box::new(Base(
                            Sum
                        )),
                        Box::new(Percentage(
                            genetic::Percentage(
                                0.5057426950497094
                            )
                        ))
                    ))
                ))
            )),
            Box::new(Multiply(
                Box::new(Average(
                    Box::new(IfThenElse(
                        Lesser(
                            Character(
                                Left,
                                All,
                                Duration
                            ),
                            Sum
                        ),
                        Box::new(Tier {
                            x: Box::new(Fixed(
                                -172718.8580098679
                            )),
                            s: Box::new(Divide(
                                Box::new(Base(
                                    Sum
                                )),
                                Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        BetAmount
                                    )
                                ))
                            )),
                            a: Box::new(Fixed(
                                0.831625876879555
                            )),
                            b: Box::new(Percentage(
                                genetic::Percentage(
                                    0.8287304141580899
                                )
                            )),
                            p: Box::new(Fixed(
                                -1388719.2850521852
                            ))
                        }),
                        Box::new(Bezier(
                            CubicBezierSegment {
                                from: Point {
                                    x: -514120.70496685966,
                                    y: 349447.7986859039
                                },
                                ctrl1: Point {
                                    x: 416241.1214321129,
                                    y: 772362.1733482637
                                },
                                ctrl2: Point {
                                    x: -347894.30878267233,
                                    y: 892776.7776328623
                                },
                                to: Point {
                                    x: -697699.0109735497,
                                    y: -23139.099566670375
                                }
                            },
                            Box::new(IfThenElse(
                                Greater(
                                    Sum,
                                    Character(
                                        Right,
                                        Specific,
                                        MatchesLen
                                    )
                                ),
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.7423890094379099
                                    )
                                )),
                                Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        MatchesLen
                                    )
                                ))
                            ))
                        ))
                    )),
                    Box::new(Multiply(
                        Box::new(Multiply(
                            Box::new(Fixed(
                                -275581.5729857625
                            )),
                            Box::new(Min(
                                Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        Earnings
                                    )
                                )),
                                Box::new(Fixed(
                                    -314394.25262951414
                                ))
                            ))
                        )),
                        Box::new(Base(
                            Sum
                        ))
                    ))
                )),
                Box::new(Base(
                    Sum
                ))
            ))
        ).optimize()
    };






    let tournament_strategy = BetStrategy {
        fitness: 3464814909.0,
        successes: 5343.0,
        failures: 6209.0,
        record_len: 91468.0,
        characters_len: 7972,
        max_character_len: 86,
        bet_strategy: Lesser(
            Multiply(
                Box::new(Min(
                    Box::new(Tier {
                        x: Box::new(Bezier(
                            CubicBezierSegment {
                                from: Point {
                                    x: -125939.73758775856,
                                    y: 558306.0765623606
                                },
                                ctrl1: Point {
                                    x: -546298.5175292832,
                                    y: -941743.1333357835
                                },
                                ctrl2: Point {
                                    x: 299987.45374034665,
                                    y: -846944.7657732507
                                },
                                to: Point {
                                    x: 477908.2888921151,
                                    y: 792472.7967653895
                                }
                            },
                            Box::new(Base(
                                Sum
                            ))
                        )),
                        s: Box::new(Fixed(
                            494042.2131760126
                        )),
                        a: Box::new(Fixed(
                            -979758.5702110073
                        )),
                        b: Box::new(Plus(
                            Box::new(Fixed(
                                0.9931955466379329
                            )),
                            Box::new(Tier {
                                x: Box::new(Abs(
                                    Box::new(Base(
                                        Sum
                                    ))
                                )),
                                s: Box::new(Percentage(
                                    genetic::Percentage(
                                        0.16199667087000294
                                    )
                                )),
                                a: Box::new(Fixed(
                                    -228084.10041304628
                                )),
                                b: Box::new(Bezier(
                                    CubicBezierSegment {
                                        from: Point {
                                            x: 881195.7438158233,
                                            y: 54440.07169953813
                                        },
                                        ctrl1: Point {
                                            x: 638861.5796083921,
                                            y: 64281.32179142287
                                        },
                                        ctrl2: Point {
                                            x: 16850.31309062679,
                                            y: 341663.06757682486
                                        },
                                        to: Point {
                                            x: 696006.8709268479,
                                            y: -226956.09109931058
                                        }
                                    },
                                    Box::new(Base(
                                        Sum
                                    ))
                                )),
                                p: Box::new(Fixed(
                                    0.5251080933279956
                                ))
                            })
                        )),
                        p: Box::new(Tier {
                            x: Box::new(IfThenElse(
                                LesserEqual(
                                    Character(
                                        Right,
                                        All,
                                        Favored
                                    ),
                                    Sum
                                ),
                                Box::new(Minus(
                                    Box::new(Base(
                                        Character(
                                            Left,
                                            All,
                                            Earnings
                                        )
                                    )),
                                    Box::new(Base(
                                        Character(
                                            Right,
                                            All,
                                            Upsets
                                        )
                                    ))
                                )),
                                Box::new(Tier {
                                    x: Box::new(Base(
                                        Character(
                                            Left,
                                            All,
                                            Winrate
                                        )
                                    )),
                                    s: Box::new(Percentage(
                                        genetic::Percentage(
                                            0.6554229192329973
                                        )
                                    )),
                                    a: Box::new(Percentage(
                                        genetic::Percentage(
                                            0.6116608725592111
                                        )
                                    )),
                                    b: Box::new(Percentage(
                                        genetic::Percentage(
                                            0.19920648456154047
                                        )
                                    )),
                                    p: Box::new(Percentage(
                                        genetic::Percentage(
                                            0.24479615419311054
                                        )
                                    ))
                                })
                            )),
                            s: Box::new(Minus(
                                Box::new(Fixed(
                                    385248.67813513475
                                )),
                                Box::new(Multiply(
                                    Box::new(Percentage(
                                        genetic::Percentage(
                                            0.44857156829173844
                                        )
                                    )),
                                    Box::new(Base(
                                        Character(
                                            Right,
                                            All,
                                            BetAmount
                                        )
                                    ))
                                ))
                            )),
                            a: Box::new(Plus(
                                Box::new(Minus(
                                    Box::new(Percentage(
                                        genetic::Percentage(
                                            0.5693947247413959
                                        )
                                    )),
                                    Box::new(Base(
                                        Character(
                                            Left,
                                            All,
                                            Odds
                                        )
                                    ))
                                )),
                                Box::new(Tier {
                                    x: Box::new(Percentage(
                                        genetic::Percentage(
                                            0.26203418568393394
                                        )
                                    )),
                                    s: Box::new(Fixed(
                                        -540876.4520127631
                                    )),
                                    a: Box::new(Fixed(
                                        -604853.7699231465
                                    )),
                                    b: Box::new(Base(
                                        Sum
                                    )),
                                    p: Box::new(Fixed(
                                        399524.56038283324
                                    ))
                                })
                            )),
                            b: Box::new(Average(
                                Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        Earnings
                                    )
                                )),
                                Box::new(Divide(
                                    Box::new(Base(
                                        Sum
                                    )),
                                    Box::new(Percentage(
                                        genetic::Percentage(
                                            0.4496524655666723
                                        )
                                    ))
                                ))
                            )),
                            p: Box::new(Divide(
                                Box::new(Tier {
                                    x: Box::new(Base(
                                        Character(
                                            Right,
                                            Specific,
                                            Duration
                                        )
                                    )),
                                    s: Box::new(Base(
                                        Sum
                                    )),
                                    a: Box::new(Fixed(
                                        -426883.15839831135
                                    )),
                                    b: Box::new(Percentage(
                                        genetic::Percentage(
                                            0.9966598247776367
                                        )
                                    )),
                                    p: Box::new(Fixed(
                                        -680670.8141180149
                                    ))
                                }),
                                Box::new(Fixed(
                                    120895570911.13152
                                ))
                            ))
                        })
                    }),
                    Box::new(Divide(
                        Box::new(Tier {
                            x: Box::new(Fixed(
                                -489460.93057019747
                            )),
                            s: Box::new(Fixed(
                                -931956.0543221628
                            )),
                            a: Box::new(Minus(
                                Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        Winrate
                                    )
                                )),
                                Box::new(Fixed(
                                    82875.54905337591
                                ))
                            )),
                            b: Box::new(Fixed(
                                -75347.71380509381
                            )),
                            p: Box::new(Percentage(
                                genetic::Percentage(
                                    0.4991753238399313
                                )
                            ))
                        }),
                        Box::new(Average(
                            Box::new(Tier {
                                x: Box::new(Fixed(
                                    -25875170642.165882
                                )),
                                s: Box::new(Base(
                                    Sum
                                )),
                                a: Box::new(IfThenElse(
                                    Lesser(
                                        Character(
                                            Right,
                                            All,
                                            BetAmount
                                        ),
                                        Character(
                                            Left,
                                            All,
                                            Winrate
                                        )
                                    ),
                                    Box::new(Percentage(
                                        genetic::Percentage(
                                            0.0664850197893543
                                        )
                                    )),
                                    Box::new(Fixed(
                                        703306.3192054636
                                    ))
                                )),
                                b: Box::new(IfThenElse(
                                    GreaterEqual(
                                        Character(
                                            Left,
                                            Specific,
                                            Earnings
                                        ),
                                        Character(
                                            Right,
                                            Specific,
                                            BetAmount
                                        )
                                    ),
                                    Box::new(Fixed(
                                        593190.6347782079
                                    )),
                                    Box::new(Percentage(
                                        genetic::Percentage(
                                            0.511182081038494
                                        )
                                    ))
                                )),
                                p: Box::new(Fixed(
                                    -71609.35325486262
                                ))
                            }),
                            Box::new(Min(
                                Box::new(Minus(
                                    Box::new(Fixed(
                                        257691.20569695602
                                    )),
                                    Box::new(Base(
                                        Sum
                                    ))
                                )),
                                Box::new(Divide(
                                    Box::new(Base(
                                        Sum
                                    )),
                                    Box::new(Percentage(
                                        genetic::Percentage(
                                            0.6652469637370692
                                        )
                                    ))
                                ))
                            ))
                        ))
                    ))
                )),
                Box::new(Abs(
                    Box::new(Bezier(
                        CubicBezierSegment {
                            from: Point {
                                x: -849233.5411807073,
                                y: 536871.442345981
                            },
                            ctrl1: Point {
                                x: 571276.2791722527,
                                y: -984162.5867313852
                            },
                            ctrl2: Point {
                                x: -949334.501893556,
                                y: -2629.4735207254007
                            },
                            to: Point {
                                x: 751302.0589147923,
                                y: -891840.8342130894
                            }
                        },
                        Box::new(Divide(
                            Box::new(Bezier(
                                CubicBezierSegment {
                                    from: Point {
                                        x: 791905.9140242244,
                                        y: 370680.5974740821
                                    },
                                    ctrl1: Point {
                                        x: -414863.7615699266,
                                        y: 163783.61588699985
                                    },
                                    ctrl2: Point {
                                        x: 634981.2934599012,
                                        y: -376348.8446572641
                                    },
                                    to: Point {
                                        x: -641990.8003703516,
                                        y: -607965.5019197996
                                    }
                                },
                                Box::new(Max(
                                    Box::new(Base(
                                        Sum
                                    )),
                                    Box::new(Percentage(
                                        genetic::Percentage(
                                            0.7338732107996203
                                        )
                                    ))
                                ))
                            )),
                            Box::new(Minus(
                                Box::new(Divide(
                                    Box::new(Base(
                                        Sum
                                    )),
                                    Box::new(Base(
                                        Character(
                                            Right,
                                            Specific,
                                            Odds
                                        )
                                    ))
                                )),
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.8300690787305854
                                    )
                                ))
                            ))
                        ))
                    ))
                ))
            ),
            Fixed(
                -880108.081781311
            )
        ).optimize(),
        prediction_strategy: Multiply(
            Box::new(Plus(
                Box::new(Multiply(
                    Box::new(Multiply(
                        Box::new(Plus(
                            Box::new(Fixed(
                                0.11838313321420314
                            )),
                            Box::new(Multiply(
                                Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        BetAmount
                                    )
                                )),
                                Box::new(Fixed(
                                    -405554.8162589807
                                ))
                            ))
                        )),
                        Box::new(Tier {
                            x: Box::new(Fixed(
                                -436331.4258511975
                            )),
                            s: Box::new(Fixed(
                                645089.9604987475
                            )),
                            a: Box::new(IfThenElse(
                                Lesser(
                                    Character(
                                        Right,
                                        All,
                                        Upsets
                                    ),
                                    Character(
                                        Left,
                                        All,
                                        BetAmount
                                    )
                                ),
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.7978095640411195
                                    )
                                )),
                                Box::new(Fixed(
                                    -388960.362994521
                                ))
                            )),
                            b: Box::new(Tier {
                                x: Box::new(Fixed(
                                    313112.0894934327
                                )),
                                s: Box::new(Percentage(
                                    genetic::Percentage(
                                        0.19809683690388205
                                    )
                                )),
                                a: Box::new(Fixed(
                                    -839045.9695737254
                                )),
                                b: Box::new(Fixed(
                                    -55942.617686365884
                                )),
                                p: Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        Upsets
                                    )
                                ))
                            }),
                            p: Box::new(Fixed(
                                -852617.124347188
                            ))
                        })
                    )),
                    Box::new(Multiply(
                        Box::new(Fixed(
                            823619.2381865528
                        )),
                        Box::new(Base(
                            Sum
                        ))
                    ))
                )),
                Box::new(Minus(
                    Box::new(Divide(
                        Box::new(Tier {
                            x: Box::new(Tier {
                                x: Box::new(Base(
                                    Character(
                                        Left,
                                        Specific,
                                        Odds
                                    )
                                )),
                                s: Box::new(Fixed(
                                    373932.37592413067
                                )),
                                a: Box::new(Percentage(
                                    genetic::Percentage(
                                        0.08704726547287313
                                    )
                                )),
                                b: Box::new(Percentage(
                                    genetic::Percentage(
                                        0.9263019524736106
                                    )
                                )),
                                p: Box::new(Percentage(
                                    genetic::Percentage(
                                        0.05519622390614876
                                    )
                                ))
                            }),
                            s: Box::new(Fixed(
                                595231.0958594468
                            )),
                            a: Box::new(Base(
                                Character(
                                    Right,
                                    All,
                                    Favored
                                )
                            )),
                            b: Box::new(Average(
                                Box::new(Fixed(
                                    206478.516909258
                                )),
                                Box::new(Base(
                                    Sum
                                ))
                            )),
                            p: Box::new(Base(
                                Sum
                            ))
                        }),
                        Box::new(Plus(
                            Box::new(Minus(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.042269388160969486
                                    )
                                )),
                                Box::new(Base(
                                    Sum
                                ))
                            )),
                            Box::new(Min(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.3644376218490993
                                    )
                                )),
                                Box::new(Base(
                                    Character(
                                        Right,
                                        All,
                                        Earnings
                                    )
                                ))
                            ))
                        ))
                    )),
                    Box::new(Plus(
                        Box::new(Max(
                            Box::new(Average(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.9485122859316665
                                    )
                                )),
                                Box::new(Base(
                                    Sum
                                ))
                            )),
                            Box::new(Max(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.028296304154386446
                                    )
                                )),
                                Box::new(Base(
                                    Character(
                                        Right,
                                        Specific,
                                        Duration
                                    )
                                ))
                            ))
                        )),
                        Box::new(Fixed(
                            723121.9531532457
                        ))
                    ))
                ))
            )),
            Box::new(Plus(
                Box::new(Fixed(
                    -873192484884625800000000.0
                )),
                Box::new(Multiply(
                    Box::new(Abs(
                        Box::new(Minus(
                            Box::new(Base(
                                Character(
                                    Right,
                                    All,
                                    Duration
                                )
                            )),
                            Box::new(Percentage(
                                genetic::Percentage(
                                    0.12057496809388703
                                )
                            ))
                        ))
                    )),
                    Box::new(Average(
                        Box::new(IfThenElse(
                            GreaterEqual(
                                Character(
                                    Right,
                                    All,
                                    Upsets
                                ),
                                Character(
                                    Left,
                                    All,
                                    Favored
                                )
                            ),
                            Box::new(Fixed(
                                -475484.5565565787
                            )),
                            Box::new(Fixed(
                                425956.50656123087
                            ))
                        )),
                        Box::new(Plus(
                            Box::new(Base(
                                Character(
                                    Right,
                                    All,
                                    Duration
                                )
                            )),
                            Box::new(Percentage(
                                genetic::Percentage(
                                    0.627572205490094
                                )
                            ))
                        ))
                    ))
                ))
            ))
        ).optimize(),
        money_strategy: Bezier(
            CubicBezierSegment {
                from: Point {
                    x: -747458.7628762603,
                    y: -403111.990811987
                },
                ctrl1: Point {
                    x: 288277.5618894706,
                    y: 477730.95399280876
                },
                ctrl2: Point {
                    x: 409304.1675801608,
                    y: -567997.4576738664
                },
                to: Point {
                    x: 355536.31747081724,
                    y: -505095.8256285543
                }
            },
            Box::new(Bezier(
                CubicBezierSegment {
                    from: Point {
                        x: -711416.5856136662,
                        y: 489714.04431534914
                    },
                    ctrl1: Point {
                        x: 498434.01048957434,
                        y: -692853.0704836051
                    },
                    ctrl2: Point {
                        x: -690999.6768571509,
                        y: 457783.7248892396
                    },
                    to: Point {
                        x: -781559.5956864354,
                        y: 164862.76375961208
                    }
                },
                Box::new(Max(
                    Box::new(Min(
                        Box::new(Fixed(
                            605026.4853693934
                        )),
                        Box::new(Base(
                            Character(
                                Right,
                                Specific,
                                Upsets
                            )
                        ))
                    )),
                    Box::new(Multiply(
                        Box::new(Fixed(
                            0.9310506343380435
                        )),
                        Box::new(Min(
                            Box::new(Average(
                                Box::new(Percentage(
                                    genetic::Percentage(
                                        0.40670389622638053
                                    )
                                )),
                                Box::new(Base(
                                    Character(
                                        Right,
                                        All,
                                        Winrate
                                    )
                                ))
                            )),
                            Box::new(Bezier(
                                CubicBezierSegment {
                                    from: Point {
                                        x: 420244.65518223384,
                                        y: -910257.3582252034
                                    },
                                    ctrl1: Point {
                                        x: 284945.7718377433,
                                        y: 764092.782751615
                                    },
                                    ctrl2: Point {
                                        x: -911518.546410833,
                                        y: -335712.7264578018
                                    },
                                    to: Point {
                                        x: -274025.8370849741,
                                        y: -236396.47271071142
                                    }
                                },
                                Box::new(Base(
                                    Character(
                                        Left,
                                        All,
                                        Upsets
                                    )
                                ))
                            ))
                        ))
                    ))
                ))
            ))
        ).optimize()
    };



    let records = {
        let data = include_str!("../records/saltyRecordsM--2018-1-16-14.29.txt");
        record::parse_csv(&data).unwrap()
    };

    let mut simulation: simulation::Simulation<BetStrategy, BetStrategy> = simulation::Simulation::new();

    simulation.matchmaking_strategy = Some(matchmaking_strategy2);
    simulation.tournament_strategy = Some(tournament_strategy2);

    log!("Running...");

    simulation.simulate(records);

    //write_strategy("strategies/matchmaking_strategy", &matchmaking_strategy)?;
    //write_strategy("strategies/tournament_strategy", &tournament_strategy)?;

    log!("fitness: {:#?},\nsuccesses: {:#?},\nfailures: {:#?},\nrecord_len: {:#?},\ncharacters_len: {:#?},\nmax_character_len: {:#?},",
        simulation.sum,
        simulation.successes,
        simulation.failures,
        simulation.record_len,
        simulation.characters.len(),
        simulation.max_character_len
    );


    Ok(())
}*/


#[cfg(any(target_arch = "wasm32", target_arch = "asmjs"))]
fn main() {

}


#[cfg(not(any(target_arch = "wasm32", target_arch = "asmjs")))]
fn main() {
    //stdweb::initialize();

    /*let bezier = CubicBezierSegment {
        from: Point::new(0.83253485,0.018677153),
        ctrl1: Point::new(0.08993364,0.018677153),
        ctrl2: Point::new(0.46272424,0.018678138),
        to: Point::new(0.65694433,0.018677153)
    };

    log!("{:#?}", bezier.sample_y(0.0));
    log!("{:#?}", bezier.sample_y(0.1));
    log!("{:#?}", bezier.sample_y(0.2));
    log!("{:#?}", bezier.sample_y(0.3));
    log!("{:#?}", bezier.sample_y(0.4));
    log!("{:#?}", bezier.sample_y(0.5));
    log!("{:#?}", bezier.sample_y(0.6));
    log!("{:#?}", bezier.sample_y(0.7));
    log!("{:#?}", bezier.sample_y(0.8));
    log!("{:#?}", bezier.sample_y(0.9));
    log!("{:#?}", bezier.sample_y(1.0));*/

    run_simulation().unwrap();
    //write_file("tmp").unwrap();

    /*log!("{:#?}", records);*/

    /*log!("{:#?}", (2, "hi"));*/

    //log!("{:#?}", "hi!");

    //stdweb::event_loop();
}
