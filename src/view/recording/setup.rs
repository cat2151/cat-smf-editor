//! Session settings: one value per item, changed from the settings window or by shortcut.
use super::*;
use cat_smf_editor::score::pattern::recording::{Mode, Modulation, Range, Scale, Velocity};

const BPM: std::ops::RangeInclusive<i32> = 20..=300;
const MEASURES: std::ops::RangeInclusive<i32> = 1..=64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Item {
    Sound,
    Range,
    Measures,
    Bpm,
    Mode,
    Tonic,
    Scale,
    NoteBase,
    ChordBase,
    Velocity,
    Modulation,
    Quantize,
}
/// In the order of the rows, each group together; the container, velocity and modulation are
/// chosen on the keyboard window.
pub(super) const ITEMS: [Item; 9] = [
    Item::Range,
    Item::Measures,
    Item::Bpm,
    Item::Quantize,
    Item::Mode,
    Item::Tonic,
    Item::Scale,
    Item::NoteBase,
    Item::ChordBase,
];

/// What a row sets: the take or the pitches of the pads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Group {
    Take,
    Pitch,
}

/// Ctrl+key steps forward and Ctrl+Shift+key back; plain keys are pads.
const SHORTCUTS: [(Item, Key); 11] = [
    (Item::Sound, Key::P),
    (Item::Range, Key::R),
    (Item::Measures, Key::D),
    (Item::Bpm, Key::B),
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
            Self::Measures => "小節数",
            Self::Bpm => "BPM",
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
    pub(super) fn group(self) -> Group {
        match self {
            Self::Sound | Self::Velocity | Self::Modulation => unreachable!("not a row"),
            Self::Range | Self::Measures | Self::Bpm | Self::Quantize => Group::Take,
            Self::Mode | Self::Tonic | Self::Scale | Self::NoteBase | Self::ChordBase => {
                Group::Pitch
            }
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

/// Only the base of the pads the mode plays has a row, and the loop length only in Loop.
pub(super) fn shown(s: &Session, item: Item) -> bool {
    let t = s.settings();
    let chords = t.mode.chord_notes().is_some();
    match item {
        Item::Measures => matches!(t.range, Range::Loop { .. }),
        Item::NoteBase => !chords,
        Item::ChordBase => chords,
        _ => true,
    }
}

/// Rows that do not apply to the current choices or a drum container, or that a started take
/// fixes, are dimmed and skipped.
pub(super) fn enabled(s: &Session, item: Item) -> bool {
    let t = s.settings();
    let pitched = t.mode != Mode::Letters && !t.drums;
    let fixed = play::armed(s);
    shown(s, item)
        && match item {
            Item::Sound => s.sounds.len() > 1 && !fixed,
            // A take writes CC1 from one choice.
            Item::Range | Item::Measures | Item::Bpm | Item::Modulation => !fixed,
            Item::Tonic | Item::Scale | Item::ChordBase => pitched,
            Item::NoteBase => !t.drums,
            Item::Mode | Item::Velocity | Item::Quantize => true,
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
        Item::Sound | Item::Velocity | Item::Modulation => unreachable!("not a row"),
        Item::Range => match t.range {
            Range::Free => "Free".into(),
            Range::Loop { .. } => "Loop".into(),
        },
        Item::Measures => s.measures.to_string(),
        Item::Bpm => bpm(t.tempo).to_string(),
        Item::Mode => t.mode.label().into(),
        Item::Tonic => window::pitch_class(t.key().tonic).into(),
        Item::Scale => match t.key().scale {
            Scale::MajorPentatonic => "Major pentatonic".into(),
            Scale::MinorPentatonic => "Minor pentatonic".into(),
        },
        Item::NoteBase => cat_smf_editor::score::note_view::note_name(t.note_base),
        Item::ChordBase => cat_smf_editor::score::note_view::note_name(t.chord_base),
        Item::Quantize => if t.quantize { "1/16" } else { "Off" }.into(),
    }
}

/// Both values of a two-valued item, and the index of the chosen one.
pub(super) fn choices(s: &Session, item: Item) -> Option<([&'static str; 2], usize)> {
    let t = s.settings();
    match item {
        Item::Range => Some((["Free", "Loop"], usize::from(t.range != Range::Free))),
        Item::Scale => Some((
            ["Major pentatonic", "Minor pentatonic"],
            usize::from(t.key().scale == Scale::MinorPentatonic),
        )),
        Item::Quantize => Some((["Off", "1/16"], usize::from(t.quantize))),
        Item::Mode if t.drums => Some((
            Mode::DRUMS.map(Mode::label),
            usize::from(t.mode == Mode::DRUMS[1]),
        )),
        _ => None,
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
        Item::Modulation => modulate(a, s, cycle(&Modulation::ALL, s.settings().modulation, step)),
        _ => edit(s.settings_mut(), item, step, big),
    }
    if !enabled(s, s.item) {
        s.item = first(s);
    }
}

/// The keys captured so far keep the CC1 they were played with.
pub(super) fn modulate(a: &mut App, s: &mut Session, modulation: Modulation) {
    if !enabled(s, Item::Modulation) {
        return;
    }
    if let Err(error) = super::capture::flush(a, s, true) {
        s.error = Some(error);
    }
    s.settings_mut().modulation = modulation;
}

/// The name and keys of an item chosen on the keyboard window, for its tooltip.
pub(super) fn tip(item: Item) -> String {
    format!("{}  {}", item.label(), item.keys())
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
        Item::Bpm => t.tempo = tempo(bpm(t.tempo) + step * if big { 10 } else { 1 }),
        Item::Mode => {
            let all: &[Mode] = if t.drums { &Mode::DRUMS } else { &Mode::ALL };
            t.mode = cycle(all, t.mode, step);
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
            let big = big || t.mode == Mode::Letters;
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
