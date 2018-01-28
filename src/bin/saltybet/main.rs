extern crate stdweb;
extern crate serde_json;
extern crate salty_bet_bot;

use salty_bet_bot::common::{parse_f64, parse_money, Port, create_tab, Message, Information, get_text_content};
use salty_bet_bot::simulation::{Bet};
use stdweb::web::{document, set_timeout, INode};


pub fn observe_changes() {
    create_tab(|| {
        println!("Tab created");

        let port = Port::new("saltybet");

        fn send(port: Port) {
            let left_bettors_illuminati = document().query_selector_all("#bettors1 > p.bettor-line > strong.goldtext").len() as f64;
            let right_bettors_illuminati = document().query_selector_all("#bettors2 > p.bettor-line > strong.goldtext").len() as f64;

            let left_bettors_normal = document().query_selector_all("#bettors1 > p.bettor-line > strong:not(.goldtext)").len() as f64;
            let right_bettors_normal = document().query_selector_all("#bettors2 > p.bettor-line > strong:not(.goldtext)").len() as f64;

            let current_balance = document().query_selector("#balance")
                .and_then(get_text_content)
                .and_then(|x| parse_f64(x.as_str()));

            let wager_box = document().query_selector("#wager");

            let left_button = document().query_selector("#player1:enabled");

            let right_button = document().query_selector("#player2:enabled");

            let status = document().query_selector("#betstatus")
                .and_then(get_text_content)
                .map(|x| x == "Bets are locked until the next match.")
                .unwrap_or(false);

            if status &&
               document().query_selector("#sbettors1 > span.redtext > span.counttext").is_some() &&
               document().query_selector("#sbettors2 > span.bluetext > span.counttext").is_some() {

                let left_bet = document().query_selector("#lastbet > span:first-of-type.redtext")
                    .and_then(get_text_content)
                    .and_then(|x| parse_money(x.as_str()));

                let right_bet = document().query_selector("#lastbet > span:first-of-type.bluetext")
                    .and_then(get_text_content)
                    .and_then(|x| parse_money(x.as_str()));

                println!("Bets: {:#?} {:#?}", left_bet, right_bet);

                port.send_message(&serde_json::to_string(&Message::Information(Information {
                    // TODO handle the situation where the player is Illuminati
                    player_is_illuminati: false,
                    left_bettors_illuminati,
                    right_bettors_illuminati,
                    left_bettors_normal,
                    right_bettors_normal,
                    bet: match left_bet {
                        Some(left) => match right_bet {
                            None => Bet::Left(left),
                            Some(_) => unreachable!(),
                        },
                        None => match right_bet {
                            Some(right) => Bet::Right(right),
                            None => Bet::None,
                        },
                    }
                })).unwrap());

                println!("Message sent");
            }

            current_balance.and_then(|current_balance|
            wager_box.and_then(|wager_box|
            left_button.and_then(|left_button|
            right_button.map(|right_button| {
            }))));

            set_timeout(|| send(port), 5000);
        }

        send(port);
    });
}


fn main() {
    stdweb::initialize();

    observe_changes();

    stdweb::event_loop();
}
