# MIDI Gen

A small VST3 plugin, written in Rust, that generates melodies as MIDI. Pick a key, a scale, and a mood, and it plays a melody you can route to any instrument and record into a clip as starter material.

Built with [nih-plug](https://github.com/robbert-vdh/nih-plug) and its egui editor.

## What it does

- Generates a melody in the key and scale you choose.
- Lets you pick the character of the melody by **mood** instead of technical settings.
- Plays one melody on a loop until you press **Regenerate**, so you can record it, tweak it, or roll a new one.
- Outputs plain MIDI. It makes no sound itself; route it to a synth.

## Controls

| Control | What it does |
|---|---|
| **Key** | Root note of the melody (C to B). Changing it transposes the current melody. |
| **Scale** | Major, Minor, Dorian, Phrygian, Mixolydian, Harmonic Minor, or Minor Pentatonic. Changing it generates a new melody. |
| **Mood** | The character of the melody (see below). Changing it generates a new melody. |
| **Regenerate** | Throws away the current melody and generates a new one. |

### Moods

| Mood | Character |
|---|---|
| Dreamy | Sparse and floating, with long soft notes |
| Uplifting | Bright and rising, with bigger melodic jumps |
| Dark | Low, tense, and sparse |
| Groovy | Busy, syncopated, and bouncy (default) |
| Hypnotic | A short riff that locks in and repeats |
| Emotional | Slow, falling lines with long notes |

Each mood sets the technical details behind the scenes: how busy the rhythm is, note length, loudness, register, how often the melody leaps, whether it leans up or down, melody length (2 or 4 bars), and whether the hook repeats exactly.

## How the melodies are made

Melodies are random, but constrained by simple composition rules, so they sound intentional rather than noisy.

1. **Rhythm.** One of 11 hand-written one-bar rhythms is chosen, weighted by the mood's density.
2. **Hook.** A short motif is built as a random walk through the scale: mostly steps, some thirds, rare leaps. A leap is usually followed by a step back. The contour is pulled toward the middle of the range and nudged up or down by the mood.
3. **Anchoring.** Notes on strong beats are often snapped to the tonic triad, so the melody feels rooted in the key. Off-beat notes are free to be passing tones.
4. **Call and response.** The hook ends on an unresolved degree (a "question"). Later bars reuse the hook as a transposed sequence or an exact return, and the final bar answers it by landing on the tonic and holding it.
5. **Playback.** The finished melody is stored as a fixed list of notes and looped, so it doesn't change while you record.

A random number generator seeded from the system clock provides the variety. Each plugin instance starts with a different melody, and each press of Regenerate draws a new one.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- macOS with Xcode Command Line Tools (the setup below is for Apple Silicon)
- A DAW that supports VST3 plugins with MIDI output (tested with Ableton Live)

## Build and install

```bash
# Build the VST3 bundle
cargo xtask bundle midi_gen --release

# Install it for the current user
cp -R target/bundled/midi_gen.vst3 ~/Library/Audio/Plug-Ins/VST3/
```

Then rescan plugins in your DAW. In Ableton Live, Option-click **Rescan** in Settings → Plug-Ins, and make sure **Use VST3 System Folders** is on.

To avoid copying after every build, you can add `target/bundled` as a custom VST3 folder in your DAW instead, and delete the copy in `~/Library/Audio/Plug-Ins/VST3/` so there aren't two versions.

## Using it in Ableton Live

1. Add **MIDI Gen** to a MIDI track.
2. Create a second MIDI track with an instrument (a synth).
3. On the second track, set **MIDI From** to the MIDI Gen track, and set the second dropdown to **MIDI Gen**.
4. Set the second track's **Monitor** to **In**.
5. Press play. The plugin only emits notes while the transport is running.

To capture a melody, arm the second track, enable record, and let it play. You get a normal MIDI clip you can edit in the piano roll. Pressing Regenerate while recording starts the new melody immediately, so for clean takes, regenerate first and then record.

## Troubleshooting

**Linker error about `unknown architecture arm64e.x1-macos` (or similar `.tbd` errors).**
The Command Line Tools SDK is newer than the installed linker. Point the build at an older SDK:

```bash
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk
cargo clean
cargo xtask bundle midi_gen --release
```

Use whichever older SDK exists in `/Library/Developer/CommandLineTools/SDKs/`. To make it permanent, add the export to `~/.zshrc`, or set it under `[env]` in `.cargo/config.toml`.

**The plugin doesn't appear in the DAW.**
Search for the display name **MIDI Gen** (with a space), not `midi_gen`. Then Option-click Rescan in Live. If it still isn't listed, clear the quarantine flag and re-sign it:

```bash
xattr -cr ~/Library/Audio/Plug-Ins/VST3/midi_gen.vst3
codesign --force --deep --sign - ~/Library/Audio/Plug-Ins/VST3/midi_gen.vst3
```

**"This VST3 plug-in could not be opened" in Live.**
Check Live's log (`~/Library/Preferences/Ableton/Live <version>/Log.txt`) for lines mentioning `MIDI Gen`. Live rejects instrument-category plugins that don't declare a MIDI input bus, which is why the plugin sets `MIDI_INPUT` to `MidiConfig::Basic` even though it ignores incoming notes.

**No sound, or the notes don't reach the synth.**
Live's handling of VST3 MIDI output is limited. If MIDI Gen isn't listed as a source in the other track's **MIDI From** menu, test the plugin in a host with fuller support (Reaper, Bitwig) to confirm it works.

## Project structure

```
.
├── Cargo.toml          # crate config; nih_plug and nih_plug_egui from git
├── .cargo/config.toml  # `cargo xtask` alias
├── xtask/              # nih-plug bundler that produces the .vst3
└── src/
    ├── lib.rs          # plugin entry point: thin glue between nih-plug and the engine
    ├── params.rs       # host-visible parameters (key, scale, mood)
    ├── editor.rs       # the egui window
    ├── engine.rs       # playback: transport position -> timed MIDI events
    ├── melody.rs       # melody generation: rhythms, hook, call and response
    ├── mood.rs         # mood presets and the technical settings behind them
    ├── theory.rs       # keys, scales, scale position -> MIDI note
    └── rng.rs          # tiny random number generator
```

Where to make changes:

- **New mood:** `mood.rs` only (add a variant, list it in `ALL`, fill in its label, tagline, and settings).
- **New scale:** `theory.rs` only (add a variant and its intervals, stable degrees, and open degrees).
- **New rhythms or melody rules:** `melody.rs`.
- **New control in the window:** add the parameter in `params.rs`, draw it in `editor.rs`, and pass its value into `BlockInput` in `lib.rs`.
- **New output behavior (for example chords mode):** `engine.rs` plus a new generator module next to `melody.rs`. The engine has no nih-plug dependency, so it's easy to extend and reason about.

## Limitations

- Assumes 4/4 time.
- Melody only. There is no chord mode yet.
- The melody isn't aware of the chords you play underneath it, so passing notes can clash.
- Only 11 rhythm templates, so patterns repeat across melodies.
- The melody is not saved with the project. Record it to a clip to keep it.

## Ideas for next steps

- Chords mode that reuses the same phrase structure, driven by a chord progression.
- Generate several candidate melodies internally and keep the best-scoring one.
- Follow the host's time signature.
- More rhythm templates and more moods.
- Export the current melody as a `.mid` file straight from the plugin.
