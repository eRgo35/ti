use clap::Parser;

use crate::timer::time_to_countdown;

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

    #[arg(long, default_value_t = String::from(""), help = "Path to custom font file")]
    pub font: String,

    #[arg(long, default_value_t = String::from(""), help = "Path to cache file")]
    pub cache: String,

    #[arg(long, default_value_t = false, help = "Clear cache")]
    pub clear: bool,

    #[arg(
        value_name = "TIME",
        value_parser = parse_time,
        help = "Time as H, H:M or H:M:S (overrides -H/-M/-S)"
    )]
    pub time: Option<(u64, u64, u64)>,
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

    #[test]
    fn rejects_overflow() {
        assert!(parse_time("18446744073709551616").is_err()); // u64::MAX + 1
        assert!(parse_time("18446744073709551615").is_err()); // hours * 3600 overflows
        assert!(parse_time("0:18446744073709551615").is_err()); // minutes * 60 overflows
        assert!(parse_time("0:1:18446744073709551615").is_err()); // sum overflows
        assert_eq!(parse_time("0:0:18446744073709551615"), Ok((0, 0, u64::MAX)));
    }
}
