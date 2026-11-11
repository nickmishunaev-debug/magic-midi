//! Chord generation: progressions, chord colours, voice leading, and stab rhythms.
//!
//! Like `melody.rs`, this module knows nothing about plugins or MIDI. It turns settings into notes.
//!
//! What makes each result different (all random, all within deep-house habits):
//!   1. Progression: picked from a library of common loops (i-VI-III-VII, i-iv-VII-III, ii-V-I-vi ...),
//!      plus mode-specific ones, plus two-chord vamps for riffy moods. Sometimes the last chord is
//!      swapped for a turnaround (v or VII) so the loop pulls back to the start.
//!   2. Chord colour: every chord is built by stacking diatonic thirds, so each scale/mode gets its
//!      own chord qualities automatically. Triad, 7th or 9th, sometimes without the 5th, and
//!      occasionally a suspended 4th instead of the 3rd.
//!   3. Voicing: every chord is placed in the same register and given the inversion that moves
//!      the least from the previous chord (voice leading), with some randomness among near-ties.
//!   4. Rhythm: a stab pattern (sustained pad, offbeat house stabs, syncopated piano, or sparse hits)
//!      repeated each bar, with an occasional different pattern on the last bar as a turnaround.

use crate::mood::ChordSettings;
use crate::note::Note;
use crate::rng::Rng;
use crate::theory::ScaleKind;

/// How the chords are played rhythmically.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ChordRhythm {
    /// One long chord that sustains for its whole length
    Pad,
    /// Classic house stabs on the off-beats
    Offbeat,
    /// Syncopated deep-house piano/organ stabs
    Syncopated,
    /// A few spaced-out hits per bar
    Sparse,
}

#[derive(Clone, Copy)]
pub struct ChordCfg {
    pub scale: ScaleKind,
    pub bars_per_chord: u32,
    pub richness: u8,
    pub rhythm: ChordRhythm,
    pub strum: f32,
    pub variation: f32,
    pub vamp: bool,
}

impl ChordCfg {
    pub fn new(scale: ScaleKind, cs: &ChordSettings) -> Self {
        Self {
            scale,
            bars_per_chord: cs.bars_per_chord,
            richness: cs.richness,
            rhythm: cs.rhythm,
            strum: cs.strum,
            variation: cs.variation,
            vamp: cs.vamp,
        }
    }
}

// ───────────────────────── Progression library ─────────────────────────
// Each progression is four scale-degree indices (0 = I/i, 1 = ii, 2 = III, 3 = IV/iv, 4 = V/v,
// 5 = VI/vi, 6 = VII). The chord quality comes from the scale, so the same degrees work in any mode.

type Prog = [usize; 4];

const MINOR_PROGS: &[Prog] = &[
    [0, 5, 2, 6], // i  - VI  - III - VII   (the classic)
    [0, 3, 6, 2], // i  - iv  - VII - III
    [0, 6, 5, 6], // i  - VII - VI  - VII
    [0, 3, 0, 6], // i  - iv  - i   - VII
    [0, 2, 6, 3], // i  - III - VII - iv
    [5, 6, 0, 0], // VI - VII - i   - i
    [0, 5, 3, 6], // i  - VI  - iv  - VII
    [0, 3, 4, 0], // i  - iv  - v   - i
    [0, 4, 5, 3], // i  - v   - VI  - iv
    [0, 2, 5, 6], // i  - III - VI  - VII
];

const MAJOR_PROGS: &[Prog] = &[
    [0, 5, 3, 4], // I  - vi - IV - V
    [1, 4, 0, 5], // ii - V  - I  - vi
    [0, 2, 3, 4], // I  - iii - IV - V
    [5, 3, 0, 4], // vi - IV - I  - V
    [0, 3, 5, 4], // I  - IV - vi - V
    [0, 5, 1, 4], // I  - vi - ii - V
    [0, 4, 5, 3], // I  - V  - vi - IV
    [1, 4, 0, 0], // ii - V  - I  - I
];

const VAMPS_MINOR: &[Prog] = &[
    [0, 3, 0, 3], // i - iv, repeated
    [0, 5, 0, 5], // i - VI, repeated
    [0, 6, 0, 6], // i - VII, repeated
];

const VAMPS_MAJOR: &[Prog] = &[
    [1, 4, 1, 4], // ii - V, repeated
    [0, 3, 0, 3], // I - IV, repeated
    [0, 5, 0, 5], // I - vi, repeated
];

const DORIAN_EXTRA: &[Prog] = &[[0, 3, 0, 6], [0, 3, 6, 3]];
const PHRYGIAN_EXTRA: &[Prog] = &[[0, 1, 0, 6], [0, 1, 6, 0]];
const MIXOLYDIAN_EXTRA: &[Prog] = &[[0, 6, 3, 0], [0, 6, 3, 3]];
const HARMONIC_EXTRA: &[Prog] = &[[0, 3, 4, 0], [0, 5, 4, 0]];

fn is_minor(scale: ScaleKind) -> bool {
    scale.chord_scale().intervals()[2] == 3
}

fn mode_extras(scale: ScaleKind) -> &'static [Prog] {
    match scale {
        ScaleKind::Dorian => DORIAN_EXTRA,
        ScaleKind::Phrygian => PHRYGIAN_EXTRA,
        ScaleKind::Mixolydian => MIXOLYDIAN_EXTRA,
        ScaleKind::HarmonicMinor => HARMONIC_EXTRA,
        _ => &[],
    }
}

/// Semitones from the scale's tonic to position `i` in the seven-note chord scale.
fn semis_at(scale: ScaleKind, i: usize) -> i32 {
    scale.chord_scale().intervals()[i % 7] + 12 * (i / 7) as i32
}

/// True if the chord built on degree `d` is a plain major or minor triad. The progression library
/// is written for natural minor and major, so in other modes some degrees come out diminished
/// or augmented (for example the VI chord in Dorian). Those are skipped.
fn triad_ok(scale: ScaleKind, d: usize) -> bool {
    let third = semis_at(scale, d + 2) - semis_at(scale, d);
    let fifth = semis_at(scale, d + 4) - semis_at(scale, d);
    (third == 3 || third == 4) && fifth == 7
}

fn prog_ok(scale: ScaleKind, prog: &Prog) -> bool {
    prog.iter().all(|&d| triad_ok(scale, d))
}

fn pick_progression(rng: &mut Rng, scale: ScaleKind, vamp: bool) -> Prog {
    let minor = is_minor(scale);
    let extras = mode_extras(scale);

    for _ in 0..16 {
        let prog = if vamp {
            let pool = if minor { VAMPS_MINOR } else { VAMPS_MAJOR };
            pool[rng.range(pool.len())]
        } else if !extras.is_empty() && rng.chance(0.4) {
            extras[rng.range(extras.len())]
        } else {
            let pool = if minor { MINOR_PROGS } else { MAJOR_PROGS };
            pool[rng.range(pool.len())]
        };
        if prog_ok(scale, &prog) {
            return prog;
        }
    }
    // Every scale offered has a usable tonic and subdominant chord
    [0, 3, 0, 3]
}

// ───────────────────────── Chord colour ─────────────────────────

/// How many chord tones: 3 = triad, 4 = 7th, 5 = 9th.
fn pick_tone_count(rng: &mut Rng, richness: u8) -> usize {
    let r = rng.f();
    match richness {
        0 => {
            if r < 0.6 {
                3
            } else {
                4
            }
        }
        1 => {
            if r < 0.1 {
                3
            } else if r < 0.7 {
                4
            } else {
                5
            }
        }
        _ => {
            if r < 0.4 {
                4
            } else {
                5
            }
        }
    }
}

/// Stacks thirds on scale degree `d` (in scale positions). Returns the positions and how many.
/// Sometimes drops the 5th (root-3rd-7th-9th voicings are very common in deep house),
/// and occasionally replaces the 3rd with the 4th for a suspended sound.
fn chord_tones(rng: &mut Rng, d: i32, count: usize, sus: bool, skip_seventh: bool) -> ([i32; 5], usize) {
    // With `skip_seventh` the chord gets an added 9th instead (e.g. a minor add9)
    let all = if skip_seventh {
        [d, d + 2, d + 4, d + 8, d + 8]
    } else {
        [d, d + 2, d + 4, d + 6, d + 8]
    };
    let count = if skip_seventh { count.min(4) } else { count };
    let drop_fifth = count >= 4 && rng.chance(0.45);
    let mut tones = [0i32; 5];
    let mut n = 0;
    for (i, &pos) in all.iter().enumerate().take(count) {
        if i == 2 && drop_fifth {
            continue;
        }
        tones[n] = if i == 1 && sus { d + 3 } else { pos };
        n += 1;
    }
    (tones, n)
}

// ───────────────────────── Voicing and voice leading ─────────────────────────

/// Where chords should sit, in semitones relative to the tonic.
const CENTRE: f32 = -5.0;

/// Chooses the inversion and octave for a chord. Candidates are scored on staying near the
/// target register, not spreading too wide, and moving as little as possible from the previous
/// chord; a little randomness picks among near-equal options.
fn voice(scale: ScaleKind, rng: &mut Rng, tones: &[i32], prev: &[i32]) -> ([i32; 5], usize) {
    let m = tones.len();
    let mut best = [0i32; 5];
    let mut best_cost = f32::MAX;

    for inv in 0..m {
        for k in -2i32..=1 {
            let mut semis = [0i32; 5];
            for j in 0..m {
                let up = if j < inv { 7 } else { 0 };
                semis[j] = scale.midi(0, tones[j] + up + 7 * k);
            }
            semis[..m].sort_unstable();

            let lo = semis[0];
            let hi = semis[m - 1];
            let mean = semis[..m].iter().sum::<i32>() as f32 / m as f32;

            let mut cost = (mean - CENTRE).abs() * 0.5;
            let span = (hi - lo) as f32;
            if span > 16.0 {
                cost += (span - 16.0) * 1.5;
            }
            let floor = CENTRE as i32 - 12;
            if lo < floor {
                cost += (floor - lo) as f32;
            }
            for &c in &semis[..m] {
                let nearest = prev.iter().map(|&p| (c - p).abs()).min().unwrap_or(0);
                cost += nearest as f32 * 0.8;
            }
            cost += rng.f() * 2.0;

            if cost < best_cost {
                best_cost = cost;
                best = semis;
            }
        }
    }
    (best, m)
}

// ───────────────────────── Rhythm ─────────────────────────

type Hits = &'static [(u32, u32)];

/// Hits are (start step, length in steps), 16 steps per bar.
const OFFBEAT: &[Hits] = &[
    &[(2, 2), (6, 2), (10, 2), (14, 2)],
    &[(2, 2), (6, 2), (10, 2), (13, 3)],
    &[(2, 3), (6, 2), (10, 3), (14, 2)],
    &[(2, 1), (6, 1), (10, 1), (14, 1)],
];

const SYNCOPATED: &[Hits] = &[
    &[(0, 3), (3, 3), (6, 2), (10, 3), (14, 2)],
    &[(0, 2), (3, 2), (6, 3), (10, 2), (13, 3)],
    &[(0, 3), (3, 2), (8, 3), (11, 3)],
    &[(3, 3), (6, 3), (10, 3), (14, 2)],
    &[(0, 2), (2, 2), (6, 2), (8, 2), (11, 2), (14, 2)],
];

const SPARSE: &[Hits] = &[
    &[(0, 4), (10, 4)],
    &[(0, 6), (8, 6)],
    &[(0, 3), (6, 3), (12, 4)],
    &[(3, 4), (11, 4)],
];

fn templates(r: ChordRhythm) -> &'static [Hits] {
    match r {
        ChordRhythm::Pad => &[],
        ChordRhythm::Offbeat => OFFBEAT,
        ChordRhythm::Syncopated => SYNCOPATED,
        ChordRhythm::Sparse => SPARSE,
    }
}

fn hit_accent(step: u32, rng: &mut Rng) -> i32 {
    let base = if step % 16 == 0 {
        8
    } else if step % 4 == 0 {
        3
    } else if step % 2 == 1 {
        -6
    } else {
        0
    };
    base + (rng.range(5) as i32 - 2)
}

/// Plays the whole chord at once (or strummed, if the mood asks for it).
fn push_hit(out: &mut Vec<Note>, rng: &mut Rng, cfg: &ChordCfg, semis: &[i32], step: u32, len: u32) {
    let acc = hit_accent(step, rng);
    let m = semis.len();
    for (j, &s) in semis.iter().enumerate() {
        let top_voice = if j == m - 1 { 4 } else { 0 };
        out.push(Note {
            step,
            len,
            semi: s,
            vel_off: acc + top_voice + (rng.range(5) as i32 - 2),
            offset: cfg.strum * j as f32,
        });
    }
}

// ───────────────────────── Generator ─────────────────────────

/// Builds a four-chord loop (`4 * bars_per_chord` bars) into `out`.
pub fn generate_chords(cfg: &ChordCfg, rng: &mut Rng, out: &mut Vec<Note>) {
    out.clear();

    let scale = cfg.scale.chord_scale();
    let bpc = cfg.bars_per_chord;
    let total_bars = 4 * bpc;

    // 1. Progression, with an occasional turnaround on the last chord
    let mut prog = pick_progression(rng, cfg.scale, cfg.vamp);
    if !cfg.vamp && rng.chance(cfg.variation * 0.35) {
        let turn = if is_minor(cfg.scale) { [4, 6][rng.range(2)] } else { 4 };
        if triad_ok(cfg.scale, turn) {
            prog[3] = turn;
        }
    }

    // 2. Rhythm: one main pattern, and an alternative for the last bar
    let tpls = templates(cfg.rhythm);
    let (main, alt): (Hits, Hits) = if tpls.is_empty() {
        (&[], &[])
    } else {
        (tpls[rng.range(tpls.len())], tpls[rng.range(tpls.len())])
    };

    // 3. Build each chord, voice-leading from the previous one
    let mut prev = [0i32; 5];
    let mut prev_n = 0;

    for c in 0..4u32 {
        let d = prog[c as usize] as i32;
        let count = pick_tone_count(rng, cfg.richness);
        let sus = rng.chance(0.07);
        let third = semis_at(cfg.scale, d as usize + 2) - semis_at(cfg.scale, d as usize);
        let seventh = semis_at(cfg.scale, d as usize + 6) - semis_at(cfg.scale, d as usize);
        let minor_major7 = third == 3 && seventh == 11;
        let (tones, tn) = chord_tones(rng, d, count, sus, minor_major7);
        let (semis, m) = voice(scale, rng, &tones[..tn], &prev[..prev_n]);
        prev = semis;
        prev_n = m;

        if cfg.rhythm == ChordRhythm::Pad {
            // One sustained chord for its whole length
            push_hit(out, rng, cfg, &semis[..m], c * bpc * 16, 16 * bpc);
        } else {
            for b in 0..bpc {
                let bar = c * bpc + b;
                let is_last_bar = bar == total_bars - 1;
                let tpl = if is_last_bar && rng.chance(cfg.variation) { alt } else { main };
                for &(s, l) in tpl {
                    push_hit(out, rng, cfg, &semis[..m], bar * 16 + s, l);
                }
            }
        }
    }
}
