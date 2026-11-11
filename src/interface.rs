use std::{
    io::{self, stdout, Write},
    panic,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use figlet_rs::FIGlet;

use crate::{
    app::{App, Update},
    pomodoro::Pomodoro,
    timer::Timer,
};

/// Upper bound on how long the loop sleeps when nothing is due, so the UI
/// stays responsive even while the timer is paused or finished.
const IDLE_POLL: Duration = Duration::from_millis(250);

/// How the interface ended.
pub enum Outcome {
    Quit,
    Interrupted,
}

enum Input {
    Interrupt,
    Char(char),
}

/// Puts the terminal into raw mode with a hidden cursor and restores it when
/// dropped, on every exit path including errors and panics.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let guard = Self;
        queue!(stdout(), Hide)?;
        stdout().flush()?;

        // Restore first so the panic message is not printed in raw mode.
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            restore_terminal();
            previous(info);
        }));

        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Best effort and idempotent: there is nothing useful to do if the terminal
/// refuses to be restored.
fn restore_terminal() {
    let mut out = stdout();
    let _ = queue!(out, ResetColor, Show);
    let _ = out.flush();
    let _ = terminal::disable_raw_mode();
}

/// Maps a key event to an input. Only presses count: Windows (and terminals
/// using the kitty keyboard protocol) also report repeats and releases, which
/// would otherwise trigger every action twice.
fn handle_keyboard(
    KeyEvent {
        code,
        modifiers,
        kind,
        ..
    }: KeyEvent,
) -> Option<Input> {
    if kind != KeyEventKind::Press {
        return None;
    }

    match (code, modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Input::Interrupt),
        (KeyCode::Char(c), _) => Some(Input::Char(c.to_ascii_lowercase())),
        _ => None,
    }
}

/// Lines of the big clock. Falls back to plain text if the font cannot render
/// the time.
fn render_lines(font: &FIGlet, time: &str) -> Vec<String> {
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

fn status_segments(timer: &Timer, pomodoro: Option<&Pomodoro>) -> [(Color, String); 2] {
    if let Some(pomodoro) = pomodoro {
        let label = pomodoro.label();
        return if timer.is_paused() {
            [
                (Color::DarkGrey, format!("{label} PAUSED")),
                (
                    Color::Reset,
                    " | SPACE [Resume] | R [Reset] | N [Skip] | Q [Quit]".into(),
                ),
            ]
        } else {
            [
                (phase_color(pomodoro), label),
                (
                    Color::Reset,
                    " | SPACE [Pause] | R [Reset] | N [Skip] | Q [Quit]".into(),
                ),
            ]
        };
    }

    if timer.is_finished() {
        [
            (Color::Red, "TIMER FINISHED".into()),
            (Color::Reset, " | R [Reset] | Q [Quit]".into()),
        ]
    } else if timer.is_paused() {
        [
            (Color::DarkGrey, "TIMER PAUSED".into()),
            (
                Color::Reset,
                " | SPACE [Resume] | R [Reset] | Q [Quit]".into(),
            ),
        ]
    } else {
        [
            (Color::Reset, "SPACE [Pause] | R [Reset] | Q [Quit]".into()),
            (Color::Reset, String::new()),
        ]
    }
}

/// Breaks are drawn in green so they stand out from work at a glance.
fn phase_color(pomodoro: &Pomodoro) -> Color {
    if pomodoro.is_break() {
        Color::Green
    } else {
        Color::Reset
    }
}

/// The first `max` characters of `text`.
fn clip(text: &str, max: usize) -> &str {
    match text.char_indices().nth(max) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}

/// Draws the whole screen for a `width` x `height` terminal.
fn draw_timer(
    out: &mut impl Write,
    app: &App,
    font: &FIGlet,
    (width, height): (u16, u16),
) -> io::Result<()> {
    let timer = app.timer();
    let pomodoro = app.pomodoro_state();
    let lines = render_lines(font, &timer.print());
    let (x, y) = centered_origin(&lines, width, height);
    let status_row = height.saturating_sub(1);

    queue!(out, Clear(ClearType::All))?;

    if timer.is_paused() {
        queue!(out, SetForegroundColor(Color::DarkGrey))?;
    } else if let Some(pomodoro) = pomodoro {
        queue!(out, SetForegroundColor(phase_color(pomodoro)))?;
    }

    // Keep the figure inside the terminal and off the status row so a small
    // window never wraps or scrolls.
    for (row, line) in (y..status_row).zip(&lines) {
        queue!(
            out,
            MoveTo(x, row),
            Print(clip(line, usize::from(width - x)))
        )?;
    }

    queue!(out, SetForegroundColor(Color::Reset), MoveTo(0, status_row))?;

    let mut remaining = usize::from(width);
    for (color, text) in status_segments(timer, pomodoro) {
        let text = clip(&text, remaining);
        remaining -= text.chars().count();
        queue!(out, SetForegroundColor(color), Print(text))?;
    }
    queue!(out, SetForegroundColor(Color::Reset))?;

    out.flush()
}

pub fn handle_interface(mut app: App, font: FIGlet) -> io::Result<Outcome> {
    let _guard = TerminalGuard::new()?;
    let mut out = stdout();

    let mut dirty = true;

    loop {
        let now = Instant::now();

        match app.tick(now) {
            Update::Unchanged => {}
            Update::Changed => dirty = true,
            Update::PhaseEnded => {
                // Terminal bell.
                queue!(out, Print('\u{7}'))?;
                dirty = true;
            }
        }

        if dirty {
            draw_timer(&mut out, &app, &font, terminal::size()?)?;
            dirty = false;
        }

        let timeout = app
            .until_next_tick(now)
            .map_or(IDLE_POLL, |d| d.min(IDLE_POLL));

        if !event::poll(timeout)? {
            continue;
        }

        let now = Instant::now();
        match event::read()? {
            Event::Resize(..) => dirty = true,
            Event::Key(event) => match handle_keyboard(event) {
                Some(Input::Interrupt) => return Ok(Outcome::Interrupted),
                Some(Input::Char(key)) => {
                    if app.key(key, now) {
                        return Ok(Outcome::Quit);
                    }
                    dirty = true;
                }
                None => {}
            },
            _ => {}
        }
    }
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

    fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> KeyEvent {
        KeyEvent::new_with_kind(code, modifiers, kind)
    }

    #[test]
    fn key_press_is_handled() {
        let press = key(KeyCode::Char(' '), KeyModifiers::NONE, KeyEventKind::Press);
        assert!(matches!(handle_keyboard(press), Some(Input::Char(' '))));
        let upper = key(KeyCode::Char('Q'), KeyModifiers::SHIFT, KeyEventKind::Press);
        assert!(matches!(handle_keyboard(upper), Some(Input::Char('q'))));
        let ctrl_c = key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert!(matches!(handle_keyboard(ctrl_c), Some(Input::Interrupt)));
    }

    #[test]
    fn key_release_and_repeat_are_ignored() {
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            assert!(handle_keyboard(key(KeyCode::Char(' '), KeyModifiers::NONE, kind)).is_none());
            assert!(
                handle_keyboard(key(KeyCode::Char('c'), KeyModifiers::CONTROL, kind)).is_none()
            );
        }
    }

    #[test]
    fn non_char_keys_are_ignored() {
        let enter = key(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Press);
        assert!(handle_keyboard(enter).is_none());
    }

    fn status_text(timer: &Timer, pomodoro: Option<&Pomodoro>) -> String {
        status_segments(timer, pomodoro)
            .into_iter()
            .map(|(_, text)| text)
            .collect()
    }

    const CONFIG: crate::pomodoro::Config = crate::pomodoro::Config {
        work: 60,
        short_break: 30,
        long_break: 90,
        rounds: 2,
    };

    #[test]
    fn countdown_status_line() {
        let t0 = Instant::now();
        let mut timer = Timer::new(5, t0);
        assert_eq!(
            status_text(&timer, None),
            "SPACE [Pause] | R [Reset] | Q [Quit]"
        );
        timer.pause(t0);
        assert_eq!(
            status_text(&timer, None),
            "TIMER PAUSED | SPACE [Resume] | R [Reset] | Q [Quit]"
        );
        let finished = Timer::new(0, t0);
        assert_eq!(
            status_text(&finished, None),
            "TIMER FINISHED | R [Reset] | Q [Quit]"
        );
    }

    #[test]
    fn pomodoro_status_line_shows_phase_and_round() {
        let t0 = Instant::now();
        let mut pomodoro = Pomodoro::new(CONFIG);
        let mut timer = Timer::new(60, t0);
        assert_eq!(
            status_text(&timer, Some(&pomodoro)),
            "WORK 1/2 | SPACE [Pause] | R [Reset] | N [Skip] | Q [Quit]"
        );
        timer.pause(t0);
        assert_eq!(
            status_text(&timer, Some(&pomodoro)),
            "WORK 1/2 PAUSED | SPACE [Resume] | R [Reset] | N [Skip] | Q [Quit]"
        );
        pomodoro.advance();
        assert_eq!(
            status_segments(&timer, Some(&pomodoro))[0].1,
            "BREAK 1/2 PAUSED"
        );
    }

    #[test]
    fn breaks_are_green_and_work_is_default() {
        let t0 = Instant::now();
        let timer = Timer::new(60, t0);
        let mut pomodoro = Pomodoro::new(CONFIG);
        assert_eq!(phase_color(&pomodoro), Color::Reset);
        assert_eq!(status_segments(&timer, Some(&pomodoro))[0].0, Color::Reset);
        pomodoro.advance(); // break
        assert_eq!(phase_color(&pomodoro), Color::Green);
        assert_eq!(status_segments(&timer, Some(&pomodoro))[0].0, Color::Green);
        pomodoro.advance(); // work 2
        pomodoro.advance(); // long break
        assert_eq!(pomodoro.label(), "LONG BREAK");
        assert_eq!(phase_color(&pomodoro), Color::Green);
    }

    /// The escape sequence `queue!` writes for `command`.
    fn ansi(command: impl crossterm::Command) -> String {
        let mut buf = Vec::new();
        queue!(buf, command).unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// Draws `app` into a buffer as a `width` x `height` terminal.
    fn draw(app: &App, size: (u16, u16)) -> String {
        let font = crate::font::load_font("").unwrap();
        let mut buf = Vec::new();
        draw_timer(&mut buf, app, &font, size).unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// A cache that drawing never touches.
    fn unused_cache() -> crate::cache::Cache {
        let path = std::env::temp_dir().join(format!("ti_test_{}_draw", std::process::id()));
        crate::cache::Cache::new(path.to_string_lossy().into_owned(), "unused")
    }

    #[test]
    fn draw_puts_status_line_on_last_row() {
        let cache = unused_cache();
        let app = App::countdown(65, &cache, true, Instant::now());
        let screen = draw(&app, (120, 30));
        let status_row = screen.find(&ansi(MoveTo(0, 29))).expect("status row");
        let status = screen
            .find("SPACE [Pause] | R [Reset] | Q [Quit]")
            .expect("status text");
        assert!(status_row < status, "{screen:?}");
        // The 72x7 clock is centred: its first row starts at (24, 11).
        assert!(screen.contains(&ansi(MoveTo(24, 11))), "{screen:?}");
    }

    #[test]
    fn draw_clips_status_line() {
        let cache = unused_cache();
        let app = App::countdown(65, &cache, true, Instant::now());
        let screen = draw(&app, (9, 30));
        assert!(screen.contains("SPACE [Pa"), "{screen:?}");
        assert!(!screen.contains("SPACE [Pau"), "{screen:?}");

        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &cache, true, t0);
        app.key('p', t0);
        // 8 chars of "WORK 1/2 PAUSED", nothing of the next segment.
        let screen = draw(&app, (8, 30));
        assert!(screen.contains("WORK 1/2"), "{screen:?}");
        assert!(!screen.contains("WORK 1/2 "), "{screen:?}");
        assert!(!screen.contains("SPACE"), "{screen:?}");
    }

    #[test]
    fn draw_keeps_clock_off_the_status_row() {
        let cache = unused_cache();
        let app = App::countdown(65, &cache, true, Instant::now());
        // A 3-row terminal: the 5-row figure gets rows 0 and 1 only.
        let screen = draw(&app, (200, 3));
        assert!(screen.contains(&ansi(MoveTo(64, 0))), "{screen:?}");
        assert!(screen.contains(&ansi(MoveTo(64, 1))), "{screen:?}");
        assert!(!screen.contains(&ansi(MoveTo(64, 2))), "{screen:?}");
        assert!(screen.contains(&ansi(MoveTo(0, 2))), "{screen:?}");
    }

    #[test]
    fn draw_colours_breaks_green_and_pauses_grey() {
        let cache = unused_cache();
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &cache, true, t0);
        let green = ansi(SetForegroundColor(Color::Green));
        let grey = ansi(SetForegroundColor(Color::DarkGrey));
        assert!(!draw(&app, (120, 30)).contains(&green));

        app.key('n', t0); // break
        let screen = draw(&app, (120, 30));
        // The clock and the label are both green.
        assert_eq!(screen.matches(&green).count(), 2, "{screen:?}");

        app.key('p', t0);
        let screen = draw(&app, (120, 30));
        assert!(!screen.contains(&green), "{screen:?}");
        assert!(screen.contains(&grey), "{screen:?}");
    }

    #[test]
    fn draw_survives_a_zero_size_terminal() {
        let cache = unused_cache();
        let app = App::countdown(65, &cache, true, Instant::now());
        for size in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            draw(&app, size);
        }
    }

    #[test]
    fn clip_counts_chars() {
        assert_eq!(clip("abc", 5), "abc");
        assert_eq!(clip("abc", 2), "ab");
        assert_eq!(clip("█████", 3), "███");
        assert_eq!(clip("abc", 0), "");
    }
}
