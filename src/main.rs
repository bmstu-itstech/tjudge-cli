use clap::{Arg, ArgAction, Command, value_parser};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::game::{Game, GameError};
use crate::games::*;
use crate::subprocess_player::SubprocessPlayer;

mod debug;
mod game;
mod games;
mod subprocess_player;

const IO_ERROR_CODE: i32 = 3;

#[derive(clap::ValueEnum, Clone, Debug)]
#[clap(rename_all = "snake_case")]
enum GameName {
    Dilemma,
    TugOfWar,
}

fn main() -> ExitCode {
    let matches = build_cli().get_matches();

    if matches.get_flag("verbose") {
        debug::set_verbose(true);
    }

    let game: Box<dyn Game> = match matches.get_one::<GameName>("game").unwrap() {
        GameName::Dilemma => Box::new(PrisonerDilemma::default()),
        GameName::TugOfWar => Box::new(TugOfWar::default()),
    };

    let mut l = SubprocessPlayer::from_program(matches.get_one::<PathBuf>("program_left").unwrap())
        .unwrap_or_else(|e| {
            eprintln!("failed to init left player: {}", e);
            std::process::exit(IO_ERROR_CODE)
        });

    let mut r =
        SubprocessPlayer::from_program(matches.get_one::<PathBuf>("program_right").unwrap())
            .unwrap_or_else(|e| {
                eprintln!("failed to init right player: {}", e);
                std::process::exit(IO_ERROR_CODE)
            });

    let iters = *matches.get_one::<u32>("iters").unwrap();

    let res = game.round(&mut l, &mut r, iters);
    match res {
        Ok(res) => {
            println!("{} {}", res.0, res.1);
            ExitCode::SUCCESS
        }
        Err(err) => match err {
            GameError::ErrorLeft(why) => {
                eprintln!("{}", why);
                ExitCode::from(1)
            }
            GameError::ErrorRight(why) => {
                eprintln!("{}", why);
                ExitCode::from(2)
            }
        },
    }
}

fn build_cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("game")
                .required(true)
                .index(1)
                .value_parser(value_parser!(GameName)),
        )
        .arg(
            Arg::new("program_left")
                .required(true)
                .index(2)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("program_right")
                .required(true)
                .index(3)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .action(ArgAction::SetTrue)
                .help("Enable verbose output"),
        )
        .arg(
            Arg::new("iters")
                .short('i')
                // Хотелось бы вынести в константу u32, но тогда нужно использовать format!().
                // String сам по себе не удовлетворяет трейту, а as_str() есть ссылка на droppable
                // данные. Поэтому... так
                .default_value("10")
                .value_parser(value_parser!(u32))
                .help("Number of runs of each program within the game"),
        )
}
