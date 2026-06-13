# Contributing to strobetune

Thanks for your interest! Issues and pull requests are welcome.

## Prerequisites

- **Rust** (stable) — install from [rustup.rs](https://rustup.rs).
- **Linux:** ALSA development headers for the audio backend —
  `sudo apt install libasound2-dev` (Debian/Ubuntu) or
  `sudo dnf install alsa-lib-devel` (Fedora). macOS and Windows need nothing
  extra.

## Build & run

```sh
git clone https://github.com/cshamrick/strobetune
cd strobetune
cargo run --release
```

## Tests

```sh
cargo test
cargo clippy
```

Headless tests cover the parts that don't need audio hardware — please keep them
(and clippy) green:

- `strobe.rs` — folds synthetic sine waves through the real DSP path and asserts
  a matched tone is nearly stationary while detuned tones drift, oppositely for
  sharp vs. flat.
- `detector.rs` — feeds harmonic-rich tones (weak-fundamental low E, sympathetic
  high E, Drop D low D) and asserts the YIN detector snaps to the right string.
- `notes.rs` — transpose math and dynamic tuning display names.

## Project layout

| File          | Responsibility |
|---------------|----------------|
| `main.rs`     | CLI flags, terminal setup, run loop, clean restore |
| `app.rs`      | state, key handling, per-frame update |
| `audio.rs`    | cpal input stream → mono sample queue |
| `dsp.rs`      | band-pass + rectification (conditioning only) |
| `detector.rs` | optional YIN pitch detect for auto string select |
| `strobe.rs`   | wheel phase, lamp folding, accumulation |
| `ui.rs`       | ratatui layout & rendering |
| `notes.rs`    | tuning presets, note names, transpose |
| `config.rs`   | constants & defaults |

Most visual and DSP behaviour is tunable via constants in `config.rs` — e.g.
`DRIFT_HUE_STILL` / `DRIFT_HUE_FAST` / `DRIFT_SATURATION` and `DRIFT_FULL_RED`
for the drift colouring, and `BANDPASS_Q` / `ACCUM_DECAY` for the strobe response.

## Commits & releases

This repo uses **[Conventional Commits](https://www.conventionalcommits.org)**,
**squash-only merges**, and
**[release-please](https://github.com/googleapis/release-please)**:

- Open a PR with a Conventional Commit title (`feat:`, `fix:`, `docs:`, `chore:`,
  `ci:`, …). A CI check enforces the title format.
- Merges to `main` are squashed, using the PR title as the commit message — so
  `main`'s history stays conventional.
- release-please maintains a "release" PR that bumps the version and updates
  `CHANGELOG.md`. Merging it tags the release; CI then builds the macOS binaries
  (Apple Silicon + Intel) and updates the Homebrew formula automatically.

Pre-1.0, `feat:` bumps the minor version and `fix:` bumps the patch.
