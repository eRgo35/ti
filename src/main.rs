mod args;
mod cache;
mod font;
mod timer;

use std::{
    io::{stdout, Write},
    process::exit,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crossterm::{
    cursor::{Hide, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{self, Clear, ClearType},
    QueueableCommand,
};

use args::Args;
use cache::Cache;
use clap::Parser;
use figlet_rs::FIGfont;
use font::load_font;
use timer::Timer;

const FONT_WIDTH: usize = 9;
const REFRESH_RATE: Duration = Duration::from_millis(100);

fn draw_timer(time: &str, font: &FIGfont) {
    let figure = font.convert(time).unwrap();

    // TODO: Calculate figure width and height properly
    let figure_width = (time.len() * FONT_WIDTH) as u16;
    let figure_height = figure.to_string().lines().count() as u16;

    let (terminal_width, terminal_height) = terminal::size().unwrap();

    let x = terminal_width
        .saturating_sub(figure_width)
        .saturating_div(2);
    let y = terminal_height
        .saturating_sub(figure_height)
        .saturating_div(2);

    let mut stdout = stdout();

    execute!(stdout, Hide).unwrap();

    stdout.queue(Clear(ClearType::All)).unwrap();

    for (i, line) in figure.to_string().lines().enumerate() {
        stdout.queue(MoveTo(x, y + i as u16)).unwrap();
        stdout.queue(Print(line)).unwrap();
    }

    stdout.queue(MoveTo(0, terminal_height)).unwrap();

    stdout
        .queue(Print("SPACE [Pause/Resume] | R [Reset] | Q [Quit]"))
        .unwrap();

    stdout.flush().unwrap();
}

fn handle_exit(code: i32) {
    terminal::disable_raw_mode().unwrap();
    println!();
    exit(code);
}

fn handle_keyboard() -> Option<char> {
    if let Ok(Event::Key(KeyEvent {
        code, modifiers, ..
    })) = event::read()
    {
        match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => handle_exit(1),
            (KeyCode::Char(c), _) => return Some(c.to_ascii_lowercase()),
            _ => {}
        }
    }
    None
}

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

    terminal::enable_raw_mode().unwrap();

    loop {
        if event::poll(REFRESH_RATE).unwrap() {
            if let Some(key) = handle_keyboard() {
                let mut timer = timer.lock().unwrap();
                match key {
                    'q' => {
                        timer.stop();
                        break;
                    }
                    's' => timer.start(),
                    'p' => timer.pause(),
                    ' ' => timer.toggle(),
                    'r' => timer.reset(),
                    _ => {}
                }
            }
        }

        let time = timer.lock().unwrap().print();
        draw_timer(&time, &font);
    }

    countdown_thread.join().unwrap();
    cache.clear();

    terminal::disable_raw_mode().unwrap();
    println!();
}
