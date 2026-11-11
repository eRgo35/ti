# ti

A small Rust terminal timer (countdown and pomodoro) that draws the time as
FIGlet text. Keep it simple: no new dependencies without asking.

## Workflow: spec → plan → approval → implement

Never design by editing code live. Every feature, refactor, dependency bump
or multi-file change goes through these steps; one-line fixes and pure
lookups are exempt.

1. **Spec** (only for a new feature or behaviour change):
   `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`. Covers why, goals,
   non-goals, the user-visible behaviour (CLI, keys, status line, cache) and
   the owner's decisions. Ask about every open design question before
   writing it; don't pick an interpretation silently.
2. **Plan**: `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`, with:
   - a header with **Goal**, **Architecture** (which modules change and why)
     and **Spec** (a link to it, or the issue)
   - a **Decisions** table (D1, D2, ...) copied from the spec or the session
   - **Global Constraints** (no new deps, no `unwrap`/`expect` in production
     code, Conventional Commits, one commit per task)
   - **Review Focus**: the risky edge cases, each pinned by a named test
   - **Tasks**: for each one, the files it touches and checkbox (`- [ ]`)
     steps, test first (write the failing test, see it fail, implement,
     see it pass), then commit.
3. **Approval**: show the plan and wait for the owner's go-ahead before
   touching any source file. If the plan turns out wrong while implementing
   (an API break, a new decision, more scope), stop, update the plan and ask
   again.
4. **Implement** task by task, ticking the plan's checkboxes as you go, so
   progress can be followed in the repo.

## Quality gates

Run these before every commit. CI runs fmt, clippy and the tests on Linux,
and the build on Linux, macOS and Windows:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --locked --all-targets
```

## Testing

- Timer behaviour (keys, ticks, pomodoro phases, cache) lives in
  `src/app.rs` and takes `now: Instant` explicitly. Test it there with
  simulated time instead of sleeping.
- `tests/cli.rs` runs the real binary. Only add cases that exit **before**
  the screen opens; anything that opens it hangs when `cargo test` runs in a
  real terminal.
- The interactive screen isn't covered by automated tests. After a UI
  change, check it in a pseudo-terminal or by hand, and say which you did.
- Don't claim something works if you didn't run it. Say what you ran and
  what you couldn't check.

## Conventions

- Conventional Commits: `<type>(optional-scope): imperative summary`, e.g.
  `feat(pomodoro): skip to the next phase with N`.
- Work on a branch; `senpai` is the main branch. Commit or push only when the
  owner asks.
- Issues are GitHub Issues on `eRgo35/ti`; manage them with `gh`.
- Every user-visible change gets a line under `## [Unreleased]` in
  `CHANGELOG.md`, and the README is updated to match.
