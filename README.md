# ti

[![crates.io](https://img.shields.io/crates/v/ti.svg)](https://crates.io/crates/ti)

A simple terminal countdown timer that draws the remaining time as large FIGlet text.

## Installation

Install from crates.io:

```sh
cargo install ti
```

Prebuilt binaries and shell, PowerShell and MSI installers are available on the [GitHub Releases page](https://github.com/eRgo35/ti/releases).

Build from source:

```sh
git clone https://github.com/eRgo35/ti && cd ti && cargo install --path .
```

## Usage

```
$ ti --help
A simple terminal timer

Usage: ti [OPTIONS] [TIME] [COMMAND]

Commands:
  pomodoro  Pomodoro timer: work sessions and breaks in a loop
  help      Print this message or the help of the given subcommand(s)

Arguments:
  [TIME]  Time as H, H:M or H:M:S (overrides -H/-M/-S)

Options:
  -H, --hours <HOURS>      Hours [default: 0]
  -M, --minutes <MINUTES>  Minutes [default: 0]
  -S, --seconds <SECONDS>  Seconds [default: 0]
      --font <FONT>        Path to custom font file
      --cache <CACHE>      Path to cache file
      --clear              Clear cache
  -h, --help               Print help
  -V, --version            Print version
```

### Setting the duration

Use the flags:

```sh
ti -M 25                # 25 minutes
ti -H 1 -M 30 -S 15     # 1 hour, 30 minutes, 15 seconds
ti 0:0:30               # 30 seconds (positional TIME, see below)
```

Or the positional `TIME` argument, in the form `H`, `H:M` or `H:M:S`. Its parts are read left to right as hours, minutes, seconds, so they fill from hours first:

```sh
ti 1:30       # 1 hour 30 minutes
ti 0:25:00    # 25 minutes
ti 90         # 90 hours
```

When `TIME` is given, it replaces the `-H`/`-M`/`-S` flags entirely. Parts that `TIME` omits are 0, so `ti 1:30 -M 5` counts down from 1 hour 30 minutes.

An invalid `TIME` (non-numeric, more than three components, or too large) is rejected with a usage error and exit code 2.

### Pomodoro

`ti pomodoro` runs work sessions and breaks in a loop: 25 minutes of work, a 5 minute break, and a 15 minute long break after every 4th work session. When a phase ends, the terminal bell rings and the next phase starts on its own.

```sh
ti pomodoro                                  # 25 / 5 / 15, long break every 4 sessions
ti pomodoro --work 50 --break 10             # longer sessions
ti pomodoro --long-break 30 --rounds 3       # long break after every 3rd session
```

| Option                   | Default | Meaning                           |
| ------------------------ | ------- | --------------------------------- |
| `--work <MINUTES>`       | 25      | Work session length               |
| `--break <MINUTES>`      | 5       | Short break length                |
| `--long-break <MINUTES>` | 15      | Long break length                 |
| `--rounds <ROUNDS>`      | 4       | Work sessions before a long break |

All values must be at least 1. `--font`, `--cache` and `--clear` work as for the normal timer and can go before or after `pomodoro`. The countdown options (`TIME`, `-H`, `-M`, `-S`) cannot be combined with `pomodoro`.

The status line shows the phase and round, for example `WORK 2/4` or `LONG BREAK`. Breaks are drawn in green.

### Key bindings

| Key      | Action                                                               |
| -------- | -------------------------------------------------------------------- |
| `SPACE`  | Pause / resume                                                       |
| `P`      | Pause                                                                |
| `S`      | Start (resume)                                                       |
| `R`      | Reset to the original duration (pomodoro: restart the current phase) |
| `N`      | Pomodoro only: skip to the next phase                                |
| `Q`      | Quit                                                                 |
| `Ctrl+C` | Quit immediately with exit code `1`                                  |

Keys are case-insensitive.

### Cache

While the timer runs, the remaining time in seconds is saved every second to `ti_countdown.tmp` in the OS temp directory (for example `/tmp`), or to the file given with `--cache <path>`.

- Starting `ti` again with the same cache file resumes the saved countdown. The duration from the command line is still the one `R` resets to.
- `--clear` ignores the saved countdown and starts from the given time. The cache file is overwritten as the timer runs.
- `R` resets the countdown and deletes the cache file. A running timer writes the file again on its next tick.
- Quitting with `Q` or `Ctrl+C` keeps the cache file.
- A timer that has finished starts fresh the next time.

A pomodoro session is cached separately, in `ti_pomodoro.tmp` (or the `--cache` file), as its phase, round and remaining seconds. Starting `ti pomodoro` again continues the same phase and round; `--clear` starts a new session at the first work session. `R` restarts the current phase and keeps the session cached. A saved session whose round is beyond `--rounds` is ignored.

### Font

`--font <path>` loads a FIGlet font file (`.flf`) to draw the digits. The default is the bundled ANSI Mono font. An unreadable or invalid font file is an error with exit code 1; `ti` does not fall back to the default font. A font must have glyphs for the digits and `:`.

## License

MIT — see [LICENSE.md](LICENSE.md)
