//! Session settings: one value per item, changed from the settings window or by shortcut.
use super::*;
use cat_smf_editor::score::pattern::recording::{
    Layout, Mode, Modulation, Range, Scale, Start, Velocity,
};

const BPM: std::ops::RangeInclusive<i32> = 20..=300;
const MEASURES: std::ops::RangeInclusive<i32> = 1..=64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Item {
    Sound,
    Range,
    Start,
    Measures,
    Bpm,
    Layout,
    Mode,
    Tonic,
    Scale,
    NoteBase,
    ChordBase,
    Velocity,
    Modulation,
    Quantize,
}
pub(super) const ITEMS: [Item; 14] = [
    Item::Sound,
    Item::Range,
    Item::Start,
    Item::Measures,
    Item::Bpm,
    Item::Layout,
    Item::Mode,
    Item::Tonic,
    Item::Scale,
    Item::NoteBase,
    Item::ChordBase,
    Item::Velocity,
    Item::Modulation,
    Item::Quantize,
];

/// Ctrl+key steps forward and Ctrl+Shift+key back; plain keys are pads.
const SHORTCUTS: [(Item, Key); 13] = [
    (Item::Sound, Key::P),
    (Item::Range, Key::R),
    (Item::Start, Key::S),
    (Item::Measures, Key::D),
    (Item::Bpm, Key::B),
    (Item::Layout, Key::L),
    (Item::Tonic, Key::K),
    (Item::Scale, Key::M),
    (Item::NoteBase, Key::N),
    (Item::ChordBase, Key::H),
    (Item::Velocity, Key::V),
    (Item::Modulation, Key::W),
    (Item::Quantize, Key::Q),
];

impl Item {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Sound => "Container",
            Self::Range => "録音範囲",
            Self::Start => "録音開始",
            Self::Measures => "小節数",
            Self::Bpm => "BPM",
            Self::Layout => "配列",
            Self::Mode => "Mode",
            Self::Tonic => "Tonic",
            Self::Scale => "Scale",
            Self::NoteBase => "Note base",
            Self::ChordBase => "Chord base",
            Self::Velocity => "Velocity",
            Self::Modulation => "Modulation",
            Self::Quantize => "Quantize",
        }
    }
    /// Two-valued items toggle; the others also step back with Shift.
    pub(super) fn keys(self) -> String {
        if self == Self::Mode {
            return "Tab / Shift+Tab".into();
        }
        let key = SHORTCUTS.iter().find(|(i, _)| *i == self).unwrap().1;
        let name = key.name();
        match self {
            Self::Sound
            | Self::Measures
            | Self::Bpm
            | Self::Tonic
            | Self::NoteBase
            | Self::ChordBase
            | Self::Velocity
            | Self::Modulation => {
                format!("Ctrl+{name} / Ctrl+Shift+{name}")
            }
            _ => format!("Ctrl+{name}"),
        }
    }
}

/// Microseconds per quarter note at `bpm`.
pub(super) fn tempo(bpm: i32) -> u32 {
    (60_000_000.0 / f64::from(bpm.clamp(*BPM.start(), *BPM.end()))).round() as u32
}
pub(super) fn bpm(tempo: u32) -> i32 {
    (60_000_000.0 / f64::from(tempo)).round() as i32
}

/// Rows that do not apply to the current choices or a drum container, or that a started take
/// fixes, are dimmed and skipped.
pub(super) fn enabled(s: &Session, item: Item) -> bool {
    let t = s.settings();
    let rows = t.layout == Layout::Rows;
    let pitched = rows && !t.drums;
    let fixed = play::armed(s);
    match item {
        Item::Sound => s.sounds.len() > 1 && !fixed,
        Item::Measures => matches!(t.range, Range::Loop { .. }) && !fixed,
        // A take writes CC1 from one choice.
        Item::Range | Item::Start | Item::Bpm | Item::Modulation => !fixed,
        Item::Mode | Item::Tonic | Item::Scale => pitched,
        Item::ChordBase => pitched,
        Item::NoteBase => !t.drums,
        Item::Layout | Item::Velocity | Item::Quantize => true,
    }
}

pub(super) fn first(s: &Session) -> Item {
    ITEMS.into_iter().find(|&i| enabled(s, i)).unwrap()
}

pub(super) fn step_item(s: &Session, down: bool) -> Item {
    let index = ITEMS.iter().position(|&i| i == s.item).unwrap_or(0);
    let mut candidates: Vec<_> = if down {
        ITEMS[index + 1..].to_vec()
    } else {
        ITEMS[..index].iter().rev().copied().collect()
    };
    candidates.retain(|&i| enabled(s, i));
    candidates.first().copied().unwrap_or(s.item)
}

pub(super) fn value(s: &Session, item: Item) -> String {
    let t = s.settings();
    match item {
        Item::Sound => s.sounds[s.selected].name.clone(),
        Item::Range => match t.range {
            Range::Free => "Free".into(),
            Range::Loop { .. } => "Loop".into(),
        },
        Item::Start => match t.start {
            Start::Button => "Rec".into(),
            Start::Key => "Key".into(),
        },
        Item::Measures => s.measures.to_string(),
        Item::Bpm => bpm(t.tempo).to_string(),
        Item::Layout => match t.layout {
            Layout::Rows => "z a q 1".into(),
            Layout::Letters => "c d e f g a b".into(),
        },
        Item::Mode => t.mode.label().into(),
        Item::Tonic => window::pitch_class(t.key().tonic).into(),
        Item::Scale => match t.key().scale {
            Scale::MajorPentatonic => "Major pentatonic".into(),
            Scale::MinorPentatonic => "Minor pentatonic".into(),
        },
        Item::NoteBase => cat_smf_editor::score::note_view::note_name(t.note_base),
        Item::ChordBase => cat_smf_editor::score::note_view::note_name(t.chord_base),
        Item::Velocity => t.velocity.label().into(),
        Item::Modulation => t.modulation.label(),
        Item::Quantize => if t.quantize { "1/16" } else { "Off" }.into(),
    }
}

/// The choice `step` places from `current` in `all`, wrapping around.
fn cycle<T: Copy + PartialEq>(all: &[T], current: T, step: i32) -> T {
    let i = all.iter().position(|&v| v == current).unwrap_or(0) as i32;
    all[(i + step).rem_euclid(all.len() as i32) as usize]
}

/// The nearest pitch from `base` by `step` octaves (`big`) or scale notes.
fn neighbor(base: u8, tonic: u8, steps: &[u8], step: i32, big: bool) -> u8 {
    if big {
        return u8::try_from(i32::from(base) + 12 * step)
            .ok()
            .filter(|&p| p < 128)
            .unwrap_or(base);
    }
    let mut p = i32::from(base);
    loop {
        p += step;
        if !(0..128).contains(&p) {
            return base;
        }
        if steps.contains(&((p - i32::from(tonic)).rem_euclid(12) as u8)) {
            return p as u8;
        }
    }
}

/// Changes an enabled item; another container reopens the instrument.
pub(super) fn change(a: &mut App, s: &mut Session, item: Item, step: i32, big: bool) {
    if !enabled(s, item) {
        return;
    }
    match item {
        Item::Sound => {
            let n = s.sounds.len() as i32;
            select(a, s, (s.selected as i32 + step).rem_euclid(n) as usize);
        }
        Item::Measures => {
            s.measures = (i32::from(s.measures) + step * if big { 4 } else { 1 })
                .clamp(*MEASURES.start(), *MEASURES.end()) as u16;
            let measures = s.measures;
            s.settings_mut().range = Range::Loop { measures };
        }
        Item::Range => {
            let measures = s.measures;
            let t = s.settings_mut();
            t.range = match t.range {
                Range::Free => Range::Loop { measures },
                Range::Loop { .. } => Range::Free,
            }
        }
        Item::Modulation => {
            // The keys captured so far keep the CC1 they were played with.
            if let Err(error) = super::capture::flush(a, s, true) {
                s.error = Some(error);
            }
            let t = s.settings_mut();
            t.modulation = cycle(&Modulation::ALL, t.modulation, step);
        }
        _ => edit(s.settings_mut(), item, step, big),
    }
    if !enabled(s, s.item) {
        s.item = first(s);
    }
}

/// Plays another container while the phrases go on; one that cannot play opens the settings
/// beside the error.
pub(super) fn select(a: &mut App, s: &mut Session, index: usize) {
    play::leave(s);
    keep_voicing(a, s);
    s.selected = index;
    if let Some(v) = s
        .sound()
        .container
        .and_then(|id| a.config.keyboard_voicings.get(&id))
    {
        s.settings.set_voicing(*v);
    }
    if !play::start(a, s) {
        s.settings_open = true;
    }
    if !enabled(s, s.item) {
        s.item = first(s);
    }
    super::phrase::reveal(a, s);
}

/// Remembers what the pads of the playing container play, for its next selection.
pub(super) fn keep_voicing(a: &mut App, s: &Session) {
    if let Some(id) = s.sound().container {
        a.config.select_keyboard_voicing(id, s.settings().voicing());
    }
}

fn edit(t: &mut Settings, item: Item, step: i32, big: bool) {
    let key = t.key();
    match item {
        Item::Sound | Item::Measures | Item::Range | Item::Modulation => unreachable!(),
        Item::Velocity => t.velocity = cycle(&Velocity::ALL, t.velocity, step),
        Item::Quantize => t.quantize = !t.quantize,
        Item::Start => {
            t.start = match t.start {
                Start::Button => Start::Key,
                Start::Key => Start::Button,
            }
        }
        Item::Bpm => t.tempo = tempo(bpm(t.tempo) + step * if big { 10 } else { 1 }),
        Item::Layout => {
            t.layout = match t.layout {
                Layout::Rows => Layout::Letters,
                Layout::Letters => Layout::Rows,
            }
        }
        Item::Mode => {
            let i = Mode::ALL.iter().position(|&m| m == t.mode).unwrap() as i32;
            t.mode = Mode::ALL[(i + step).rem_euclid(Mode::ALL.len() as i32) as usize];
        }
        Item::Tonic => t.set_key(recording::Key {
            tonic: (i32::from(key.tonic) + step).rem_euclid(12) as u8,
            ..key
        }),
        Item::Scale => t.set_key(recording::Key {
            scale: match key.scale {
                Scale::MajorPentatonic => Scale::MinorPentatonic,
                Scale::MinorPentatonic => Scale::MajorPentatonic,
            },
            ..key
        }),
        Item::NoteBase => {
            // Letters only use the octave of the base.
            let big = big || t.layout == Layout::Letters;
            t.note_base = neighbor(t.note_base, key.tonic, key.scale.steps(), step, big)
        }
        Item::ChordBase => {
            t.chord_base = neighbor(t.chord_base, key.tonic, key.scale.diatonic(), step, big)
        }
    }
}

/// Applies the item shortcut of a pressed key; `false` when the key is not one.
pub(super) fn shortcut(a: &mut App, s: &mut Session, key: Key, modifiers: Modifiers) -> bool {
    if key == Key::Tab {
        let step = if commands::matches(modifiers, Modifiers::NONE) {
            1
        } else if commands::matches(modifiers, Modifiers::SHIFT) {
            -1
        } else {
            return false;
        };
        change(a, s, Item::Mode, step, false);
        return true;
    }
    let step = if commands::matches(modifiers, Modifiers::CTRL) {
        1
    } else if commands::matches(modifiers, Modifiers::CTRL | Modifiers::SHIFT) {
        -1
    } else {
        return false;
    };
    let Some(&(item, _)) = SHORTCUTS.iter().find(|(_, k)| *k == key) else {
        return false;
    };
    change(a, s, item, step, false);
    true
}
