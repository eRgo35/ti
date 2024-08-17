use std::{
    io::{stdout, Write},
    process::exit,
    sync::{Arc, Mutex},
    time::Duration,
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

const REFRESH_RATE: Duration = Duration::from_millis(100);
const FONT_WIDTH: usize = 9;

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

fn draw_timer(timer: &Arc<Mutex<Timer>>, font: &FIGfont) {
    let time = timer.lock().unwrap().print();
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

    if timer.lock().unwrap().is_paused() {
        execute!(stdout, SetForegroundColor(Color::DarkGrey)).unwrap();
    }

    for (i, line) in figure.to_string().lines().enumerate() {
        stdout.queue(MoveTo(x, y + i as u16)).unwrap();
        stdout.queue(Print(line)).unwrap();
    }

    execute!(stdout, SetForegroundColor(Color::Reset)).unwrap();

    stdout.queue(MoveTo(0, terminal_height)).unwrap();

    if timer.lock().unwrap().is_paused() {
        execute!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("TIMER PAUSED"),
            SetForegroundColor(Color::Reset),
            Print(" | SPACE [Resume] | R [Reset] | Q [Quit]",)
        )
        .unwrap();
    } else if timer.lock().unwrap().is_finished() {
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

pub fn handle_interface(timer: Arc<Mutex<Timer>>, cache: Arc<Mutex<Cache>>, font: FIGfont) {
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
                    'r' => {
                        timer.reset();
                        cache.lock().unwrap().clear();
                    }
                    _ => {}
                }
            }
        }

        draw_timer(&timer, &font);
    }

    terminal::disable_raw_mode().unwrap();
    println!("\nQuitting...");
}
