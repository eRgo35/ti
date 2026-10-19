mod args;
mod cache;
mod font;
mod interface;
mod timer;

use args::Args;
use cache::Cache;
use clap::{error::ErrorKind, CommandFactory, Parser};
use font::load_font;
use interface::handle_interface;
use std::time::Instant;
use timer::{time_to_countdown, Timer};

fn main() {
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
    let font = load_font(args.font);
    let now = Instant::now();
    let timer = if args.clear {
        Timer::new(original, now)
    } else {
        cache
            .load()
            .map(|cached| Timer::from_cache(cached, original, now))
            .unwrap_or_else(|| Timer::new(original, now))
    };

    handle_interface(timer, &cache, font);
}
