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

    let cache = Cache::new(args.cache);
    let font = load_font(args.font);
    let timer = Arc::new(Mutex::new(
        cache
            .load()
            .map(Timer::from_cache)
            .unwrap_or_else(|| Timer::new(hours, minutes, seconds)),
    ));

    let timer_clone = Arc::clone(&timer);
    let cache_clone = cache.clone();
    let countdown_thread = thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let mut timer = timer_clone.lock().unwrap();
        timer.tick();
        cache_clone.save(timer.countdown());
        if timer.is_finished() {
            break;
        }
    });

    handle_interface(timer, font);

    countdown_thread.join().unwrap();
    cache.clear();
}
