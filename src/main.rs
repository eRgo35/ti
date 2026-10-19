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
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use timer::{time_to_countdown, Timer};

fn main() {
    let args = Args::parse();

    let (hours, minutes, seconds) = args
        .time
        .unwrap_or((args.hours, args.minutes, args.seconds));

    if time_to_countdown(hours, minutes, seconds).is_none() {
        Args::command()
            .error(ErrorKind::ValueValidation, "the total time is too large")
            .exit();
    }

    let cache = Arc::new(Mutex::new(Cache::new(args.cache)));
    let font = load_font(args.font);
    let timer = Arc::new(Mutex::new(if args.clear {
        Timer::new(hours, minutes, seconds)
    } else {
        cache
            .lock()
            .unwrap()
            .load()
            .map(|cached_countdown| Timer::from_cache(cached_countdown, hours, minutes, seconds))
            .unwrap_or_else(|| Timer::new(hours, minutes, seconds))
    }));

    // TODO: Synchronize timer reset calls with the countdown thread
    let timer_clone = Arc::clone(&timer);
    let cache_clone = Arc::clone(&cache);
    let countdown_thread = thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let mut timer = timer_clone.lock().unwrap();

        if timer.is_finished() || timer.is_stopped() {
            break;
        }

        timer.tick();
        cache_clone.lock().unwrap().save(timer.countdown());
    });

    handle_interface(timer, cache, font);

    countdown_thread.join().unwrap();
}
