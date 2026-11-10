//! Melody generation: rhythm templates, the melodic walk, and phrase architecture.
//!
//! This module knows nothing about plugins or MIDI. It only turns settings into a list of notes.

use crate::mood::MoodSettings;
use crate::rng::Rng;
use crate::theory::ScaleKind;

/// One note of a generated phrase. Times are in 16th-note steps from the phrase start.
#[derive(Clone, Copy)]
pub struct PNote {
    pub step: u32,
    pub len: u32,
    /// Scale position (see `ScaleKind::midi`)
    pub pos: i32,
    /// Accent relative to the mood's base velocity
    pub vel_off: i32,
}

#[derive(Clone, Copy)]
pub struct GenCfg {
    pub scale: ScaleKind,
    pub bars: u32,
    pub density: f32,
    pub up_bias: f32,
    pub leapiness: f32,
    pub repeat: bool,
}

impl GenCfg {
    pub fn new(scale: ScaleKind, ms: &MoodSettings) -> Self {
        Self {
            scale,
            bars: ms.bars,
            density: ms.density,
            up_bias: ms.up_bias,
            leapiness: ms.leapiness,
            repeat: ms.repeat,
        }
    }
}

/// One-bar rhythms as (start step, length in steps), 16 steps per bar.
/// Hand-picked for a deep-house feel: syncopation, dotted figures, and room to breathe.
/// To add more variety, just add entries here.
const RHYTHMS: &[&[(u32, u32)]] = &[
    &[(0, 3), (3, 3), (6, 2), (8, 4)],
    &[(0, 2), (3, 1), (4, 4), (10, 2), (12, 4)],
    &[(2, 2), (4, 2), (7, 3), (10, 2), (14, 2)],
    &[(0, 6), (6, 2), (8, 6)],
    &[(0, 3), (3, 3), (6, 3), (9, 3), (12, 4)],
    &[(3, 2), (6, 2), (8, 2), (11, 2), (14, 2)],
    &[(0, 1), (1, 1), (2, 2), (4, 4), (8, 3), (11, 5)],
    &[(0, 2), (2, 2), (4, 2), (6, 2), (8, 2), (10, 2), (12, 2), (14, 2)],
    &[(0, 4), (6, 2), (10, 6)],
    &[(0, 3), (4, 2), (6, 2), (10, 2), (12, 4)],
    &[(2, 1), (3, 1), (4, 4), (8, 2), (11, 1), (12, 4)],
];

fn rhythm_weight(t: &[(u32, u32)], target: f32) -> f32 {
    let d = t.len() as f32 - target;
    1.0 / (1.0 + d * d)
}

fn pick_rhythm(rng: &mut Rng, density: f32) -> &'static [(u32, u32)] {
    let target = 3.0 + density * 4.0; // 3 notes per bar (sparse) .. 7 (busy)
    let mut total = 0.0;
    for &t in RHYTHMS.iter() {
        total += rhythm_weight(t, target);
    }
    let mut r = rng.f() * total;
    for &t in RHYTHMS.iter() {
        let w = rhythm_weight(t, target);
        if r < w {
            return t;
        }
        r -= w;
    }
    RHYTHMS[0]
}

fn reflect(x: i32, lo: i32, hi: i32) -> i32 {
    let mut v = x;
    if v < lo {
        v = lo + (lo - v);
    }
    if v > hi {
        v = hi - (v - hi);
    }
    v.clamp(lo, hi)
}

/// Nearest scale position (within range) whose degree is one of `idxs`.
fn nearest_in(pos: i32, len: i32, lo: i32, hi: i32, idxs: &[usize]) -> i32 {
    for d in 0..=len {
        for &cand in &[pos - d, pos + d] {
            if cand >= lo && cand <= hi && idxs.contains(&(cand.rem_euclid(len) as usize)) {
                return cand;
            }
        }
    }
    pos
}

/// Next pitch in a melodic walk: mostly steps, some thirds, rare leaps (more with a high
/// `leapiness`). A leap is answered by a step back the other way. The contour is pulled toward
/// the centre of the range and nudged up or down by the mood's `up_bias`.
fn next_pos(
    rng: &mut Rng,
    cfg: &GenCfg,
    prev: i32,
    leap_dir: i32,
    lo: i32,
    hi: i32,
    centre: i32,
) -> (i32, i32) {
    let l = cfg.leapiness;
    let w = [0.14, 0.42, 0.28 * l, 0.11 * l, 0.05 * l];
    let total: f32 = w.iter().sum();
    let mut r = rng.f() * total;
    let mut size = 4;
    for (i, &wi) in w.iter().enumerate() {
        if r < wi {
            size = i as i32;
            break;
        }
        r -= wi;
    }

    let p_up = (0.5 + cfg.up_bias + (centre - prev) as f32 * 0.08).clamp(0.15, 0.85);
    let mut dir = if rng.chance(p_up) { 1 } else { -1 };
    if leap_dir != 0 && rng.chance(0.85) {
        dir = -leap_dir;
        size = 1;
    }
    let next = reflect(prev + dir * size, lo, hi);
    let new_leap = if size >= 3 { dir } else { 0 };
    (next, new_leap)
}

/// Velocity accent: downbeat strongest, beats medium, off-16ths softer, plus a little human wobble.
fn accent(step: u32, rng: &mut Rng) -> i32 {
    let base = if step % 16 == 0 {
        10
    } else if step % 4 == 0 {
        4
    } else if step % 2 == 1 {
        -8
    } else {
        0
    };
    base + (rng.range(9) as i32 - 4)
}

/// Builds one melody of `cfg.bars` bars into `out`. `motif` is scratch space.
/// Both vectors are cleared first and reused, so nothing is allocated once they have capacity.
///
/// Phrase architecture:
///   Bar 1          HOOK / CALL: a short motif (one rhythm + contour) that ends on an "open" degree.
///   Odd bars       SEQUENCE: the same motif shifted up/down a few scale steps (sometimes inverted),
///                  with an occasional dropped note. Still ends open, so tension builds.
///   Even bars      HOOK RETURNS: the motif repeated exactly, so the ear locks onto it.
///   Last bar       RESPONSE / ANSWER: the motif varied (shifted or inverted), landing on the tonic
///                  with a longer held note, which resolves the question asked in bar 1.
/// In riff-style moods the hook repeats exactly in every bar except the last.
/// Strong beats lean toward tonic-triad notes so everything sounds anchored in the key.
pub fn generate_phrase(cfg: &GenCfg, rng: &mut Rng, motif: &mut Vec<PNote>, out: &mut Vec<PNote>) {
    out.clear();
    motif.clear();

    let sc = cfg.scale;
    let len = sc.intervals().len() as i32;
    let lo = -(len / 2);
    let hi = len + len / 2;
    let centre = (lo + hi) / 2;
    let stable = sc.stable();
    let open = sc.open();

    // 1. Hook rhythm and contour
    let rhythm = pick_rhythm(rng, cfg.density);
    let mut pos = stable[rng.range(stable.len())] as i32;
    let mut leap_dir = 0;
    for (i, &(step, l)) in rhythm.iter().enumerate() {
        if i > 0 {
            let (p, ld) = next_pos(rng, cfg, pos, leap_dir, lo, hi, centre);
            pos = p;
            leap_dir = ld;
            if step % 4 == 0 && rng.chance(0.75) {
                pos = nearest_in(pos, len, lo, hi, stable);
            }
        }
        let vel_off = accent(step, rng);
        motif.push(PNote { step, len: l, pos, vel_off });
    }

    // The hook ends open (a question), unless it's a one-bar melody, which resolves home.
    let last = motif.len() - 1;
    let prev_pos = motif[last].pos;
    motif[last].pos = if cfg.bars == 1 {
        nearest_in(prev_pos, len, lo, hi, &[0])
    } else {
        nearest_in(prev_pos, len, lo, hi, open)
    };

    // 2. Lay out the bars
    for b in 0..cfg.bars {
        let base = b * 16;

        if b == 0 {
            for m in motif.iter() {
                out.push(PNote { step: base + m.step, ..*m });
            }
            continue;
        }

        let is_last = b == cfg.bars - 1;
        let exact = !is_last && (cfg.repeat || b % 2 == 0); // the hook comes back unchanged

        let (shift, invert) = if exact {
            (0, false)
        } else if is_last {
            ([0, -1, -2, 1][rng.range(4)], rng.chance(0.4))
        } else {
            ([-2, -1, 1, 2][rng.range(4)], rng.chance(0.2))
        };

        let n = motif.len();
        let drop_idx = if !exact && n >= 5 && rng.chance(0.3) {
            Some(1 + rng.range(n - 2))
        } else {
            None
        };

        let mut p = (motif[0].pos + shift).clamp(lo, hi);
        for (i, m) in motif.iter().enumerate() {
            if i > 0 {
                let iv = m.pos - motif[i - 1].pos;
                p = reflect(p + if invert { -iv } else { iv }, lo, hi);
            }
            if Some(i) == drop_idx {
                continue;
            }
            let mut q = p;
            if !exact && m.step % 4 == 0 && rng.chance(0.6) {
                q = nearest_in(q, len, lo, hi, stable);
            }
            out.push(PNote {
                step: base + m.step,
                len: m.len,
                pos: q,
                vel_off: m.vel_off,
            });
        }

        if let Some(l) = out.last_mut() {
            if is_last {
                // Answer: land on the tonic and hold it
                l.pos = nearest_in(l.pos, len, lo, hi, &[0]);
                let local = l.step - base;
                l.len = (16 - local).min(8).max(l.len);
            } else if !exact && rng.chance(0.7) {
                l.pos = nearest_in(l.pos, len, lo, hi, open);
            }
        }
    }
}
