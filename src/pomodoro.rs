/// Phase lengths in seconds and how many work sessions come before a long
/// break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub work: u64,
    pub short_break: u64,
    pub long_break: u64,
    pub rounds: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Work,
    Break,
    LongBreak,
}

/// Where a pomodoro session is in its cycle. The countdown itself lives in
/// `Timer`; this only knows which phase comes next and how long it lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pomodoro {
    config: Config,
    phase: Phase,
    /// The current work session, from 1 to `config.rounds`.
    round: u64,
}

impl Pomodoro {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            phase: Phase::Work,
            round: 1,
        }
    }

    /// Length of the current phase in seconds.
    pub fn duration(&self) -> u64 {
        match self.phase {
            Phase::Work => self.config.work,
            Phase::Break => self.config.short_break,
            Phase::LongBreak => self.config.long_break,
        }
    }

    /// Moves to the next phase: work, break, work, break, ... and a long break
    /// after the last round, then starts over at round 1.
    pub fn advance(&mut self) {
        (self.phase, self.round) = match self.phase {
            Phase::Work if self.round >= self.config.rounds => (Phase::LongBreak, self.round),
            Phase::Work => (Phase::Break, self.round),
            Phase::Break => (Phase::Work, self.round + 1),
            Phase::LongBreak => (Phase::Work, 1),
        };
    }

    pub fn is_break(&self) -> bool {
        self.phase != Phase::Work
    }

    pub fn label(&self) -> String {
        let rounds = self.config.rounds;
        match self.phase {
            Phase::Work => format!("WORK {}/{rounds}", self.round),
            Phase::Break => format!("BREAK {}/{rounds}", self.round),
            Phase::LongBreak => "LONG BREAK".to_owned(),
        }
    }

    /// Cache line: `<phase> <round> <remaining seconds>`.
    pub fn encode(&self, remaining: u64) -> String {
        let phase = match self.phase {
            Phase::Work => "work",
            Phase::Break => "break",
            Phase::LongBreak => "long-break",
        };
        format!("{phase} {} {remaining}", self.round)
    }

    /// Reads a cache line written by `encode`. Returns `None` for anything
    /// that does not fit `config`, such as a round beyond `config.rounds`.
    pub fn decode(config: Config, text: &str) -> Option<(Self, u64)> {
        let mut parts = text.split_whitespace();
        let phase = match parts.next()? {
            "work" => Phase::Work,
            "break" => Phase::Break,
            "long-break" => Phase::LongBreak,
            _ => return None,
        };
        let round: u64 = parts.next()?.parse().ok()?;
        let remaining: u64 = parts.next()?.parse().ok()?;
        if parts.next().is_some() || !(1..=config.rounds).contains(&round) {
            return None;
        }

        Some((
            Self {
                config,
                phase,
                round,
            },
            remaining,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: Config = Config {
        work: 25 * 60,
        short_break: 5 * 60,
        long_break: 15 * 60,
        rounds: 4,
    };

    #[test]
    fn full_cycle() {
        let mut p = Pomodoro::new(CONFIG);
        let mut seen = vec![];
        for _ in 0..10 {
            seen.push((p.label(), p.duration(), p.is_break()));
            p.advance();
        }
        let expected = [
            ("WORK 1/4", 1500, false),
            ("BREAK 1/4", 300, true),
            ("WORK 2/4", 1500, false),
            ("BREAK 2/4", 300, true),
            ("WORK 3/4", 1500, false),
            ("BREAK 3/4", 300, true),
            ("WORK 4/4", 1500, false),
            ("LONG BREAK", 900, true),
            ("WORK 1/4", 1500, false),
            ("BREAK 1/4", 300, true),
        ];
        let expected: Vec<_> = expected
            .iter()
            .map(|&(l, d, b)| (l.to_owned(), d, b))
            .collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn single_round_goes_straight_to_long_break() {
        let mut p = Pomodoro::new(Config {
            rounds: 1,
            ..CONFIG
        });
        p.advance();
        assert_eq!(p.label(), "LONG BREAK");
        p.advance();
        assert_eq!(p.label(), "WORK 1/1");
    }

    #[test]
    fn encode_decode_roundtrip() {
        let mut p = Pomodoro::new(CONFIG);
        for _ in 0..8 {
            let line = p.encode(42);
            assert_eq!(Pomodoro::decode(CONFIG, &line), Some((p, 42)), "{line}");
            p.advance();
        }
        assert_eq!(Pomodoro::new(CONFIG).encode(7), "work 1 7");
    }

    #[test]
    fn decode_rejects_bad_lines() {
        for bad in [
            "",
            "work",
            "work 1",
            "rest 1 10",
            "work x 10",
            "work 1 -1",
            "work 1 10 extra",
            "work 0 10",
            "work 5 10", // beyond rounds
            "123",       // a plain countdown cache
        ] {
            assert_eq!(Pomodoro::decode(CONFIG, bad), None, "{bad:?}");
        }
        assert!(Pomodoro::decode(CONFIG, " break 3 10\n").is_some());
    }
}
