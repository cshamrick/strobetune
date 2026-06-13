<p align="center">
  <img src="assets/logo.png" alt="strobetune" width="440">
</p>

<p align="center"><b>An analog-style strobe tuner for guitar, in your terminal.</b></p>

Tune your guitar by eye, the way an analog strobe tuner does. A selected note
spins a virtual reference wheel, your guitar drives a virtual lamp, and the
interaction between them paints a pattern that:

- **holds still** when the string is in tune,
- **drifts** one way when you're sharp, the other when you're flat.

There's **no pitch detection in the display** — no cents readout, no "in-tune"
light, just the drift, exactly like a real disc-and-lamp strobe.

## Install

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

```sh
cargo install --git https://github.com/cshamrick/strobetune
```

You'll need an audio input (built-in mic, interface, …) and ideally a truecolor
terminal for the smoothest colours. On macOS, the first run asks for microphone
permission.

## Usage

```sh
strobetune
```

Pick the string you're tuning — or just play, and strobetune detects it — then
watch the pattern. **Still means in tune; drifting means off.** Colour reinforces
it: a steady band reads teal and warms toward blue the faster it drifts.

```
1-6  select string        a      toggle auto string detect
t/T  next / prev tuning    [ ]    transpose ∓ semitone
0    reset transpose       q      quit
```

Auto-detect is **on by default** — play any string and it's selected for you.
Press `1`–`6` to take over manually.

### Tunings

| Preset    | Strings 1–6        |
|-----------|--------------------|
| Standard  | E2 A2 D3 G3 B3 E4  |
| Drop D    | D2 A2 D3 G3 B3 E4  |
| Open G    | D2 G2 D3 G3 B3 D4  |

Transpose shifts every string by `2^(semitones / 12)`, and the displayed name
follows along (e.g. `Standard E♭`, `Open A`).

## How it works

```
audio in → mono → band-pass @ reference → half-wave rectify → virtual lamp
selected tuning + string + transpose → active reference frequency → wheel phase
fold(lamp, wheel phase) → accumulate with persistence → strobe image → ratatui
```

The trick: the accumulator is indexed by **reference wheel phase**, not by time.
Each audio sample's lamp brightness lands in the bin for the wheel's current
phase. When the played period matches the wheel's, the bright moments always fall
in the same bin — a still band. When it's off, that coincidence point precesses
at the beat frequency and the band slides. No frequency is ever measured; the
drift is emergent.

The one place a real pitch algorithm appears is **auto string detection**, which
uses a YIN (autocorrelation) estimate to pick *which* string you're playing —
robust to the harmonic coincidences that fool simpler approaches. It only selects
the string; the display itself never sees a pitch.

## Contributing

Build steps, the module map, tests, and the release process live in
[CONTRIBUTING.md](CONTRIBUTING.md). Issues and PRs welcome.

## License

[MIT](LICENSE) — free to use, modify, and distribute without restriction.

<sub>strobetune models the general analog strobe-tuner principle. It isn't affiliated
with, compatible with, or equivalent to any commercial tuner product, and uses
original naming and visual design.</sub>
