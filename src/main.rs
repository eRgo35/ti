mod app;
mod args;
mod cache;
mod font;
mod interface;
mod pomodoro;
mod timer;

use app::App;
use args::{Args, Command};
use cache::Cache;
use clap::{error::ErrorKind, CommandFactory};
use font::load_font;
use interface::{handle_interface, Outcome};
use std::{
    process::{self, ExitCode},
    time::Instant,
};
use timer::time_to_countdown;

fn main() -> ExitCode {
    let args = Args::try_parse_args(std::env::args_os()).unwrap_or_else(|err| err.exit());

    let now = Instant::now();
    let cache;
    let app = match &args.command {
        Some(Command::Pomodoro(pomodoro)) => {
            cache = Cache::new(args.cache, "ti_pomodoro.tmp");
            App::pomodoro(pomodoro.config(), &cache, args.clear, now)
        }
        None => {
            let (hours, minutes, seconds) =
                args.time
                    .unwrap_or((args.hours, args.minutes, args.seconds));
            let Some(original) = time_to_countdown(hours, minutes, seconds) else {
                Args::command()
                    .error(ErrorKind::ValueValidation, "the total time is too large")
                    .exit();
            };
            cache = Cache::new(args.cache, "ti_countdown.tmp");
            App::countdown(original, &cache, args.clear, now)
        }
    };

    let font = load_font(&args.font).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        process::exit(1);
    });

    // The terminal is restored by the time handle_interface returns.
    match handle_interface(app, font) {
        Ok(Outcome::Quit) => {
            println!("\nQuitting...");
            ExitCode::SUCCESS
        }
        Ok(Outcome::Interrupted) => {
            println!();
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
