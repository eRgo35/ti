//! End-to-end checks of the interactive screen.
//!
//! Each test starts the real `ti` binary in its own pseudo-terminal, so the
//! screen never takes over the terminal running `cargo test`. Every wait has
//! a deadline and the child is killed when a test ends, so a failing test
//! can't hang the suite. Each test also uses its own cache file, so the tests
//! can run in parallel.

use std::{
    env, fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

const DEADLINE: Duration = Duration::from_secs(10);
const BELL: &[u8] = b"\x07";
const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
const SHOW_CURSOR: &[u8] = b"\x1b[?25h";
/// Reverse video: the screen flashes with every bell.
const REVERSE: &[u8] = b"\x1b[7m";
/// Clearing the screen starts every frame.
const CLEAR: &[u8] = b"\x1b[2J";
/// crossterm's `Color::Green` as a foreground colour.
const GREEN: &[u8] = b"\x1b[38;5;10m";
/// A terminal asking for the cursor position (ConPTY does this on start).
const CURSOR_QUERY: &[u8] = b"\x1b[6n";

/// `ti` running in a pseudo-terminal.
struct Screen {
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    /// How much of `output` the `expect` calls so far have consumed.
    seen: usize,
    cursor_queries_answered: usize,
    name: String,
    cache: PathBuf,
}

/// A cache path of its own for each test.
fn cache_path(name: &str) -> PathBuf {
    env::temp_dir().join(format!("ti_screen_test_{}_{name}", std::process::id()))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

impl Screen {
    /// Starts `ti --cache <own file> args...` in a `cols` x `rows` terminal.
    /// The cache starts as `seed`, or absent.
    fn start(name: &str, seed: Option<&str>, args: &[&str], (cols, rows): (u16, u16)) -> Self {
        let cache = cache_path(name);
        match seed {
            Some(text) => fs::write(&cache, text).unwrap(),
            None => {
                let _ = fs::remove_file(&cache);
            }
        }

        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_ti"));
        command.arg("--cache");
        command.arg(&cache);
        command.args(args);
        let child = pair.slave.spawn_command(command).unwrap();
        // Only the child keeps the slave open, so reads end when it exits.
        drop(pair.slave);

        let output = Arc::new(Mutex::new(Vec::new()));
        let mut reader = pair.master.try_clone_reader().unwrap();
        let sink = Arc::clone(&output);
        thread::spawn(move || {
            let mut buf = [0; 4096];
            while let Ok(n @ 1..) = reader.read(&mut buf) {
                sink.lock().unwrap().extend_from_slice(&buf[..n]);
            }
        });

        Self {
            child,
            writer: pair.master.take_writer().unwrap(),
            _master: pair.master,
            output,
            seen: 0,
            cursor_queries_answered: 0,
            name: name.to_owned(),
            cache,
        }
    }

    fn output(&self) -> Vec<u8> {
        self.output.lock().unwrap().clone()
    }

    fn dump(&self) -> String {
        String::from_utf8_lossy(&self.output())
            .escape_debug()
            .to_string()
    }

    fn answer_cursor_queries(&mut self) {
        let queries = count(&self.output(), CURSOR_QUERY);
        while self.cursor_queries_answered < queries {
            self.send(b"\x1b[1;1R");
            self.cursor_queries_answered += 1;
        }
    }

    /// Waits until `needle` appears after everything `expect` matched
    /// before, and moves past it.
    fn expect(&mut self, needle: &[u8]) {
        let deadline = Instant::now() + DEADLINE;
        loop {
            self.answer_cursor_queries();
            let output = self.output();
            if let Some(at) = find(&output[self.seen..], needle) {
                self.seen += at + needle.len();
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {:?} in {}",
                String::from_utf8_lossy(needle),
                self.dump()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Waits for the flash to end and checks that the screen is redrawn
    /// without reverse video.
    fn expect_flash_to_end(&mut self) {
        thread::sleep(Duration::from_millis(1500));
        let output = self.output();
        let frame = output
            .windows(CLEAR.len())
            .rposition(|window| window == CLEAR)
            .map_or(&output[..], |at| &output[at..]);
        assert!(
            find(frame, REVERSE).is_none(),
            "still flashing: {}",
            String::from_utf8_lossy(frame).escape_debug()
        );
    }

    /// Everything after the last `expect` match.
    fn unseen(&self) -> Vec<u8> {
        self.output()[self.seen..].to_vec()
    }

    fn send(&mut self, keys: &[u8]) {
        self.writer.write_all(keys).unwrap();
        self.writer.flush().unwrap();
    }

    /// Waits for `ti` to exit and returns its exit code.
    fn exit_code(&mut self) -> u32 {
        let deadline = Instant::now() + DEADLINE;
        loop {
            self.answer_cursor_queries();
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.exit_code();
            }
            assert!(
                Instant::now() < deadline,
                "ti did not exit: {}",
                self.dump()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn cache(&self) -> Option<String> {
        fs::read_to_string(&self.cache).ok()
    }

    /// Waits until the cache file holds `expected`.
    fn expect_cache(&mut self, expected: &str) {
        let deadline = Instant::now() + DEADLINE;
        while self.cache().as_deref() != Some(expected) {
            assert!(
                Instant::now() < deadline,
                "cache is {:?}, expected {expected:?}",
                self.cache()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Restarts `ti` on the same cache file, leaving it as it is.
    fn restart(self, args: &[&str]) -> Self {
        let (name, seed) = (self.name.clone(), self.cache());
        // Dropping removes the cache file, so it goes before the new start.
        drop(self);
        Self::start(&name, seed.as_deref(), args, SIZE)
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.cache);
    }
}

const SIZE: (u16, u16) = (120, 30);
/// 1 minute sessions, so a seeded cache decides when a phase ends; a long
/// break of 15 minutes (900 seconds) after every 3rd session.
const POMODORO: &[&str] = &[
    "pomodoro",
    "--work",
    "1",
    "--break",
    "1",
    "--long-break",
    "15",
    "--rounds",
    "3",
];

#[test]
fn countdown_finishes_with_bell_and_flash() {
    let mut screen = Screen::start("finishes", None, &["--clear", "-s", "2"], SIZE);
    screen.expect(HIDE_CURSOR);
    screen.expect(b"SPACE [Pause]");
    assert!(find(&screen.unseen(), BELL).is_none());
    screen.expect(BELL);
    screen.expect(REVERSE);
    screen.expect(b"TIMER FINISHED");
    screen.expect_cache("0");
    screen.expect_flash_to_end();
}

#[test]
fn reset_deletes_cache() {
    let mut screen = Screen::start("reset", None, &["--clear", "-s", "30"], SIZE);
    screen.expect_cache("29");
    // Paused, so no tick writes the cache again.
    screen.send(b" ");
    screen.expect(b"TIMER PAUSED");
    screen.send(b"r");
    screen.expect(b"TIMER PAUSED");
    thread::sleep(Duration::from_millis(1500));
    assert_eq!(screen.cache(), None);
}

#[test]
fn space_pauses_and_resumes() {
    let mut screen = Screen::start("space", None, &["--clear", "-s", "30"], SIZE);
    screen.expect_cache("29");
    screen.send(b" ");
    screen.expect(b"TIMER PAUSED");
    let paused = screen.cache();
    thread::sleep(Duration::from_millis(1500));
    assert_eq!(screen.cache(), paused, "a paused timer must not tick");
    screen.send(b" ");
    screen.expect(b"SPACE [Pause]");
}

#[test]
fn q_exits_0_and_restores_cursor() {
    let mut screen = Screen::start("quit", None, &["--clear", "-s", "30"], SIZE);
    screen.expect(HIDE_CURSOR);
    screen.expect(b"SPACE [Pause]");
    screen.send(b"q");
    assert_eq!(screen.exit_code(), 0);
    screen.expect(SHOW_CURSOR);
    screen.expect(b"Quitting...");
}

#[test]
fn countdown_resumes_from_cache() {
    let mut screen = Screen::start("resume", Some("75"), &["-s", "30"], SIZE);
    screen.expect_cache("74");
}

#[test]
fn ctrl_c_exits_1_and_keeps_cache() {
    let mut screen = Screen::start("ctrl_c", None, &["--clear", "-s", "30"], SIZE);
    screen.expect_cache("29");
    screen.send(b"\x03");
    assert_eq!(screen.exit_code(), 1);
    screen.expect(SHOW_CURSOR);
    assert!(screen.cache().is_some());
}

#[test]
fn pomodoro_resumes_seeded_cache() {
    let mut screen = Screen::start("pomo_resume", Some("work 2 30"), POMODORO, SIZE);
    screen.expect(b"WORK 2/3");
    screen.expect_cache("work 2 29");
}

#[test]
fn pomodoro_phase_end_rings_bell_flashes_and_turns_green() {
    let mut screen = Screen::start("pomo_bell", Some("work 2 2"), POMODORO, SIZE);
    screen.expect(b"WORK 2/3");
    assert!(find(&screen.unseen(), BELL).is_none());
    screen.expect(BELL);
    screen.expect(REVERSE);
    screen.expect(GREEN);
    screen.expect(b"BREAK 2/3");
    screen.expect_cache("break 2 59");
    screen.expect_flash_to_end();
    screen.expect(GREEN);
    screen.expect(b"BREAK 2/3");
}

#[test]
fn pomodoro_skip_pause_reset_and_resume() {
    let mut screen = Screen::start("pomo_keys", Some("work 2 30"), POMODORO, SIZE);
    screen.expect(b"WORK 2/3");
    screen.send(b"n");
    screen.expect(b"BREAK 2/3");
    screen.send(b"n");
    screen.expect(b"WORK 3/3");
    screen.send(b"n");
    screen.expect(b"LONG BREAK");
    screen.send(b"p");
    screen.expect(b"LONG BREAK PAUSED");
    screen.send(b"r");
    screen.expect_cache("long-break 3 900");
    screen.send(b"q");
    assert_eq!(screen.exit_code(), 0);

    // The session is saved and continues where it was.
    let mut screen = screen.restart(POMODORO);
    screen.expect(b"LONG BREAK");
    screen.expect_cache("long-break 3 899");
}

#[test]
fn pomodoro_clear_starts_fresh() {
    let mut screen = Screen::start(
        "pomo_clear",
        Some("long-break 3 100"),
        &[POMODORO, &["--clear"]].concat(),
        SIZE,
    );
    screen.expect(b"WORK 1/3");
    screen.expect_cache("work 1 59");
}

#[test]
fn tiny_terminal() {
    let mut screen = Screen::start("tiny", None, &["--clear", "-s", "30"], (10, 3));
    // The status line is clipped to the terminal's width.
    screen.expect(b"SPACE [Pau");
    screen.send(b"q");
    assert_eq!(screen.exit_code(), 0);
    assert!(
        find(&screen.output(), b"panicked").is_none(),
        "{}",
        screen.dump()
    );
}
