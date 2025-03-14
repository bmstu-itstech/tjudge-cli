use std::env;
use std::process::ExitCode;
use crate::dilemma::PrisonerDilemma;
use crate::game::{Game, GameError};
use crate::program_player::ProgramPlayer;

mod game;
mod dilemma;
mod program_player;

fn main() -> ExitCode {
    if env::args().len() != 4 {
        eprintln!("Usage: {} <game> <program1> <program2>", env::args().nth(0).unwrap());
        return ExitCode::from(2);
    }
    
    let game = env::args().nth(1).unwrap();
    let game = match game.as_str() {
        "dilemma" => PrisonerDilemma::default(),
        _ => {
            eprintln!("unknown game '{}', expected one of ['dilemma']", game);
            return ExitCode::FAILURE;
        }
    };

    let mut l = ProgramPlayer::new(env::args().nth(2).unwrap().as_str()).unwrap();
    let mut r = ProgramPlayer::new(env::args().nth(3).unwrap().as_str()).unwrap();
    
    let res = game.round(&mut l, &mut r, 5);
    match res {
        Ok(res) => {
            println!("{} {}", res.0, res.1);
            ExitCode::SUCCESS
        }
        Err(err) => {
            match err {
                GameError::ErrorLeft(why) => {
                    eprintln!("error left: {}", why);
                    ExitCode::from(1)
                }
                GameError::ErrorRight(why) => {
                    eprintln!("error right: {}", why);
                    ExitCode::from(2)
                }
            }
        }
    }
}
