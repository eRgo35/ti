# Alert flash: a visible fallback for the bell

## Why

A finished pomodoro phase rings the terminal bell, and that's the only
alert. Many people mute the bell or never turn it on (the owner included),
so a phase change can go unnoticed. A plain countdown (`ti 25m`) gives no
alert at all when it finishes. A program can't find out whether the
terminal's bell is muted, so the fallback has to happen every time.

## Goals

- Every alert is visible as well as audible: the bell rings and the whole
  screen flashes.
- A plain countdown alerts when it reaches 0, the same way a pomodoro phase
  change does.

## Non-goals

- Desktop notifications (OSC 9/777 or a notification crate).
- An option to turn the flash or the bell off, or to choose between them.
- Detecting whether the bell is muted.

## Behaviour

- **When:** a pomodoro phase ends (also when one late tick runs through
  several phases: one alert), or a countdown reaches 0 while it runs. A
  timer that's already finished when `ti` starts (`ti 0`, or a cache holding
  `0`, which starts fresh anyway) doesn't alert. `N` (skip) doesn't alert:
  the user pressed it. After `R` on a finished countdown, it alerts again
  when it finishes again.
- **What:** the terminal bell (BEL), as today, plus a **flash**: for 1
  second the whole screen is drawn in reverse video (every cell, the clock
  and the status line). Then the screen goes back to normal: a green break,
  or `TIMER FINISHED`.
- **During the flash** the timer keeps running, the clock keeps updating in
  reverse video, and keys work as usual. The flash ends on time; a key press
  doesn't end it.
- **CLI, keys, status line text, cache:** unchanged. No new option.

## Decisions (owner, 2026-11-18)

| # | Decision |
|---|----------|
| F1 | The fallback is a full-screen flash (reverse video, about 1 s), not a notification or a status banner. |
| F2 | It's always on, with no option. |
| F3 | The plain countdown also rings and flashes when it finishes. |
| F4 | This is a behaviour change, so the release is **1.7.0**. |
