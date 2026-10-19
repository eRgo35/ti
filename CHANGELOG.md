# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.5.0] - 2026-10-19

### Fixed
- Invalid or oversized time input (`ti abc`, `ti 1:2:3:4`, `ti 1::2`, values that overflow) now prints a usage error and exits with status 2 instead of panicking (#5).
- The terminal is always restored on exit: the cursor is shown again and raw mode is turned off, including on Ctrl+C, errors and panics. Running without a terminal now prints an error instead of panicking (#6).
- Windows: each key press is handled once instead of twice (press and release events) (#7).
- The countdown is driven by the UI loop on a monotonic clock: reset after the timer has finished restarts it, quitting is immediate instead of lagging up to a second, and redraws no longer flood the terminal (#8).
- An unwritable `--cache` path no longer crashes the timer; the timer keeps running without saving (#9).
- `--font` with a missing or invalid file now reports an error and exits with status 1 instead of being silently ignored or panicking; the timer is centred using the width of the rendered font and the status line stays on screen in small terminals (#10).

### Changed
- Release workflow regenerated with dist 0.33.0 (replacing cargo-dist 0.20.0), which no longer uses the retired `ubuntu-20.04` runner that made releases hang forever. The dist configuration moved from `Cargo.toml` to `dist-workspace.toml` (#4).

### Added
- CI workflow running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` and a build on Linux, macOS and Windows for every pull request and push to `senpai` (#4).

### Docs
- README documents the positional `TIME` argument, `--clear`, key bindings, installation methods and font/cache behaviour (#11).

## [1.4.0] - 2024-08-17

### Added
- `--clear` flag to ignore and discard the cached countdown.
- Paused, finished and running states are shown in the status line, with the timer dimmed while paused.

### Changed
- The countdown is cached while running and resumed on the next start; reset restores the originally requested time and clears the cache.

## [1.3.2] - 2024-08-13

### Changed
- Version bump to exercise the release workflow.

## [1.3.1] - 2024-08-13

### Changed
- Dropped the release-plz and CI workflows in favour of the cargo-dist release workflow.

## [1.3.0](https://github.com/eRgo35/ti/releases/tag/v1.3.0) - 2024-08-13

### Added
- positional argument for time
- timer is multithreaded

### Fixed
- README.me
- moved font to resources
- ux improvements
- refactor timer initialization
- refactor
- optmize

### Other
- release-plz setup
- cargo-dist action setup
- release 1.3.0
- release 1.2.3
- github actions: init
- 1.2.2
- 1.2.1
- moved drawing to separate mod
- 1.2.0
- 1.1.2
- version bump
- 1.1.1 more functional
- 1.1.0 added documentation, fixed pathing and fonts
- initial release 1.0.0
