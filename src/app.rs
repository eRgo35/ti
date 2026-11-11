use std::time::{Duration, Instant};

use crate::{
    cache::Cache,
    pomodoro::{Config, Pomodoro},
    timer::Timer,
};

/// What a `tick` changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    Unchanged,
    Changed,
    /// A pomodoro phase ran out and the next one started.
    PhaseEnded,
}

/// Timer behaviour without the terminal: keys, ticks, pomodoro phases and
/// the cache. Every method takes `now` so it can be tested deterministically.
pub struct App<'a> {
    timer: Timer,
    pomodoro: Option<Pomodoro>,
    cache: &'a Cache,
}

impl<'a> App<'a> {
    /// A plain countdown of `original` seconds, resumed from the cache unless
    /// `clear` is set.
    pub fn countdown(original: u64, cache: &'a Cache, clear: bool, now: Instant) -> Self {
        let cached = if clear { None } else { cache.load() };
        let timer = match cached {
            Some(cached) => Timer::from_cache(cached, original, now),
            None => Timer::new(original, now),
        };

        Self {
            timer,
            pomodoro: None,
            cache,
        }
    }

    /// A pomodoro session, resumed from the cache unless `clear` is set or
    /// the cache does not hold a session that fits `config`.
    pub fn pomodoro(config: Config, cache: &'a Cache, clear: bool, now: Instant) -> Self {
        let cached = if clear {
            None
        } else {
            cache
                .load_text()
                .and_then(|text| Pomodoro::decode(config, &text))
        };
        let (pomodoro, timer) = match cached {
            Some((pomodoro, remaining)) => (
                pomodoro,
                Timer::from_cache(remaining, pomodoro.duration(), now),
            ),
            None => {
                let pomodoro = Pomodoro::new(config);
                (pomodoro, Timer::new(pomodoro.duration(), now))
            }
        };

        Self {
            timer,
            pomodoro: Some(pomodoro),
            cache,
        }
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    pub fn pomodoro_state(&self) -> Option<&Pomodoro> {
        self.pomodoro.as_ref()
    }

    pub fn until_next_tick(&self, now: Instant) -> Option<Duration> {
        self.timer.until_next_tick(now)
    }

    /// Advances the countdown to `now`. In pomodoro mode a finished phase
    /// starts the next one at the moment it ended, so time that ran past
    /// the end (a late loop) counts against the next phases.
    pub fn tick(&mut self, now: Instant) -> Update {
        let mut update = if self.timer.tick(now) {
            Update::Changed
        } else {
            Update::Unchanged
        };

        // Also catches a phase that a key press (pause) finished. Every
        // phase lasts at least a second, so this ends.
        if self.pomodoro.is_some() {
            while let Some(ended) = self.timer.finished_at() {
                self.next_phase(ended);
                self.timer.tick(now);
                update = Update::PhaseEnded;
            }
        }

        if update != Update::Unchanged {
            self.save();
        }
        update
    }

    /// Handles a lower-case key. Returns `true` if the app should quit.
    pub fn key(&mut self, key: char, now: Instant) -> bool {
        match key {
            'q' => return true,
            's' => self.timer.start(now),
            'p' => self.timer.pause(now),
            ' ' => self.timer.toggle(now),
            'r' => {
                self.timer.reset(now);
                if self.pomodoro.is_some() {
                    // Keep the phase and round for the next run.
                    self.save();
                } else {
                    self.cache.clear();
                }
            }
            'n' if self.pomodoro.is_some() => {
                self.next_phase(now);
                self.save();
            }
            _ => {}
        }

        false
    }

    /// Moves to the next pomodoro phase, starting it at `start`.
    fn next_phase(&mut self, start: Instant) {
        if let Some(pomodoro) = &mut self.pomodoro {
            pomodoro.advance();
            self.timer = Timer::new(pomodoro.duration(), start);
        }
    }

    fn save(&self) {
        match &self.pomodoro {
            Some(pomodoro) => self.cache.save(pomodoro.encode(self.timer.countdown())),
            None => self.cache.save(self.timer.countdown()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env::temp_dir, fs};

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /// A cache in its own temp file, removed when dropped.
    struct TempCache(Cache, std::path::PathBuf);

    impl TempCache {
        fn new(name: &str) -> Self {
            let path = temp_dir().join(format!("ti_test_{}_app_{name}", std::process::id()));
            let _ = fs::remove_file(&path);
            Self(
                Cache::new(path.to_string_lossy().into_owned(), "unused"),
                path,
            )
        }

        fn contents(&self) -> Option<String> {
            fs::read_to_string(&self.1).ok()
        }

        fn write(&self, text: &str) {
            fs::write(&self.1, text).unwrap();
        }
    }

    impl Drop for TempCache {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.1);
        }
    }

    /// 3s work, 1s break, 2s long break, 2 rounds: short enough to step
    /// through whole cycles second by second.
    const CONFIG: Config = Config {
        work: 3,
        short_break: 1,
        long_break: 2,
        rounds: 2,
    };

    fn label(app: &App) -> String {
        app.pomodoro_state().unwrap().label()
    }

    // --- plain countdown ---

    #[test]
    fn countdown_starts_fresh_without_cache() {
        let c = TempCache::new("cd_fresh");
        let app = App::countdown(10, &c.0, false, Instant::now());
        assert_eq!(app.timer().countdown(), 10);
        assert!(app.pomodoro_state().is_none());
    }

    #[test]
    fn countdown_resumes_from_cache_and_resets_to_original() {
        let c = TempCache::new("cd_resume");
        c.write("4");
        let t0 = Instant::now();
        let mut app = App::countdown(10, &c.0, false, t0);
        assert_eq!(app.timer().countdown(), 4);
        app.key('r', t0);
        assert_eq!(app.timer().countdown(), 10);
    }

    #[test]
    fn countdown_clear_ignores_cache() {
        let c = TempCache::new("cd_clear");
        c.write("4");
        let app = App::countdown(10, &c.0, true, Instant::now());
        assert_eq!(app.timer().countdown(), 10);
    }

    #[test]
    fn countdown_ignores_pomodoro_cache() {
        let c = TempCache::new("cd_pomo_cache");
        c.write("work 1 4");
        let app = App::countdown(10, &c.0, false, Instant::now());
        assert_eq!(app.timer().countdown(), 10);
    }

    #[test]
    fn countdown_saves_every_tick_and_finishes() {
        let c = TempCache::new("cd_ticks");
        let t0 = Instant::now();
        let mut app = App::countdown(2, &c.0, false, t0);
        assert_eq!(app.tick(t0), Update::Unchanged);
        assert_eq!(c.contents(), None, "nothing is written before a tick");
        assert_eq!(app.tick(t0 + secs(1)), Update::Changed);
        assert_eq!(c.contents().as_deref(), Some("1"));
        assert_eq!(app.tick(t0 + secs(2)), Update::Changed);
        assert_eq!(c.contents().as_deref(), Some("0"));
        assert!(app.timer().is_finished());
        assert_eq!(app.tick(t0 + secs(9)), Update::Unchanged);
    }

    #[test]
    fn countdown_reset_deletes_cache() {
        let c = TempCache::new("cd_reset");
        let t0 = Instant::now();
        let mut app = App::countdown(5, &c.0, false, t0);
        app.tick(t0 + secs(1));
        assert!(c.contents().is_some());
        assert!(!app.key('r', t0 + secs(1)));
        assert_eq!(c.contents(), None);
    }

    #[test]
    fn countdown_skip_key_does_nothing() {
        let c = TempCache::new("cd_skip");
        let t0 = Instant::now();
        let mut app = App::countdown(5, &c.0, false, t0);
        assert!(!app.key('n', t0));
        assert_eq!(app.timer().countdown(), 5);
        assert_eq!(c.contents(), None);
    }

    // --- keys ---

    #[test]
    fn quit_key_quits_and_others_do_not() {
        let c = TempCache::new("keys");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        for key in ['s', 'p', ' ', 'r', 'n', 'x', '1'] {
            assert!(!app.key(key, t0), "{key:?}");
        }
        assert!(app.key('q', t0));
    }

    #[test]
    fn pause_start_toggle_keys() {
        let c = TempCache::new("pause_keys");
        let t0 = Instant::now();
        let mut app = App::countdown(5, &c.0, false, t0);
        app.key('p', t0);
        assert!(app.timer().is_paused());
        assert_eq!(app.tick(t0 + secs(10)), Update::Unchanged);
        app.key('s', t0 + secs(10));
        assert!(!app.timer().is_paused());
        app.key(' ', t0 + secs(10));
        assert!(app.timer().is_paused());
        app.key(' ', t0 + secs(10));
        assert!(!app.timer().is_paused());
        assert_eq!(app.until_next_tick(t0 + secs(10)), Some(secs(1)));
    }

    // --- pomodoro ---

    #[test]
    fn pomodoro_starts_with_first_work_session() {
        let c = TempCache::new("pomo_fresh");
        let app = App::pomodoro(CONFIG, &c.0, false, Instant::now());
        assert_eq!(label(&app), "WORK 1/2");
        assert_eq!(app.timer().countdown(), 3);
    }

    #[test]
    fn pomodoro_runs_full_cycles_by_ticks() {
        let c = TempCache::new("pomo_cycle");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);

        // (second, update, label, countdown, cache) after ticking to t0+second.
        // The next phase starts on the tick that ends the previous one.
        let expected = [
            (1, Update::Changed, "WORK 1/2", 2, "work 1 2"),
            (2, Update::Changed, "WORK 1/2", 1, "work 1 1"),
            (3, Update::PhaseEnded, "BREAK 1/2", 1, "break 1 1"),
            (4, Update::PhaseEnded, "WORK 2/2", 3, "work 2 3"),
            (5, Update::Changed, "WORK 2/2", 2, "work 2 2"),
            (6, Update::Changed, "WORK 2/2", 1, "work 2 1"),
            (7, Update::PhaseEnded, "LONG BREAK", 2, "long-break 2 2"),
            (8, Update::Changed, "LONG BREAK", 1, "long-break 2 1"),
            (9, Update::PhaseEnded, "WORK 1/2", 3, "work 1 3"),
        ];
        for (second, update, phase, countdown, cache) in expected {
            let now = t0 + secs(second);
            assert_eq!(app.tick(now), update, "at {second}s");
            assert_eq!(label(&app), phase, "at {second}s");
            assert_eq!(app.timer().countdown(), countdown, "at {second}s");
            assert_eq!(c.contents().as_deref(), Some(cache), "at {second}s");
            assert!(!app.timer().is_finished(), "a phase never sits at 0");
        }
    }

    #[test]
    fn pomodoro_paused_phase_does_not_end() {
        let c = TempCache::new("pomo_paused");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        app.key('p', t0);
        assert_eq!(app.tick(t0 + secs(100)), Update::Unchanged);
        assert_eq!(label(&app), "WORK 1/2");
        assert_eq!(app.timer().countdown(), 3);
    }

    #[test]
    fn pomodoro_skip_moves_to_next_phase_and_saves() {
        let c = TempCache::new("pomo_skip");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        app.key('n', t0);
        assert_eq!(label(&app), "BREAK 1/2");
        assert_eq!(app.timer().countdown(), 1);
        assert_eq!(c.contents().as_deref(), Some("break 1 1"));
        app.key('n', t0);
        app.key('n', t0);
        assert_eq!(label(&app), "LONG BREAK");
        app.key('n', t0);
        assert_eq!(label(&app), "WORK 1/2");
    }

    #[test]
    fn pomodoro_skip_while_paused_starts_next_phase_running() {
        let c = TempCache::new("pomo_skip_paused");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        app.key('p', t0);
        app.key('n', t0);
        assert!(!app.timer().is_paused());
    }

    #[test]
    fn pomodoro_reset_restarts_phase_and_keeps_cache() {
        let c = TempCache::new("pomo_reset");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        app.key('n', t0);
        app.key('n', t0); // WORK 2/2
        app.tick(t0 + secs(2));
        assert_eq!(app.timer().countdown(), 1);
        app.key('r', t0 + secs(2));
        assert_eq!(label(&app), "WORK 2/2");
        assert_eq!(app.timer().countdown(), 3);
        assert_eq!(c.contents().as_deref(), Some("work 2 3"));
    }

    #[test]
    fn pomodoro_resumes_phase_round_and_time_from_cache() {
        let c = TempCache::new("pomo_resume");
        c.write("break 2 1\n");
        let t0 = Instant::now();
        let config = Config {
            rounds: 4,
            ..CONFIG
        };
        let mut app = App::pomodoro(config, &c.0, false, t0);
        assert_eq!(label(&app), "BREAK 2/4");
        assert_eq!(app.timer().countdown(), 1);
        assert_eq!(app.tick(t0 + secs(1)), Update::PhaseEnded);
        assert_eq!(label(&app), "WORK 3/4");
        // Reset goes back to the full length of the resumed phase.
        app.key('r', t0 + secs(1));
        assert_eq!(app.timer().countdown(), 3);
    }

    #[test]
    fn pomodoro_cached_zero_starts_that_phase_fresh() {
        let c = TempCache::new("pomo_zero");
        c.write("long-break 2 0");
        let app = App::pomodoro(CONFIG, &c.0, false, Instant::now());
        assert_eq!(label(&app), "LONG BREAK");
        assert_eq!(app.timer().countdown(), 2);
    }

    #[test]
    fn pomodoro_ignores_cache_that_does_not_fit() {
        for (name, text) in [
            ("plain", "42"),
            ("garbage", "hello"),
            ("round_too_high", "work 3 1"), // CONFIG has 2 rounds
        ] {
            let c = TempCache::new(&format!("pomo_misfit_{name}"));
            c.write(text);
            let app = App::pomodoro(CONFIG, &c.0, false, Instant::now());
            assert_eq!(label(&app), "WORK 1/2", "{text:?}");
            assert_eq!(app.timer().countdown(), 3, "{text:?}");
        }
    }

    #[test]
    fn pomodoro_clear_ignores_cache() {
        let c = TempCache::new("pomo_clear");
        c.write("break 2 1");
        let app = App::pomodoro(CONFIG, &c.0, true, Instant::now());
        assert_eq!(label(&app), "WORK 1/2");
    }

    #[test]
    fn pomodoro_quit_and_restart_continues_session() {
        let c = TempCache::new("pomo_restart");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        app.tick(t0 + secs(2));
        assert!(app.key('q', t0 + secs(2)));

        let app = App::pomodoro(CONFIG, &c.0, false, t0 + secs(60));
        assert_eq!(label(&app), "WORK 1/2");
        assert_eq!(app.timer().countdown(), 1);
    }

    #[test]
    fn late_tick_carries_overrun_across_phases() {
        let c = TempCache::new("pomo_overrun");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        // WORK 1/2 ends at 3s, BREAK 1/2 at 4s, WORK 2/2 runs 4s..7s.
        // A loop that wakes up at 5.5s must land in WORK 2/2 with 2s left
        // (the next decrement at 6s), and report one phase change.
        let late = t0 + Duration::from_millis(5500);
        assert_eq!(app.tick(late), Update::PhaseEnded);
        assert_eq!(label(&app), "WORK 2/2");
        assert_eq!(app.timer().countdown(), 2);
        assert_eq!(app.until_next_tick(late), Some(Duration::from_millis(500)));
        assert_eq!(c.contents().as_deref(), Some("work 2 2"));
        // Late by more than a whole cycle: 3+1+3+2 = 9s per cycle.
        let very_late = t0 + secs(9 + 9 + 1);
        assert_eq!(app.tick(very_late), Update::PhaseEnded);
        assert_eq!(label(&app), "WORK 1/2");
        assert_eq!(app.timer().countdown(), 2);
    }

    #[test]
    fn pause_on_phase_end_moves_on() {
        let c = TempCache::new("pomo_pause_end");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        // The pause lands exactly when WORK 1/2 runs out.
        app.key('p', t0 + secs(3));
        assert_eq!(app.tick(t0 + secs(3)), Update::PhaseEnded);
        assert_eq!(label(&app), "BREAK 1/2");
        assert!(!app.timer().is_finished());
        assert_eq!(app.tick(t0 + secs(4)), Update::PhaseEnded);
        assert_eq!(label(&app), "WORK 2/2");
    }
}
