mod args;
mod cache;
mod font;
mod interface;
mod timer;

use args::Args;
use cache::Cache;
use clap::Parser;
use font::load_font;
use interface::handle_interface;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use timer::Timer;

fn main() {
    let args = Args::parse();

    let mut hours = args.hours;
    let mut minutes = args.minutes;
    let mut seconds = args.seconds;

    if args.time.is_some() {
        let time = args.time.unwrap();
        let mut parts = time.split(':').map(|part| part.parse::<u64>().unwrap());
        hours = parts.next().unwrap_or(0);
        minutes = parts.next().unwrap_or(0);
        seconds = parts.next().unwrap_or(0);
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
