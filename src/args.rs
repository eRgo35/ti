use std::ffi::OsString;

use clap::{
    error::ErrorKind, parser::ValueSource, CommandFactory, FromArgMatches, Parser, Subcommand,
};

use crate::{pomodoro::Config, timer::time_to_countdown};

const NAME: Option<&str> = option_env!("CARGO_PKG_NAME");
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const DESCRIPTION: Option<&str> = option_env!("CARGO_PKG_DESCRIPTION");

#[derive(Parser, Debug)]
#[command(
    name = NAME.unwrap_or("ti"),
    author = "Michał Czyż",
    version = VERSION.unwrap_or("unknown"),
    about = DESCRIPTION.unwrap_or("A simple terminal timer"),
    long_about = None
)]
pub struct Args {
    #[arg(short = 'H', long, default_value_t = 0, help = "Hours")]
    pub hours: u64,

    #[arg(
        short_alias = 'm',
        short = 'M',
        long,
        default_value_t = 0,
        help = "Minutes"
    )]
    pub minutes: u64,

    #[arg(
        short_alias = 's',
        short = 'S',
        long,
        default_value_t = 0,
        help = "Seconds"
    )]
    pub seconds: u64,

    #[arg(long, global = true, default_value_t = String::from(""), hide_default_value = true, help = "Path to custom font file")]
    pub font: String,

    #[arg(long, global = true, default_value_t = String::from(""), hide_default_value = true, help = "Path to cache file")]
    pub cache: String,

    #[arg(long, global = true, default_value_t = false, help = "Clear cache")]
    pub clear: bool,

    #[arg(
        value_name = "TIME",
        value_parser = parse_time,
        help = "Time as H, H:M or H:M:S (overrides -H/-M/-S)"
    )]
    pub time: Option<(u64, u64, u64)>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Pomodoro timer: work sessions and breaks in a loop
    Pomodoro(PomodoroArgs),
}

impl Args {
    /// Parses `args` (including the program name). Countdown options cannot
    /// be combined with a subcommand, which would otherwise ignore them.
    pub fn try_parse_args<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut command = Self::command();
        let matches = command.try_get_matches_from_mut(args)?;
        let parsed = Self::from_arg_matches(&matches)?;

        if parsed.command.is_some() {
            for id in ["hours", "minutes", "seconds", "time"] {
                if matches.value_source(id) == Some(ValueSource::CommandLine) {
                    let arg = command
                        .get_arguments()
                        .find(|arg| arg.get_id() == id)
                        .map_or_else(|| id.to_owned(), ToString::to_string);
                    return Err(command.error(
                        ErrorKind::ArgumentConflict,
                        format!("'{arg}' cannot be used with a subcommand"),
                    ));
                }
            }
        }

        Ok(parsed)
    }
}

/// Largest number of minutes that still fits in seconds.
const MAX_MINUTES: u64 = u64::MAX / 60;

#[derive(clap::Args, Debug)]
pub struct PomodoroArgs {
    #[arg(long, value_name = "MINUTES", default_value_t = 25, value_parser = clap::value_parser!(u64).range(1..=MAX_MINUTES), help = "Work session length")]
    pub work: u64,

    #[arg(long = "break", value_name = "MINUTES", default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..=MAX_MINUTES), help = "Short break length")]
    pub short_break: u64,

    #[arg(long, value_name = "MINUTES", default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..=MAX_MINUTES), help = "Long break length")]
    pub long_break: u64,

    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u64).range(1..), help = "Work sessions before a long break")]
    pub rounds: u64,
}

impl PomodoroArgs {
    pub fn config(&self) -> Config {
        Config {
            work: self.work * 60,
            short_break: self.short_break * 60,
            long_break: self.long_break * 60,
            rounds: self.rounds,
        }
    }
}

/// Parses `H`, `H:M` or `H:M:S` into `(hours, minutes, seconds)`. Components
/// fill hours first; missing ones are zero.
pub fn parse_time(input: &str) -> Result<(u64, u64, u64), String> {
    let mut parts = [0u64; 3];
    for (i, part) in input.split(':').enumerate() {
        if i == parts.len() {
            return Err(format!(
                "'{input}' has too many components, expected H, H:M or H:M:S"
            ));
        }
        if part.is_empty() {
            return Err(format!("'{input}' has an empty component"));
        }
        if !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("'{part}' is not a non-negative integer"));
        }
        parts[i] = part.parse().map_err(|_| format!("'{part}' is too large"))?;
    }

    let [hours, minutes, seconds] = parts;
    if time_to_countdown(hours, minutes, seconds).is_none() {
        return Err(format!("'{input}' is too large"));
    }

    Ok((hours, minutes, seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_two_three_components() {
        assert_eq!(parse_time("5"), Ok((5, 0, 0)));
        assert_eq!(parse_time("1:30"), Ok((1, 30, 0)));
        assert_eq!(parse_time("1:2:3"), Ok((1, 2, 3)));
        assert_eq!(parse_time("0:0:0"), Ok((0, 0, 0)));
        assert_eq!(parse_time("00:05:00"), Ok((0, 5, 0)));
    }

    #[test]
    fn does_not_normalise_components() {
        assert_eq!(parse_time("0:90:75"), Ok((0, 90, 75)));
    }

    #[test]
    fn rejects_non_numeric() {
        for bad in [
            "abc", "1:x", "x:1", "1.5", "-1", "+1", " 1", "1 ", "0x10", "١",
        ] {
            assert!(parse_time(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn rejects_empty_components() {
        for bad in ["", ":", "1:", ":1", "1::2", "::"] {
            assert!(parse_time(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn rejects_too_many_components() {
        assert!(parse_time("1:2:3:4").is_err());
        assert!(parse_time("1:2:3:").is_err());
    }

    fn parse(args: &[&str]) -> Result<Args, clap::Error> {
        Args::try_parse_args(std::iter::once("ti").chain(args.iter().copied()))
    }

    fn pomodoro_config(args: &[&str]) -> Config {
        match parse(args).unwrap().command {
            Some(Command::Pomodoro(p)) => p.config(),
            None => panic!("{args:?} is not a pomodoro command"),
        }
    }

    #[test]
    fn countdown_flags_and_time() {
        let args = parse(&["-H", "1", "-m", "2", "-S", "3"]).unwrap();
        assert_eq!((args.hours, args.minutes, args.seconds), (1, 2, 3));
        assert!(args.command.is_none());
        assert_eq!(parse(&["1:30"]).unwrap().time, Some((1, 30, 0)));
        assert_eq!(parse(&["-s", "9"]).unwrap().seconds, 9);
    }

    #[test]
    fn pomodoro_defaults_are_classic() {
        assert_eq!(
            pomodoro_config(&["pomodoro"]),
            Config {
                work: 25 * 60,
                short_break: 5 * 60,
                long_break: 15 * 60,
                rounds: 4,
            }
        );
    }

    #[test]
    fn pomodoro_overrides() {
        assert_eq!(
            pomodoro_config(&[
                "pomodoro",
                "--work",
                "50",
                "--break",
                "10",
                "--long-break",
                "30",
                "--rounds",
                "2"
            ]),
            Config {
                work: 50 * 60,
                short_break: 10 * 60,
                long_break: 30 * 60,
                rounds: 2,
            }
        );
    }

    #[test]
    fn pomodoro_accepts_shared_options_before_or_after() {
        for args in [
            &["--font", "f.flf", "--cache", "c", "--clear", "pomodoro"][..],
            &["pomodoro", "--font", "f.flf", "--cache", "c", "--clear"][..],
        ] {
            let args = parse(args).unwrap();
            assert_eq!(
                (args.font.as_str(), args.cache.as_str(), args.clear),
                ("f.flf", "c", true)
            );
            assert!(args.command.is_some());
        }
    }

    #[test]
    fn pomodoro_rejects_zero_and_overflowing_lengths() {
        for flag in ["--work", "--break", "--long-break", "--rounds"] {
            let err = parse(&["pomodoro", flag, "0"]).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::ValueValidation, "{flag}");
        }
        let too_big = (MAX_MINUTES + 1).to_string();
        assert!(parse(&["pomodoro", "--work", &too_big]).is_err());
        let max = MAX_MINUTES.to_string();
        assert_eq!(
            pomodoro_config(&["pomodoro", "--work", &max]).work,
            MAX_MINUTES * 60
        );
    }

    #[test]
    fn pomodoro_rejects_countdown_options() {
        for args in [
            &["-H", "1", "pomodoro"][..],
            &["-M", "0", "pomodoro"][..],
            &["--seconds", "5", "pomodoro"][..],
            &["1:30", "pomodoro"][..],
        ] {
            let err = parse(args).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::ArgumentConflict, "{args:?}");
        }
        assert!(parse(&["pomodoro", "5"]).is_err());
        assert!(parse(&["pomodoro", "-M", "5"]).is_err());
    }

    #[test]
    fn rejects_overflow() {
        assert!(parse_time("18446744073709551616").is_err()); // u64::MAX + 1
        assert!(parse_time("18446744073709551615").is_err()); // hours * 3600 overflows
        assert!(parse_time("0:18446744073709551615").is_err()); // minutes * 60 overflows
        assert!(parse_time("0:1:18446744073709551615").is_err()); // sum overflows
        assert_eq!(parse_time("0:0:18446744073709551615"), Ok((0, 0, u64::MAX)));
    }
}
