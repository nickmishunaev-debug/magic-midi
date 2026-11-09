use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::cmp::Ordering;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;

// ───────────────────────── Music theory ─────────────────────────

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
enum Key {
    #[id = "c"]
    #[name = "C"]
    C,
    #[id = "cs"]
    #[name = "C#"]
    Cs,
    #[id = "d"]
    #[name = "D"]
    D,
    #[id = "ds"]
    #[name = "D#"]
    Ds,
    #[id = "e"]
    #[name = "E"]
    E,
    #[id = "f"]
    #[name = "F"]
    F,
    #[id = "fs"]
    #[name = "F#"]
    Fs,
    #[id = "g"]
    #[name = "G"]
    G,
    #[id = "gs"]
    #[name = "G#"]
    Gs,
    #[id = "a"]
    #[name = "A"]
    A,
    #[id = "as"]
    #[name = "A#"]
    As,
    #[id = "b"]
    #[name = "B"]
    B,
}

impl Key {
    const ALL: [Key; 12] = [
        Key::C,
        Key::Cs,
        Key::D,
        Key::Ds,
        Key::E,
        Key::F,
        Key::Fs,
        Key::G,
        Key::Gs,
        Key::A,
        Key::As,
        Key::B,
    ];

    fn semitone(self) -> i32 {
        self as i32
    }

    fn label(self) -> &'static str {
        ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"][self as usize]
    }
}

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
enum ScaleKind {
    #[id = "major"]
    #[name = "Major"]
    Major,
    #[id = "minor"]
    #[name = "Minor"]
    Minor,
    #[id = "dorian"]
    #[name = "Dorian"]
    Dorian,
    #[id = "phrygian"]
    #[name = "Phrygian"]
    Phrygian,
    #[id = "mixolydian"]
    #[name = "Mixolydian"]
    Mixolydian,
    #[id = "harmonic"]
    #[name = "Harmonic Minor"]
    HarmonicMinor,
    #[id = "minpent"]
    #[name = "Minor Pentatonic"]
    MinorPentatonic,
}

impl ScaleKind {
    const ALL: [ScaleKind; 7] = [
        ScaleKind::Major,
        ScaleKind::Minor,
        ScaleKind::Dorian,
        ScaleKind::Phrygian,
        ScaleKind::Mixolydian,
        ScaleKind::HarmonicMinor,
        ScaleKind::MinorPentatonic,
    ];

    fn label(self) -> &'static str {
        match self {
            ScaleKind::Major => "Major",
            ScaleKind::Minor => "Minor",
            ScaleKind::Dorian => "Dorian",
            ScaleKind::Phrygian => "Phrygian",
            ScaleKind::Mixolydian => "Mixolydian",
            ScaleKind::HarmonicMinor => "Harmonic Minor",
            ScaleKind::MinorPentatonic => "Minor Pentatonic",
        }
    }

    fn intervals(self) -> &'static [i32] {
        match self {
            ScaleKind::Major => &[0, 2, 4, 5, 7, 9, 11],
            ScaleKind::Minor => &[0, 2, 3, 5, 7, 8, 10],
            ScaleKind::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            ScaleKind::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            ScaleKind::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            ScaleKind::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            ScaleKind::MinorPentatonic => &[0, 3, 5, 7, 10],
        }
    }

    /// Scale-degree indices that sound "at rest" (the tonic triad).
    fn stable(self) -> &'static [usize] {
        match self {
            ScaleKind::MinorPentatonic => &[0, 1, 3],
            _ => &[0, 2, 4],
        }
    }

    /// Degrees that sound unfinished, used to end a "question" phrase.
    fn open(self) -> &'static [usize] {
        match self {
            ScaleKind::MinorPentatonic => &[2, 4],
            _ => &[1, 4, 6],
        }
    }

    /// `pos` is a scale position: 0 = tonic, 1 = next scale degree, len = tonic one octave up.
    fn midi(self, tonic: i32, pos: i32) -> i32 {
        let iv = self.intervals();
        let len = iv.len() as i32;
        tonic + 12 * pos.div_euclid(len) + iv[pos.rem_euclid(len) as usize]
    }
}

// ───────────────────────── Moods ─────────────────────────

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
enum Mood {
    #[id = "dreamy"]
    #[name = "Dreamy"]
    Dreamy,
    #[id = "uplifting"]
    #[name = "Uplifting"]
    Uplifting,
    #[id = "dark"]
    #[name = "Dark"]
    Dark,
    #[id = "groovy"]
    #[name = "Groovy"]
    Groovy,
    #[id = "hypnotic"]
    #[name = "Hypnotic"]
    Hypnotic,
    #[id = "emotional"]
    #[name = "Emotional"]
    Emotional,
}

/// Everything technical that a mood decides, so the user never has to.
#[derive(Clone, Copy)]
struct MoodSettings {
    bars: u32,       // melody length before it loops
    density: f32,    // 0..1, how busy the rhythm is
    gate: f64,       // fraction of each note's slot that is held
    velocity: i32,   // base loudness
    octave: i32,     // register shift
    up_bias: f32,    // >0 leans upward, <0 leans downward
    leapiness: f32,  // how often it jumps instead of stepping
    repeat: bool,    // repeat the hook exactly (riff-style)
}

impl Mood {
    const ALL: [Mood; 6] = [
        Mood::Dreamy,
        Mood::Uplifting,
        Mood::Dark,
        Mood::Groovy,
        Mood::Hypnotic,
        Mood::Emotional,
    ];

    fn label(self) -> &'static str {
        match self {
            Mood::Dreamy => "Dreamy",
            Mood::Uplifting => "Uplifting",
            Mood::Dark => "Dark",
            Mood::Groovy => "Groovy",
            Mood::Hypnotic => "Hypnotic",
            Mood::Emotional => "Emotional",
        }
    }

    fn tagline(self) -> &'static str {
        match self {
            Mood::Dreamy => "Sparse and floating, with long soft notes",
            Mood::Uplifting => "Bright and rising, with bigger melodic jumps",
            Mood::Dark => "Low, tense and sparse",
            Mood::Groovy => "Busy, syncopated and bouncy",
            Mood::Hypnotic => "A short riff that locks in and repeats",
            Mood::Emotional => "Slow, falling lines with long, aching notes",
        }
    }

    fn settings(self) -> MoodSettings {
        match self {
            Mood::Dreamy => MoodSettings {
                bars: 4,
                density: 0.1,
                gate: 0.95,
                velocity: 80,
                octave: 0,
                up_bias: 0.05,
                leapiness: 0.8,
                repeat: false,
            },
            Mood::Uplifting => MoodSettings {
                bars: 4,
                density: 0.6,
                gate: 0.75,
                velocity: 105,
                octave: 0,
                up_bias: 0.12,
                leapiness: 1.4,
                repeat: false,
            },
            Mood::Dark => MoodSettings {
                bars: 2,
                density: 0.3,
                gate: 0.6,
                velocity: 95,
                octave: -1,
                up_bias: -0.08,
                leapiness: 0.7,
                repeat: false,
            },
            Mood::Groovy => MoodSettings {
                bars: 2,
                density: 0.85,
                gate: 0.45,
                velocity: 108,
                octave: -1,
                up_bias: 0.0,
                leapiness: 1.0,
                repeat: false,
            },
            Mood::Hypnotic => MoodSettings {
                bars: 4,
                density: 0.4,
                gate: 0.7,
                velocity: 90,
                octave: -1,
                up_bias: 0.0,
                leapiness: 0.4,
                repeat: true,
            },
            Mood::Emotional => MoodSettings {
                bars: 4,
                density: 0.15,
                gate: 1.0,
                velocity: 85,
                octave: 0,
                up_bias: -0.1,
                leapiness: 0.6,
                repeat: false,
            },
        }
    }
}

// ───────────────────────── Tiny RNG ─────────────────────────

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03);
        r.next();
        r.next();
        r
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform float in [0, 1)
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / 16_777_216.0
    }

    fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }

    /// Uniform integer in [0, n). `n` must be > 0.
    fn range(&mut self, n: usize) -> usize {
        ((self.next() >> 11) % n as u64) as usize
    }
}

// ───────────────────────── Melody generator ─────────────────────────

/// One note of a generated phrase. Times are in 16th-note steps from the phrase start.
#[derive(Clone, Copy)]
struct PNote {
    step: u32,
    len: u32,
    pos: i32,     // scale position (see ScaleKind::midi)
    vel_off: i32, // accent relative to the mood's base velocity
}

#[derive(Clone, Copy)]
struct GenCfg {
    scale: ScaleKind,
    bars: u32,
    density: f32,
    up_bias: f32,
    leapiness: f32,
    repeat: bool,
}

/// One-bar rhythms as (start step, length in steps), 16 steps per bar.
/// Hand-picked for a deep-house feel: syncopation, dotted figures, and room to breathe.
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

/// Builds one melody of `cfg.bars` bars.
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
fn generate_phrase(cfg: &GenCfg, rng: &mut Rng, motif: &mut Vec<PNote>, out: &mut Vec<PNote>) {
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

// ───────────────────────── Plugin ─────────────────────────

#[derive(Params)]
struct MidiGenParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "key"]
    key: EnumParam<Key>,
    #[id = "scale"]
    scale: EnumParam<ScaleKind>,
    #[id = "mood"]
    mood: EnumParam<Mood>,
}

impl Default for MidiGenParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(340, 340),
            key: EnumParam::new("Key", Key::A),
            scale: EnumParam::new("Scale", ScaleKind::Minor),
            mood: EnumParam::new("Mood", Mood::Groovy),
        }
    }
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

/// Everything goes out on MIDI channel 1; the DAW's track routing decides where it lands.
const MIDI_CHANNEL: u8 = 0;

struct MidiGen {
    params: Arc<MidiGenParams>,

    rng: Rng,

    phrase: Vec<PNote>,
    motif: Vec<PNote>,
    phrase_valid: bool,
    phrase_bars: u32,
    phrase_scale: ScaleKind,
    phrase_mood: Mood,

    active: Vec<ActiveNote>,
    events: Vec<Ev>,

    was_playing: bool,
    beat_pos: f64,

    /// Set by the Regenerate button, consumed by the audio thread
    regen: Arc<AtomicBool>,
}

impl Default for MidiGen {
    fn default() -> Self {
        // A different melody every time the plugin is added
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);

        Self {
            params: Arc::new(MidiGenParams::default()),
            rng: Rng::new(seed),
            phrase: Vec::with_capacity(256),
            motif: Vec::with_capacity(64),
            phrase_valid: false,
            phrase_bars: 0,
            phrase_scale: ScaleKind::Minor,
            phrase_mood: Mood::Groovy,
            active: Vec::with_capacity(64),
            events: Vec::with_capacity(256),
            was_playing: false,
            beat_pos: 0.0,
            regen: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn enum_combo<E>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    label: &str,
    param: &EnumParam<E>,
    all: &[E],
    name: fn(E) -> &'static str,
) where
    E: Enum + PartialEq + Copy + 'static,
{
    let cur = param.value();
    egui::ComboBox::from_label(label)
        .selected_text(name(cur))
        .show_ui(ui, |ui| {
            for &v in all {
                if ui.selectable_label(v == cur, name(v)).clicked() {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, v);
                    setter.end_set_parameter(param);
                }
            }
        });
}

impl Plugin for MidiGen {
    const NAME: &'static str = "MIDI Gen";
    const VENDOR: &'static str = "Nik";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    // Live requires an event input bus on instrument-category plugins
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let regen = self.regen.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.heading("MIDI Gen");
                    ui.add_space(8.0);

                    enum_combo(ui, setter, "Key", &params.key, &Key::ALL, Key::label);
                    enum_combo(ui, setter, "Scale", &params.scale, &ScaleKind::ALL, ScaleKind::label);
                    ui.add_space(10.0);

                    ui.label("Mood");
                    let cur = params.mood.value();
                    ui.horizontal_wrapped(|ui| {
                        for &m in &Mood::ALL {
                            if ui.selectable_label(m == cur, m.label()).clicked() {
                                setter.begin_set_parameter(&params.mood);
                                setter.set_parameter(&params.mood, m);
                                setter.end_set_parameter(&params.mood);
                            }
                        }
                    });
                    ui.small(cur.tagline());
                    ui.add_space(14.0);

                    if ui
                        .add_sized([ui.available_width(), 40.0], egui::Button::new("Regenerate"))
                        .clicked()
                    {
                        regen.store(true, AtomicOrdering::Relaxed);
                    }
                });
            },
        )
    }

    fn reset(&mut self) {
        // Keep the current melody; only clear playback state
        self.active.clear();
        self.was_playing = false;
        self.beat_pos = 0.0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // We only emit MIDI, so keep the audio output silent
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                *sample = 0.0;
            }
        }

        let (playing, tempo, sample_rate, host_pos) = {
            let t = context.transport();
            (t.playing, t.tempo.unwrap_or(120.0), t.sample_rate as f64, t.pos_beats())
        };

        let num_samples = buffer.samples();
        if num_samples == 0 {
            return ProcessStatus::Normal;
        }

        if !playing {
            for a in self.active.drain(..) {
                context.send_event(NoteEvent::NoteOff {
                    timing: 0,
                    voice_id: None,
                    channel: MIDI_CHANNEL,
                    note: a.note,
                    velocity: 0.0,
                });
            }
            self.was_playing = false;
            return ProcessStatus::Normal;
        }

        let beats_per_sample = tempo / 60.0 / sample_rate;

        // Follow our own running position so consecutive blocks never overlap or leave gaps;
        // re-sync to the host if we drift, and detect jumps/loops.
        let starting = !self.was_playing;
        let (block_start, jumped) = if starting {
            (host_pos.unwrap_or(0.0), false)
        } else if let Some(h) = host_pos {
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

        // Snapshot settings; the mood decides all the technical details
        let mood = self.params.mood.value();
        let ms = mood.settings();
        let cfg = GenCfg {
            scale: self.params.scale.value(),
            bars: ms.bars,
            density: ms.density,
            up_bias: ms.up_bias,
            leapiness: ms.leapiness,
            repeat: ms.repeat,
        };
        let tonic = 60 + self.params.key.value().semitone() + 12 * ms.octave;

        // The melody is only replaced when Regenerate is pressed, or when the mood or scale
        // changes (the old melody can't be mapped onto a different scale or character).
        // Changing the key just transposes the current melody.
        let regen = self.regen.swap(false, AtomicOrdering::Relaxed);
        if regen
            || self.phrase_bars != cfg.bars
            || self.phrase_scale != cfg.scale
            || self.phrase_mood != mood
        {
            self.phrase_valid = false;
        }

        if starting || jumped {
            for a in self.active.drain(..) {
                context.send_event(NoteEvent::NoteOff {
                    timing: 0,
                    voice_id: None,
                    channel: MIDI_CHANNEL,
                    note: a.note,
                    velocity: 0.0,
                });
            }
        }

        // Schedule events for every loop of the melody this block touches
        self.events.clear();
        let phrase_beats = cfg.bars as f64 * 4.0; // assumes 4/4
        let first_slot = (block_start / phrase_beats).floor() as i64;
        let last_slot = (((block_end - 1e-9) / phrase_beats).floor() as i64).max(first_slot);

        for slot in first_slot..=last_slot {
            if !self.phrase_valid {
                generate_phrase(&cfg, &mut self.rng, &mut self.motif, &mut self.phrase);
                self.phrase_valid = true;
                self.phrase_bars = cfg.bars;
                self.phrase_scale = cfg.scale;
                self.phrase_mood = mood;
            }

            let slot_start = slot as f64 * phrase_beats;

            for n in self.phrase.iter() {
                let start = slot_start + n.step as f64 * 0.25;
                if start < block_start || start >= block_end {
                    continue;
                }
                let midi = cfg.scale.midi(tonic, n.pos).clamp(0, 127) as u8;
                let vel = (ms.velocity + n.vel_off).clamp(1, 127) as f32 / 127.0;
                let len_beats = (n.len as f64 * 0.25 * ms.gate).max(0.05);

                self.events.push(Ev {
                    beat: start,
                    on: true,
                    note: midi,
                    velocity: vel,
                });
                self.active.push(ActiveNote {
                    note: midi,
                    off_beat: start + len_beats,
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

        // Send in time order; at equal times, note-offs go before note-ons
        self.events.sort_unstable_by(|a, b| {
            a.beat
                .partial_cmp(&b.beat)
                .unwrap_or(Ordering::Equal)
                .then(a.on.cmp(&b.on))
        });

        for e in self.events.iter() {
            let timing = (((e.beat - block_start) / beats_per_sample).round().max(0.0) as u32)
                .min(num_samples as u32 - 1);
            if e.on {
                context.send_event(NoteEvent::NoteOn {
                    timing,
                    voice_id: None,
                    channel: MIDI_CHANNEL,
                    note: e.note,
                    velocity: e.velocity,
                });
            } else {
                context.send_event(NoteEvent::NoteOff {
                    timing,
                    voice_id: None,
                    channel: MIDI_CHANNEL,
                    note: e.note,
                    velocity: 0.0,
                });
            }
        }

        self.beat_pos = block_end;
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for MidiGen {
    const VST3_CLASS_ID: [u8; 16] = *b"NikMidiGenPlugin";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument];
}

nih_export_vst3!(MidiGen);