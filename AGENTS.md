# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project

**strobetune** is an analog-style strobe tuner for guitar that runs in the
terminal (Rust, with `ratatui` for the UI and `cpal` for audio). A selected note
drives a virtual rotating reference wheel; live audio drives a virtual lamp; the
phase interaction between them paints a pattern that holds **still** when the
string is in tune and **drifts** when it's sharp or flat.

## The one hard rule

**The strobe display must never be driven by pitch detection.** No FFT,
autocorrelation, cents, or "in-tune" decision in the display path. The motion is
an emergent stroboscopic effect produced by folding the input-driven lamp signal
over the reference wheel's phase (`strobe.rs`). Do not "simplify" this into a
detected pitch or a cents readout — that would defeat the entire concept.

The *only* place a pitch algorithm is allowed is `detector.rs` (optional auto
string selection via YIN), and it is used **only** to choose which string is
selected. It must never feed the display path.

## Commands

```sh
cargo run --release     # launch the app
cargo test              # headless tests (no audio hardware needed)
cargo clippy            # keep this clean
cargo build --release   # release binary
```

Keep `cargo test` and `cargo clippy` green — both exercise the DSP and detector
logic, which is where bugs hide.

## Layout

| File          | Responsibility |
|---------------|----------------|
| `main.rs`     | CLI flags, terminal setup, run loop, clean restore |
| `app.rs`      | app state, key handling, per-frame update |
| `audio.rs`    | cpal input stream → mono sample queue |
| `dsp.rs`      | band-pass + rectification (input conditioning only) |
| `detector.rs` | optional YIN pitch detection for auto string select |
| `strobe.rs`   | wheel phase, lamp folding, accumulation |
| `ui.rs`       | ratatui layout & rendering |
| `notes.rs`    | tuning presets, note names, transpose helpers |
| `config.rs`   | tunable constants (display palette, DSP, detector) |

Prefer adjusting behaviour through a `config.rs` constant when one exists rather
than scattering magic numbers. Keep the audio callback in `audio.rs` minimal and
non-blocking.

## Gotchas

- **Dependency APIs are newer than many training cutoffs — verify, don't guess.**
  `cpal` 0.18: `StreamConfig.sample_rate` is a plain `u32`, `build_input_stream`
  takes the config **by value**, errors are the unified `cpal::Error`, and a
  device name comes from `device.description()`. `ratatui` 0.30: use
  `ratatui::init()` / `ratatui::restore()`, and crossterm is re-exported as
  `ratatui::crossterm` — do **not** add a separate `crossterm` dependency (it
  causes type-mismatch version skew).
- **String numbering runs low→high:** `1 = E2` (low E) … `6 = E4` (high E), the
  reverse of the usual guitar convention. See `notes.rs`.
- **The app can't run headless.** With no args, `strobetune` launches a
  fullscreen TUI that needs a real terminal and a microphone, so it won't run in
  CI or a non-interactive shell. Use `cargo test` for logic and
  `strobetune --version` / `--help` for a smoke check.

## Conventions

- **Conventional Commits** with **squash-only merges**. The squash commit takes
  the PR title, so PR titles must be conventional — a CI check enforces this.
- `main` is protected; land changes via a pull request. Pre-1.0, `feat:` bumps
  the minor version and `fix:` bumps the patch.
- Releases are automated by **release-please**: merging its release PR tags the
  version, builds the macOS binaries (Apple Silicon + Intel), and updates the
  Homebrew formula in `cshamrick/homebrew-tap`. Do **not** tag releases or edit
  the version in `Cargo.toml` by hand.
- User-facing docs go in `README.md`; build/dev/release docs go in
  `CONTRIBUTING.md`.
- Match the surrounding style. Comments should explain *why* or note a
  constraint, not restate the obvious.

## Testing notes

Audio capture and the TUI can't run in CI, so the logic is tested with synthetic
signals: strobe stationarity vs. drift, and the detector's robustness to the
harmonic coincidences that plague guitar (e.g. a low E's overtones, and
`E4 ≈ 3 × A2`). When you touch `strobe.rs` or `detector.rs`, add tests in the
same style instead of relying on manual listening.
