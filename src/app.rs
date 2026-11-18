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
    /// A pomodoro phase or the countdown ran out: ring the bell. The screen
    /// flashes for `FLASH` (see `App::is_flashing`).
    Alert,
}

/// How long the screen flashes after an alert, as a fallback for a muted
/// terminal bell.
pub const FLASH: Duration = Duration::from_secs(1);

/// Timer behaviour without the terminal: keys, ticks, pomodoro phases and
/// the cache. Every method takes `now` so it can be tested deterministically.
pub struct App<'a> {
    timer: Timer,
    pomodoro: Option<Pomodoro>,
    cache: &'a Cache,
    /// Whether the last tick saw the countdown at 0, so finishing alerts
    /// once, and a timer that starts finished never alerts.
    was_finished: bool,
    /// The end of the current flash, if any.
    flash_until: Option<Instant>,
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
            was_finished: timer.is_finished(),
            timer,
            pomodoro: None,
            cache,
            flash_until: None,
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
            was_finished: timer.is_finished(),
            timer,
            pomodoro: Some(pomodoro),
            cache,
            flash_until: None,
        }
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    pub fn pomodoro_state(&self) -> Option<&Pomodoro> {
        self.pomodoro.as_ref()
    }

    /// Whether the screen should be drawn flashing at `now`.
    pub fn is_flashing(&self, now: Instant) -> bool {
        self.flash_until.is_some_and(|until| now < until)
    }

    /// Time until the next tick is due: the next decrement or the end of a
    /// flash, whichever comes first. `None` if nothing is due.
    pub fn until_next_tick(&self, now: Instant) -> Option<Duration> {
        let flash = self
            .flash_until
            .map(|until| until.saturating_duration_since(now));
        match (self.timer.until_next_tick(now), flash) {
            (Some(timer), Some(flash)) => Some(timer.min(flash)),
            (timer, flash) => timer.or(flash),
        }
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
                update = Update::Alert;
            }
        } else if self.timer.is_finished() && !self.was_finished {
            update = Update::Alert;
        }
        self.was_finished = self.timer.is_finished();

        // Only a change of the timer is saved, not the end of a flash.
        if update != Update::Unchanged {
            self.save();
        }

        if update == Update::Alert {
            self.flash_until = Some(now + FLASH);
        } else if self.flash_until.is_some_and(|until| now >= until) {
            self.flash_until = None;
            update = Update::Changed;
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
                // A reset countdown alerts again when it finishes.
                self.was_finished = self.timer.is_finished();
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
        assert_eq!(app.tick(t0 + secs(2)), Update::Alert);
        assert_eq!(c.contents().as_deref(), Some("0"));
        assert!(app.timer().is_finished());
        assert_eq!(app.tick(t0 + secs(9)), Update::Changed, "the flash ends");
        assert_eq!(app.tick(t0 + secs(10)), Update::Unchanged);
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
            (3, Update::Alert, "BREAK 1/2", 1, "break 1 1"),
            (4, Update::Alert, "WORK 2/2", 3, "work 2 3"),
            (5, Update::Changed, "WORK 2/2", 2, "work 2 2"),
            (6, Update::Changed, "WORK 2/2", 1, "work 2 1"),
            (7, Update::Alert, "LONG BREAK", 2, "long-break 2 2"),
            (8, Update::Changed, "LONG BREAK", 1, "long-break 2 1"),
            (9, Update::Alert, "WORK 1/2", 3, "work 1 3"),
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
        assert_eq!(app.tick(t0 + secs(1)), Update::Alert);
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
        assert_eq!(app.tick(late), Update::Alert);
        assert_eq!(label(&app), "WORK 2/2");
        assert_eq!(app.timer().countdown(), 2);
        assert_eq!(app.until_next_tick(late), Some(Duration::from_millis(500)));
        assert_eq!(c.contents().as_deref(), Some("work 2 2"));
        // Late by more than a whole cycle: 3+1+3+2 = 9s per cycle.
        let very_late = t0 + secs(9 + 9 + 1);
        assert_eq!(app.tick(very_late), Update::Alert);
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
        assert_eq!(app.tick(t0 + secs(3)), Update::Alert);
        assert_eq!(label(&app), "BREAK 1/2");
        assert!(!app.timer().is_finished());
        assert_eq!(app.tick(t0 + secs(4)), Update::Alert);
        assert_eq!(label(&app), "WORK 2/2");
    }

    // --- alerts ---

    #[test]
    fn countdown_finish_alerts_and_flashes_for_a_second() {
        let c = TempCache::new("alert_cd");
        let t0 = Instant::now();
        let mut app = App::countdown(2, &c.0, false, t0);
        assert_eq!(app.tick(t0 + secs(1)), Update::Changed);
        assert!(!app.is_flashing(t0 + secs(1)));
        assert_eq!(app.tick(t0 + secs(2)), Update::Alert);
        assert!(app.is_flashing(t0 + secs(2)));
        assert!(app.is_flashing(t0 + Duration::from_millis(2999)));
        assert!(!app.is_flashing(t0 + secs(3)));
    }

    #[test]
    fn finished_at_start_does_not_alert() {
        let c = TempCache::new("alert_zero");
        let t0 = Instant::now();
        let mut app = App::countdown(0, &c.0, false, t0);
        assert!(app.timer().is_finished());
        assert_eq!(app.tick(t0 + secs(1)), Update::Unchanged);
        assert!(!app.is_flashing(t0 + secs(1)));
    }

    #[test]
    fn skip_does_not_alert() {
        let c = TempCache::new("alert_skip");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        app.key('n', t0);
        assert_eq!(app.tick(t0), Update::Unchanged);
        assert!(!app.is_flashing(t0));
    }

    #[test]
    fn reset_then_finish_alerts_again() {
        let c = TempCache::new("alert_again");
        let t0 = Instant::now();
        let mut app = App::countdown(1, &c.0, false, t0);
        assert_eq!(app.tick(t0 + secs(1)), Update::Alert);
        assert_eq!(app.tick(t0 + secs(2)), Update::Changed, "the flash ends");
        app.key('r', t0 + secs(5));
        assert_eq!(app.tick(t0 + secs(6)), Update::Alert);
    }

    #[test]
    fn pause_on_countdown_end_alerts() {
        let c = TempCache::new("alert_pause_end");
        let t0 = Instant::now();
        let mut app = App::countdown(2, &c.0, false, t0);
        // The pause lands exactly when the countdown runs out.
        app.key('p', t0 + secs(2));
        assert_eq!(app.tick(t0 + secs(2)), Update::Alert);
    }

    #[test]
    fn flash_ends_after_a_second_while_paused() {
        let c = TempCache::new("alert_flash_paused");
        let t0 = Instant::now();
        let mut app = App::pomodoro(CONFIG, &c.0, false, t0);
        assert_eq!(app.tick(t0 + secs(3)), Update::Alert);
        let paused = t0 + Duration::from_millis(3200);
        app.key('p', paused);
        // The loop must wake up for the end of the flash.
        assert_eq!(
            app.until_next_tick(paused),
            Some(Duration::from_millis(800))
        );
        assert_eq!(app.tick(t0 + secs(4)), Update::Changed);
        assert!(!app.is_flashing(t0 + secs(4)));
        assert_eq!(app.until_next_tick(t0 + secs(4)), None);
        assert_eq!(app.tick(t0 + secs(5)), Update::Unchanged);
    }

    #[test]
    fn flash_end_does_not_rewrite_a_deleted_cache() {
        let c = TempCache::new("alert_flash_cache");
        let t0 = Instant::now();
        let mut app = App::countdown(1, &c.0, false, t0);
        assert_eq!(app.tick(t0 + secs(1)), Update::Alert);
        // Reset, then pause: the cache is deleted and nothing ticks.
        app.key('r', t0 + Duration::from_millis(1100));
        app.key('p', t0 + Duration::from_millis(1200));
        assert_eq!(c.contents(), None);
        app.tick(t0 + secs(2));
        assert_eq!(c.contents(), None);
    }
}
