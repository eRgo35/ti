mod args;

use std::{
    fs::File,
    io::{stdout, Read, Write},
    path::PathBuf,
    process::exit,
    thread::sleep,
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
use clap::Parser;
use figlet_rs::FIGfont;

const FONT_WIDTH: usize = 9;
const REFRESH_RATE: Duration = Duration::from_millis(100);

fn time_to_seconds(hours: u64, minutes: u64, seconds: u64) -> u64 {
    hours * 3600 + minutes * 60 + seconds
}

fn draw_timer(seconds: u64, font: &FIGfont) {
    let time = format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    );

    let figure = font.convert(&time).unwrap();
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
        .queue(Print("SPC [Pause/Resume] | R [Reset] | Q [Quit]"))
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

fn preserve_countdown_state(cache_file: &PathBuf, countdown: u64) {
    let state = format!("{}", countdown);
    let mut file = File::create_new(cache_file).unwrap_or(File::create(cache_file).unwrap());
    file.write_all(state.as_bytes()).unwrap();
}

fn retrieve_countdown_state(cache_file: &PathBuf) -> Option<u64> {
    let mut file = File::open(cache_file).ok()?;
    let mut countdown = String::new();
    file.read_to_string(&mut countdown).ok()?;

    let countdown = countdown.trim().parse::<u64>().ok()?;

    Some(countdown)
}

fn clear_countdown_state(cache_file: &PathBuf) {
    let _ = std::fs::remove_file(cache_file);
}

fn load_font(font: String) -> FIGfont {
    let default_font = FIGfont::standard().unwrap();
    let ansi_mono = std::include_str!("ANSI_Mono.flf");
    let ansi_mono_font = FIGfont::from_content(ansi_mono).unwrap_or(default_font);

    FIGfont::from_file(&font).unwrap_or(ansi_mono_font)
}

fn load_cache(cache: String) -> PathBuf {
    if cache.is_empty() {
        std::env::temp_dir().join("ti_countdown.tmp")
    } else {
        PathBuf::from(cache)
    }
}

fn main() {
    let Args {
        hours,
        minutes,
        seconds,
        font,
        cache,
    } = Args::parse();

    let cache: PathBuf = load_cache(cache);
    let font = load_font(font);
    let mut countdown: u64 =
        retrieve_countdown_state(&cache).unwrap_or(time_to_seconds(hours, minutes, seconds));

    terminal::enable_raw_mode().unwrap();
    let mut paused = false;

    loop {
        if event::poll(REFRESH_RATE).unwrap() {
            if let Some(key) = handle_keyboard() {
                match key {
                    'q' => break,
                    's' => {
                        paused = false;
                    }
                    'p' => {
                        paused = true;
                    }
                    ' ' => {
                        paused = !paused;
                    }
                    'r' => {
                        countdown = time_to_seconds(hours, minutes, seconds);
                    }
                    _ => {}
                }
            }
        }

        draw_timer(countdown, &font);

        if !paused {
            sleep(Duration::from_secs(1));
            if countdown == 0 {
                break;
            }
            countdown -= 1;
            preserve_countdown_state(&cache, countdown);
        }
    }

    clear_countdown_state(&cache);

    terminal::disable_raw_mode().unwrap();
    println!();
}
