<p align="center">
  <img src="assets/logo.png" alt="strobetune" width="440">
</p>

# strobetune

An analog-style **strobe tuner** for the terminal, written in Rust.

A selected note drives a virtual rotating reference wheel; your live audio drives
a virtual lamp. The display is formed purely by the *phase interaction* between
the two:

- Play a string **in tune** with the selected reference → the pattern is **still**.
- Play it **sharp** or **flat** → the pattern **drifts**, in opposite directions.

There is **no pitch detection** anywhere in the display path — no FFT, no
autocorrelation, no cents, no "in tune" light. The motion is an emergent
stroboscopic effect, exactly like an analog disc-and-lamp tuner. (The optional
auto string detection does use a pitch estimate, but *only* to pick which string
is selected — never to drive the display; see below.)

## How it works

```
audio in → mono → band-pass @ reference → half-wave rectify → virtual lamp
selected tuning + string + transpose → active reference frequency → wheel phase
fold(lamp, wheel phase) → accumulate with persistence → strobe image → ratatui
```

The one subtlety that makes the effect appear: the accumulator is indexed by
**reference wheel phase**, not by time. Each input sample's lamp brightness is
added to the bin for the wheel's *current* phase (phase folding). When the input
period matches the wheel period, the bright moments always land in the same bin
(a still band). When detuned, the coincidence point precesses around the wheel
at the beat frequency, so the band slides. Per-frame decay gives visual
persistence and lets the motion read smoothly.

## Prerequisites

- **Rust toolchain** (stable) — install from [rustup.rs](https://rustup.rs).
- **An audio input device** — strobetune listens to your system's default input
  (built-in mic, audio interface, etc.).
- **A truecolor terminal** (24-bit) for the smoothest strobe colours — most
  modern terminals qualify.
- **Linux only:** ALSA development headers are needed to build the audio
  backend — e.g. `sudo apt install libasound2-dev` (Debian/Ubuntu) or
  `sudo dnf install alsa-lib-devel` (Fedora).
- **macOS:** the first run prompts for microphone access; grant it under
  System Settings → Privacy & Security → Microphone.

## Install

If you just want to use it:

### Homebrew (macOS)

```sh
brew install cshamrick/tap/strobetune
```

Or tap first, then install by name:

```sh
brew tap cshamrick/tap
brew install strobetune
```

### Cargo (any platform)

Installs the `strobetune` command straight from the repo — no clone required:

```sh
cargo install --git https://github.com/cshamrick/strobetune
```

Then run it from anywhere:

```sh
strobetune
```

## Build from source

For development or contributing:

```sh
git clone https://github.com/cshamrick/strobetune
cd strobetune
cargo run --release     # launch
cargo test              # run the headless tests
```

## Controls

```
1-6  select string        a      toggle auto string detect
t/T  next / prev tuning    [ ]    transpose ∓ semitone
0    reset transpose       q      quit
```

The bands are coloured by **apparent drift speed**: a still (in-tune) band sits
at a calm teal and shifts toward blue the faster the strobe image rotates. Speed
is measured from the frame-to-frame motion of the displayed band (the
circular-mean of the accumulator) — it reads the *picture's* motion, not the
input's pitch, so there's still no frequency or cents calculation. The palette
and sensitivity are set by `DRIFT_HUE_STILL` / `DRIFT_HUE_FAST` /
`DRIFT_SATURATION` and `DRIFT_FULL_RED` in `config.rs`.

## Auto string detection (`a`)

**On by default** — it selects the string for you; press `a` to toggle it off,
or just press `1`–`6` to take over manually. It uses a **YIN pitch estimate**
(autocorrelation) to find the played note's period and snaps to the nearest
string of the current tuning. Periodicity-based detection is robust to the
harmonic coincidences that defeat a per-band energy approach in standard tuning
(a low E's overtones land on the B3 and E4 bands; high E sympathetically rings
the low E string) — it locks onto the true fundamental regardless.

This is the one place a real pitch algorithm is used, and **only** to pick the
string. The strobe **display path never sees a pitch estimate** — it stays pure
phase folding against the selected reference, so the tuning feedback itself is
still stroboscopic drift, not a computed pitch. Pressing `1`–`6` returns to
manual selection.

## Tunings

| Preset    | Strings 1–6              |
|-----------|--------------------------|
| Standard  | E2 A2 D3 G3 B3 E4        |
| Drop D    | D2 A2 D3 G3 B3 E4        |
| Open G    | D2 G2 D3 G3 B3 D4        |

Transpose shifts every string by `2^(semitones/12)` and updates the displayed
tuning name (e.g. `Standard E♭`, `Open A`).

## Modules

| File         | Responsibility |
|--------------|----------------|
| `main.rs`    | terminal setup, run loop, clean restore |
| `app.rs`     | state, key handling, per-frame update |
| `audio.rs`   | cpal input stream → mono sample queue |
| `dsp.rs`     | band-pass + rectification (conditioning only) |
| `detector.rs`| optional YIN pitch detect for auto string select |
| `strobe.rs`  | wheel phase, lamp folding, accumulation |
| `ui.rs`      | ratatui layout & rendering |
| `notes.rs`   | tuning presets, note names, transpose |
| `config.rs`  | constants and defaults |

## Tests

```sh
cargo test
```

Headless tests cover the parts that don't need audio hardware:

- `strobe.rs` folds synthetic sine waves through the real DSP path and asserts a
  matched tone is nearly stationary while detuned tones drift, in opposite
  directions for sharp vs. flat — the core behavior.
- `detector.rs` feeds harmonic-rich tones (weak-fundamental low E, sympathetic
  high E, Drop D low D, etc.) and asserts the YIN detector snaps to the right
  string despite the harmonic coincidences.
- `notes.rs` checks the transpose math and the dynamic tuning display names.

## Notes

This models the general analog strobe-tuner *principle*. It is not affiliated
with, compatible with, or equivalent to any commercial tuner product, and uses
original naming and visual design.

## License

Released under the [MIT License](LICENSE) — free to use, modify, and distribute
without restriction.
