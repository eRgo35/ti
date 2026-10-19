use std::{
    io::{stdout, Write},
    process::exit,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{Hide, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    QueueableCommand,
};
use figlet_rs::FIGfont;

use crate::{cache::Cache, timer::Timer};

/// Upper bound on how long the loop sleeps when nothing is due, so the UI
/// stays responsive even while the timer is paused or finished.
const IDLE_POLL: Duration = Duration::from_millis(250);
const FONT_WIDTH: usize = 9;

fn handle_exit(code: i32) -> ! {
    terminal::disable_raw_mode().unwrap();
    println!();
    exit(code);
}

fn handle_keyboard(
    KeyEvent {
        code, modifiers, ..
    }: KeyEvent,
) -> Option<char> {
    match (code, modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => handle_exit(1),
        (KeyCode::Char(c), _) => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

fn draw_timer(timer: &Timer, font: &FIGfont) {
    let time = timer.print();
    let figure = font.convert(&time).unwrap();

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

    if timer.is_paused() {
        execute!(stdout, SetForegroundColor(Color::DarkGrey)).unwrap();
    }

    for (i, line) in figure.to_string().lines().enumerate() {
        stdout.queue(MoveTo(x, y + i as u16)).unwrap();
        stdout.queue(Print(line)).unwrap();
    }

    execute!(stdout, SetForegroundColor(Color::Reset)).unwrap();

    stdout.queue(MoveTo(0, terminal_height)).unwrap();

    if timer.is_paused() {
        execute!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("TIMER PAUSED"),
            SetForegroundColor(Color::Reset),
            Print(" | SPACE [Resume] | R [Reset] | Q [Quit]",)
        )
        .unwrap();
    } else if timer.is_finished() {
        execute!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("TIMER FINISHED"),
            SetForegroundColor(Color::Reset),
            Print(" | R [Reset] | Q [Quit]"),
        )
        .unwrap();
    } else {
        execute!(stdout, Print("SPACE [Pause] | R [Reset] | Q [Quit]")).unwrap();
    }

    stdout.flush().unwrap();
}

pub fn handle_interface(mut timer: Timer, cache: &Cache, font: FIGfont) {
    terminal::enable_raw_mode().unwrap();

    let mut dirty = true;

    loop {
        let now = Instant::now();

        if timer.tick(now) {
            cache.save(timer.countdown());
            dirty = true;
        }

        if dirty {
            draw_timer(&timer, &font);
            dirty = false;
        }

        let timeout = timer
            .until_next_tick(now)
            .map_or(IDLE_POLL, |d| d.min(IDLE_POLL));

        if event::poll(timeout).unwrap() {
            let now = Instant::now();
            match event::read().unwrap() {
                Event::Resize(..) => dirty = true,
                Event::Key(event) => {
                    if let Some(key) = handle_keyboard(event) {
                        match key {
                            'q' => break,
                            's' => timer.start(now),
                            'p' => timer.pause(now),
                            ' ' => timer.toggle(now),
                            'r' => {
                                timer.reset(now);
                                cache.clear();
                            }
                            _ => {}
                        }
                        dirty = true;
                    }
                }
                _ => {}
            }
        }
    }

    terminal::disable_raw_mode().unwrap();
    println!("\nQuitting...");
}
