mod args;
mod cache;
mod font;
mod interface;
mod timer;

use args::Args;
use cache::Cache;
use clap::{error::ErrorKind, CommandFactory, Parser};
use font::load_font;
use interface::{handle_interface, Outcome};
use std::{
    process::{self, ExitCode},
    time::Instant,
};
use timer::{time_to_countdown, Timer};

fn main() -> ExitCode {
    let args = Args::parse();

    let (hours, minutes, seconds) = args
        .time
        .unwrap_or((args.hours, args.minutes, args.seconds));

    let Some(original) = time_to_countdown(hours, minutes, seconds) else {
        Args::command()
            .error(ErrorKind::ValueValidation, "the total time is too large")
            .exit();
    };

    let cache = Cache::new(args.cache);
    let font = load_font(&args.font).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        process::exit(1);
    });
    let now = Instant::now();
    let timer = if args.clear {
        Timer::new(original, now)
    } else {
        cache
            .load()
            .map(|cached| Timer::from_cache(cached, original, now))
            .unwrap_or_else(|| Timer::new(original, now))
    };

    // The terminal is restored by the time handle_interface returns.
    match handle_interface(timer, &cache, font) {
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
