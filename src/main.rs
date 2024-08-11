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
    let Args {
        hours,
        minutes,
        seconds,
        font,
        cache,
    } = Args::parse();

    let cache = Cache::new(cache);
    let font = load_font(font);
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
