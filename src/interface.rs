use std::{
    io::{stdout, Write},
    process::exit,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{Hide, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    QueueableCommand,
};
use figlet_rs::FIGfont;

use crate::{cache::Cache, timer::Timer};

/// Upper bound on how long the loop sleeps when nothing is due, so the UI
/// stays responsive even while the timer is paused or finished.
const IDLE_POLL: Duration = Duration::from_millis(250);

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

/// Lines of the big clock. Falls back to plain text if the font cannot render
/// the time.
fn render_lines(font: &FIGfont, time: &str) -> Vec<String> {
    let lines: Vec<String> = font
        .convert(time)
        .map(|figure| figure.to_string().lines().map(str::to_owned).collect())
        .unwrap_or_default();

    if lines.is_empty() {
        vec![time.to_owned()]
    } else {
        lines
    }
}

/// Top-left corner that centers `lines` in a `width` x `height` terminal.
fn centered_origin(lines: &[String], width: u16, height: u16) -> (u16, u16) {
    let figure_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let figure_width = u16::try_from(figure_width).unwrap_or(u16::MAX);
    let figure_height = u16::try_from(lines.len()).unwrap_or(u16::MAX);

    (
        width.saturating_sub(figure_width) / 2,
        height.saturating_sub(figure_height) / 2,
    )
}

fn status_segments(timer: &Timer) -> [(Color, &'static str); 2] {
    if timer.is_finished() {
        [
            (Color::Red, "TIMER FINISHED"),
            (Color::Reset, " | R [Reset] | Q [Quit]"),
        ]
    } else if timer.is_paused() {
        [
            (Color::DarkGrey, "TIMER PAUSED"),
            (Color::Reset, " | SPACE [Resume] | R [Reset] | Q [Quit]"),
        ]
    } else {
        [
            (Color::Reset, "SPACE [Pause] | R [Reset] | Q [Quit]"),
            (Color::Reset, ""),
        ]
    }
}

/// The first `max` characters of `text`.
fn clip(text: &str, max: usize) -> &str {
    match text.char_indices().nth(max) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}

fn draw_timer(timer: &Timer, font: &FIGfont) {
    let lines = render_lines(font, &timer.print());
    let (width, height) = terminal::size().unwrap();
    let (x, y) = centered_origin(&lines, width, height);
    let status_row = height.saturating_sub(1);

    let mut stdout = stdout();

    stdout.queue(Hide).unwrap();
    stdout.queue(Clear(ClearType::All)).unwrap();

    if timer.is_paused() {
        stdout.queue(SetForegroundColor(Color::DarkGrey)).unwrap();
    }

    // Keep the figure inside the terminal and off the status row so a small
    // window never wraps or scrolls.
    for (row, line) in (y..status_row).zip(&lines) {
        stdout.queue(MoveTo(x, row)).unwrap();
        stdout
            .queue(Print(clip(line, usize::from(width - x))))
            .unwrap();
    }

    stdout.queue(SetForegroundColor(Color::Reset)).unwrap();
    stdout.queue(MoveTo(0, status_row)).unwrap();

    let mut remaining = usize::from(width);
    for (color, text) in status_segments(timer) {
        let text = clip(text, remaining);
        remaining -= text.chars().count();
        stdout.queue(SetForegroundColor(color)).unwrap();
        stdout.queue(Print(text)).unwrap();
    }
    stdout.queue(SetForegroundColor(Color::Reset)).unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(rows: &[&str]) -> Vec<String> {
        rows.iter().map(|r| (*r).to_owned()).collect()
    }

    #[test]
    fn origin_uses_widest_line_in_chars() {
        // 3 chars wide but 7 bytes: width must be measured in chars.
        let figure = lines(&["█", "███", "█"]);
        assert_eq!(centered_origin(&figure, 11, 9), (4, 3));
    }

    #[test]
    fn origin_centers_plain_text() {
        assert_eq!(centered_origin(&lines(&["00:00:00"]), 20, 5), (6, 2));
    }

    #[test]
    fn origin_saturates_in_tiny_terminals() {
        let figure = lines(&["abcdef", "abcdef", "abcdef"]);
        assert_eq!(centered_origin(&figure, 2, 1), (0, 0));
        assert_eq!(centered_origin(&figure, 0, 0), (0, 0));
        assert_eq!(centered_origin(&[], 0, 0), (0, 0));
    }

    #[test]
    fn render_falls_back_to_plain_text_when_font_has_no_glyphs() {
        let font = crate::font::load_font("").unwrap();
        // The embedded font has no glyph for these characters.
        assert_eq!(render_lines(&font, "\u{1F600}"), lines(&["\u{1F600}"]));
        assert_eq!(render_lines(&font, ""), lines(&[""]));
    }

    #[test]
    fn render_default_font_is_multi_line() {
        let font = crate::font::load_font("").unwrap();
        assert!(render_lines(&font, "00:00:00").len() > 1);
    }

    #[test]
    fn clip_counts_chars() {
        assert_eq!(clip("abc", 5), "abc");
        assert_eq!(clip("abc", 2), "ab");
        assert_eq!(clip("█████", 3), "███");
        assert_eq!(clip("abc", 0), "");
    }
}
