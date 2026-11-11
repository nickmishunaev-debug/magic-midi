//! The playback engine: turns the host's transport position into timed MIDI events.
//!
//! This module has no dependency on nih-plug. `lib.rs` fills in a `BlockInput` each audio
//! block, calls `Engine::process`, and sends the returned `MidiOut` events to the host.
//!
//! The engine plays a looping list of `Note`s. Which generator produced them (melody or chords)
//! doesn't matter here. To add a new generator, produce `Note`s in `regenerate`.

use std::cmp::Ordering;

use crate::chords::{generate_chords, ChordCfg};
use crate::melody::{generate_phrase, GenCfg, PNote};
use crate::mode::Mode;
use crate::mood::Mood;
use crate::note::Note;
use crate::rng::Rng;
use crate::theory::{Key, ScaleKind};

/// Everything goes out on MIDI channel 1; the DAW's track routing decides where it lands.
pub const MIDI_CHANNEL: u8 = 0;

/// What the engine needs to know about one audio block.
pub struct BlockInput {
    pub playing: bool,
    pub tempo: f64,
    pub sample_rate: f64,
    /// Host position in beats at the start of the block, if the host reports it
    pub host_pos: Option<f64>,
    pub num_samples: usize,
    pub mode: Mode,
    pub key: Key,
    pub scale: ScaleKind,
    pub mood: Mood,
    /// True if the Regenerate button was pressed since the last block
    pub regen: bool,
}

/// A MIDI event with its sample offset inside the block.
#[derive(Clone, Copy)]
pub struct MidiOut {
    pub timing: u32,
    pub on: bool,
    pub note: u8,
    pub velocity: f32,
}

#[derive(Clone, Copy)]
struct ActiveNote {
    note: u8,
    off_beat: f64,
}

#[derive(Clone, Copy)]
struct Ev {
    beat: f64,
    on: bool,
    note: u8,
    velocity: f32,
}

pub struct Engine {
    rng: Rng,

    // Scratch space for the melody generator
    melody: Vec<PNote>,
    motif: Vec<PNote>,

    // The current loop and the settings it was generated with
    notes: Vec<Note>,
    phrase_valid: bool,
    phrase_bars: u32,
    phrase_scale: ScaleKind,
    phrase_mood: Mood,
    phrase_mode: Mode,

    // Playback state
    active: Vec<ActiveNote>,
    events: Vec<Ev>,
    out: Vec<MidiOut>,
    was_playing: bool,
    beat_pos: f64,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            // A different result every time the plugin is added
            rng: Rng::from_clock(),
            melody: Vec::with_capacity(256),
            motif: Vec::with_capacity(64),
            notes: Vec::with_capacity(512),
            phrase_valid: false,
            phrase_bars: 0,
            phrase_scale: ScaleKind::Minor,
            phrase_mood: Mood::Groovy,
            phrase_mode: Mode::Melody,
            active: Vec::with_capacity(64),
            events: Vec::with_capacity(512),
            out: Vec::with_capacity(512),
            was_playing: false,
            beat_pos: 0.0,
        }
    }

    /// Called when the host resets the plugin. Sounding notes are released on the next block.
    pub fn reset(&mut self) {
        self.was_playing = false;
        self.beat_pos = 0.0;
    }

    /// Release every sounding note at the start of the block.
    fn flush_active(&mut self) {
        for a in self.active.drain(..) {
            self.out.push(MidiOut {
                timing: 0,
                on: false,
                note: a.note,
                velocity: 0.0,
            });
        }
    }

    /// Generates a fresh loop for the current mode, mood and scale.
    fn regenerate(&mut self, input: &BlockInput, loop_bars: u32) {
        self.notes.clear();

        match input.mode {
            Mode::Melody => {
                let ms = input.mood.settings();
                let cfg = GenCfg::new(input.scale, &ms);
                generate_phrase(&cfg, &mut self.rng, &mut self.motif, &mut self.melody);
                for p in self.melody.iter() {
                    self.notes.push(Note {
                        step: p.step,
                        len: p.len,
                        semi: input.scale.midi(0, p.pos),
                        vel_off: p.vel_off,
                        offset: 0.0,
                    });
                }
            }
            Mode::Chords => {
                let cs = input.mood.chord_settings();
                let cfg = ChordCfg::new(input.scale, &cs);
                generate_chords(&cfg, &mut self.rng, &mut self.notes);
            }
        }

        self.phrase_valid = true;
        self.phrase_bars = loop_bars;
        self.phrase_scale = input.scale;
        self.phrase_mood = input.mood;
        self.phrase_mode = input.mode;
    }

    /// Processes one block and returns the MIDI events to send, in time order.
    pub fn process(&mut self, input: &BlockInput) -> &[MidiOut] {
        self.out.clear();

        if input.num_samples == 0 {
            return &self.out;
        }

        if !input.playing {
            self.flush_active();
            self.was_playing = false;
            return &self.out;
        }

        let num_samples = input.num_samples;
        let beats_per_sample = input.tempo / 60.0 / input.sample_rate;

        // Follow our own running position so consecutive blocks never overlap or leave gaps;
        // re-sync to the host if we drift, and detect jumps/loops.
        let starting = !self.was_playing;
        let (block_start, jumped) = if starting {
            (input.host_pos.unwrap_or(0.0), false)
        } else if let Some(h) = input.host_pos {
            let d = (h - self.beat_pos).abs();
            if d > 0.25 {
                (h, true)
            } else if d > 0.02 {
                (h, false)
            } else {
                (self.beat_pos, false)
            }
        } else {
            (self.beat_pos, false)
        };
        let block_end = block_start + num_samples as f64 * beats_per_sample;
        self.was_playing = true;

        // The mood decides all the technical details, differently for each mode
        let (loop_bars, gate, base_vel, tonic) = match input.mode {
            Mode::Melody => {
                let ms = input.mood.settings();
                (ms.bars, ms.gate, ms.velocity, 60 + input.key.semitone() + 12 * ms.octave)
            }
            Mode::Chords => {
                let cs = input.mood.chord_settings();
                (cs.loop_bars(), cs.gate, cs.velocity, 60 + input.key.semitone() + 12 * cs.octave)
            }
        };

        // The loop is only replaced when Regenerate is pressed, or when the mode, mood or scale
        // changes (the old loop can't be mapped onto a different scale or character).
        // Changing the key just transposes the current loop.
        if input.regen
            || self.phrase_bars != loop_bars
            || self.phrase_scale != input.scale
            || self.phrase_mood != input.mood
            || self.phrase_mode != input.mode
        {
            self.phrase_valid = false;
        }

        if starting || jumped {
            self.flush_active();
        }

        // Schedule events for every repeat of the loop this block touches
        self.events.clear();
        let loop_beats = loop_bars as f64 * 4.0; // assumes 4/4
        let first_slot = (block_start / loop_beats).floor() as i64;
        let last_slot = (((block_end - 1e-9) / loop_beats).floor() as i64).max(first_slot);

        for slot in first_slot..=last_slot {
            if !self.phrase_valid {
                self.regenerate(input, loop_bars);
            }

            let slot_start = slot as f64 * loop_beats;

            for n in self.notes.iter() {
                let nominal = slot_start + n.step as f64 * 0.25;
                let start = nominal + n.offset as f64;
                if start < block_start || start >= block_end {
                    continue;
                }
                let midi = (tonic + n.semi).clamp(0, 127) as u8;
                let vel = (base_vel + n.vel_off).clamp(1, 127) as f32 / 127.0;
                // Notes end at their nominal time, so a strummed note is just a bit shorter
                // and never runs into the next chord
                let held = (n.len as f64 * 0.25 * gate).max(n.offset as f64 + 0.05);

                self.events.push(Ev {
                    beat: start,
                    on: true,
                    note: midi,
                    velocity: vel,
                });
                self.active.push(ActiveNote {
                    note: midi,
                    off_beat: nominal + held,
                });
            }
        }

        // Note-offs that fall inside this block
        let mut i = 0;
        while i < self.active.len() {
            let a = self.active[i];
            if a.off_beat < block_end {
                self.events.push(Ev {
                    beat: a.off_beat.max(block_start),
                    on: false,
                    note: a.note,
                    velocity: 0.0,
                });
                self.active.swap_remove(i);
            } else {
                i += 1;
            }
        }

        // Time order; at equal times, note-offs go before note-ons
        self.events.sort_unstable_by(|a, b| {
            a.beat
                .partial_cmp(&b.beat)
                .unwrap_or(Ordering::Equal)
                .then(a.on.cmp(&b.on))
        });

        // Convert beat positions to sample offsets inside the block
        for e in self.events.iter() {
            let timing = (((e.beat - block_start) / beats_per_sample).round().max(0.0) as u32)
                .min(num_samples as u32 - 1);
            self.out.push(MidiOut {
                timing,
                on: e.on,
                note: e.note,
                velocity: e.velocity,
            });
        }

        self.beat_pos = block_end;
        &self.out
    }
}
