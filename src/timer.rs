use std::time::{Duration, Instant};

const SECOND: Duration = Duration::from_secs(1);

/// Whether the countdown is advancing.
///
/// A finished timer (`countdown == 0`) is always `Running` so that pausing it
/// is a no-op; it simply has nothing left to tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// The next decrement is due at `next_tick`.
    Running { next_tick: Instant },
    /// Frozen; `until_next_tick` of the current second is still left to run.
    Paused { until_next_tick: Duration },
}

/// Countdown state. All time-dependent methods take the current `Instant`
/// explicitly so the logic is deterministic and testable; the UI loop passes
/// `Instant::now()`.
#[derive(Debug)]
pub struct Timer {
    countdown: u64,
    original_countdown: u64,
    state: State,
}

/// Converts hours, minutes and seconds to total seconds, or `None` on overflow.
pub fn time_to_countdown(hours: u64, minutes: u64, seconds: u64) -> Option<u64> {
    hours
        .checked_mul(3600)?
        .checked_add(minutes.checked_mul(60)?)?
        .checked_add(seconds)
}

fn countdown_to_time(seconds: u64) -> (u64, u64, u64) {
    (seconds / 3600, (seconds % 3600) / 60, seconds % 60)
}

impl Timer {
    /// A running timer that counts down `seconds` starting at `now`.
    pub fn new(seconds: u64, now: Instant) -> Self {
        Self {
            countdown: seconds,
            original_countdown: seconds,
            state: State::Running {
                next_tick: now + SECOND,
            },
        }
    }

    /// Resumes from a cached remaining time. `original` is what `reset` goes
    /// back to. A cached `0` means the previous run finished: start afresh.
    pub fn from_cache(cached: u64, original: u64, now: Instant) -> Self {
        if cached == 0 {
            return Self::new(original, now);
        }

        Self {
            countdown: cached,
            ..Self::new(original, now)
        }
    }

    /// Freezes the countdown. No-op once finished.
    pub fn pause(&mut self, now: Instant) {
        self.tick(now);

        if self.is_finished() {
            return;
        }

        if let State::Running { next_tick } = self.state {
            self.state = State::Paused {
                until_next_tick: next_tick.saturating_duration_since(now),
            };
        }
    }

    /// Continues a paused countdown exactly where it stopped.
    pub fn start(&mut self, now: Instant) {
        if let State::Paused { until_next_tick } = self.state {
            self.state = State::Running {
                next_tick: now + until_next_tick,
            };
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        if self.is_paused() {
            self.start(now);
        } else {
            self.pause(now);
        }
    }

    /// Restores the original duration and starts a fresh second. Keeps the
    /// current paused/running mode.
    pub fn reset(&mut self, now: Instant) {
        self.countdown = self.original_countdown;
        self.state = match self.state {
            State::Running { .. } => State::Running {
                next_tick: now + SECOND,
            },
            State::Paused { .. } => State::Paused {
                until_next_tick: SECOND,
            },
        };
    }

    /// Applies every whole second that elapsed up to `now`. Deadlines advance
    /// by exactly one second each, so there is no drift. Returns whether the
    /// countdown changed.
    pub fn tick(&mut self, now: Instant) -> bool {
        let State::Running { mut next_tick } = self.state else {
            return false;
        };

        let before = self.countdown;
        while self.countdown > 0 && now >= next_tick {
            self.countdown -= 1;
            next_tick += SECOND;
        }
        self.state = State::Running { next_tick };

        self.countdown != before
    }

    /// Time until the next decrement is due, or `None` if paused or finished.
    pub fn until_next_tick(&self, now: Instant) -> Option<Duration> {
        match self.state {
            State::Running { next_tick } if self.countdown > 0 => {
                Some(next_tick.saturating_duration_since(now))
            }
            _ => None,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.countdown == 0
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.state, State::Paused { .. })
    }

    pub fn countdown(&self) -> u64 {
        self.countdown
    }

    pub fn print(&self) -> String {
        let (hours, minutes, seconds) = countdown_to_time(self.countdown);
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn conversion_basic() {
        assert_eq!(time_to_countdown(1, 2, 3), Some(3723));
        assert_eq!(time_to_countdown(0, 0, 0), Some(0));
        assert_eq!(countdown_to_time(3723), (1, 2, 3));
    }

    #[test]
    fn conversion_overflow() {
        assert_eq!(time_to_countdown(u64::MAX, 0, 0), None);
        assert_eq!(time_to_countdown(0, u64::MAX, 0), None);
        assert_eq!(time_to_countdown(0, 0, u64::MAX), Some(u64::MAX));
        assert_eq!(time_to_countdown(0, 1, u64::MAX), None);
        assert_eq!(
            time_to_countdown(u64::MAX / 3600, 0, 0),
            Some(u64::MAX / 3600 * 3600)
        );
    }

    #[test]
    fn ticks_once_per_second() {
        let t0 = Instant::now();
        let mut t = Timer::new(3, t0);
        assert!(!t.tick(t0 + Duration::from_millis(999)));
        assert_eq!(t.countdown(), 3);
        assert!(t.tick(t0 + secs(1)));
        assert_eq!(t.countdown(), 2);
        assert!(!t.tick(t0 + secs(1)));
    }

    #[test]
    fn late_tick_catches_up_without_drift() {
        let t0 = Instant::now();
        let mut t = Timer::new(10, t0);
        // UI was late by 2.5s: three whole seconds elapsed.
        assert!(t.tick(t0 + Duration::from_millis(3500)));
        assert_eq!(t.countdown(), 7);
        // Next deadline is still t0+4s, not 3.5s+1s.
        assert_eq!(
            t.until_next_tick(t0 + Duration::from_millis(3500)),
            Some(Duration::from_millis(500))
        );
    }

    #[test]
    fn finishes_at_zero_and_stays() {
        let t0 = Instant::now();
        let mut t = Timer::new(2, t0);
        assert!(t.tick(t0 + secs(100)));
        assert_eq!(t.countdown(), 0);
        assert!(t.is_finished());
        assert!(!t.tick(t0 + secs(200)));
        assert_eq!(t.until_next_tick(t0 + secs(200)), None);
        assert_eq!(t.print(), "00:00:00");
    }

    #[test]
    fn pause_freezes_and_resume_has_no_burst_or_lost_second() {
        let t0 = Instant::now();
        let mut t = Timer::new(10, t0);
        t.pause(t0 + Duration::from_millis(400));
        assert!(t.is_paused());
        // A long pause must not tick.
        assert!(!t.tick(t0 + secs(60)));
        assert_eq!(t.countdown(), 10);
        // Resume: the remaining 600ms of the first second are still owed.
        t.start(t0 + secs(60));
        assert!(!t.is_paused());
        assert!(!t.tick(t0 + secs(60)));
        assert_eq!(t.countdown(), 10);
        assert_eq!(
            t.until_next_tick(t0 + secs(60)),
            Some(Duration::from_millis(600))
        );
        assert!(!t.tick(t0 + secs(60) + Duration::from_millis(599)));
        assert!(t.tick(t0 + secs(60) + Duration::from_millis(600)));
        assert_eq!(t.countdown(), 9);
    }

    #[test]
    fn pause_applies_pending_ticks_first() {
        let t0 = Instant::now();
        let mut t = Timer::new(10, t0);
        t.pause(t0 + Duration::from_millis(2300));
        assert_eq!(t.countdown(), 8);
        t.start(t0 + secs(50));
        assert_eq!(
            t.until_next_tick(t0 + secs(50)),
            Some(Duration::from_millis(700))
        );
    }

    #[test]
    fn pause_and_toggle_on_finished_are_noops() {
        let t0 = Instant::now();
        let mut t = Timer::new(1, t0);
        t.tick(t0 + secs(1));
        assert!(t.is_finished());
        t.pause(t0 + secs(2));
        assert!(!t.is_paused());
        t.toggle(t0 + secs(2));
        assert!(!t.is_paused());
    }

    #[test]
    fn pause_exactly_on_final_tick_finishes_instead_of_pausing() {
        let t0 = Instant::now();
        let mut t = Timer::new(1, t0);
        t.pause(t0 + secs(1));
        assert!(t.is_finished());
        assert!(!t.is_paused());
    }

    #[test]
    fn toggle_flips() {
        let t0 = Instant::now();
        let mut t = Timer::new(5, t0);
        t.toggle(t0);
        assert!(t.is_paused());
        t.toggle(t0);
        assert!(!t.is_paused());
    }

    #[test]
    fn reset_after_finish_counts_again() {
        let t0 = Instant::now();
        let mut t = Timer::new(2, t0);
        t.tick(t0 + secs(2));
        assert!(t.is_finished());

        let t1 = t0 + secs(10);
        t.reset(t1);
        assert_eq!(t.countdown(), 2);
        assert!(!t.is_finished());
        assert!(!t.tick(t1 + Duration::from_millis(999)));
        assert!(t.tick(t1 + secs(1)));
        assert_eq!(t.countdown(), 1);
        assert!(t.tick(t1 + secs(2)));
        assert!(t.is_finished());
    }

    #[test]
    fn reset_keeps_paused_mode_with_full_second() {
        let t0 = Instant::now();
        let mut t = Timer::new(5, t0);
        t.pause(t0 + secs(2));
        t.reset(t0 + secs(3));
        assert!(t.is_paused());
        assert_eq!(t.countdown(), 5);
        t.start(t0 + secs(4));
        assert_eq!(t.until_next_tick(t0 + secs(4)), Some(secs(1)));
    }

    #[test]
    fn from_cache_uses_cached_and_resets_to_args() {
        let t0 = Instant::now();
        let mut t = Timer::from_cache(7, 30, t0);
        assert_eq!(t.countdown(), 7);
        assert!(!t.is_paused());
        t.reset(t0);
        assert_eq!(t.countdown(), 30);
    }

    #[test]
    fn from_cache_zero_starts_fresh() {
        let t = Timer::from_cache(0, 30, Instant::now());
        assert_eq!(t.countdown(), 30);
    }

    #[test]
    fn print_formats_hours_minutes_seconds() {
        let t = Timer::new(3723, Instant::now());
        assert_eq!(t.print(), "01:02:03");
        let t = Timer::new(100 * 3600, Instant::now());
        assert_eq!(t.print(), "100:00:00");
    }
}
