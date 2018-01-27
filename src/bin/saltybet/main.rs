extern crate stdweb;
extern crate serde_json;
extern crate salty_bet_bot;

use salty_bet_bot::common::{parse_f64, parse_money, Port, create_tab, Message, Information};
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

            let left_bet = document().query_selector("#lastbet > span:first-of-type.redtext")
                .and_then(|x| x.text_content())
                .and_then(|x| parse_money(x.as_str()));

            let right_bet = document().query_selector("#lastbet > span:first-of-type.bluetext")
                .and_then(|x| x.text_content())
                .and_then(|x| parse_money(x.as_str()));

            let current_balance = document().query_selector("#balance")
                .and_then(|x| x.text_content())
                .and_then(|x| parse_f64(x.as_str()));

            let wager_box = document().query_selector("#wager");

            let left_button = document().query_selector("#player1:enabled");

            let right_button = document().query_selector("#player2:enabled");

            println!("{:#?} {:#?} {:#?} {:#?}", left_bettors_illuminati, left_bettors_normal, right_bettors_illuminati, right_bettors_normal);

            println!("{:#?} {:#?} {:#?}", left_bet, right_bet, current_balance);

            println!("{:#?} {:#?} {:#?}", wager_box, left_button, right_button);

            if left_bettors_illuminati != 0.0 ||
               right_bettors_illuminati != 0.0 ||
               left_bettors_normal != 0.0 ||
               right_bettors_normal != 0.0 {

                // TODO handle the situation where the player is Illuminati
                let left_bettors_normal = left_bettors_normal - match left_bet {
                    Some(_) => 1.0,
                    None => 0.0,
                };

                // TODO handle the situation where the player is Illuminati
                let right_bettors_normal = right_bettors_normal - match right_bet {
                    Some(_) => 1.0,
                    None => 0.0,
                };

                port.send_message(&serde_json::to_string(&Message::Information(Information {
                    left_bettors_illuminati,
                    right_bettors_illuminati,
                    left_bettors_normal,
                    right_bettors_normal,
                    bet: match left_bet {
                        Some(left) => Bet::Left(left),
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
