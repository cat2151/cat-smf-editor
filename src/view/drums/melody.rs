//! Pitched notes of an ordinary pattern container on the shared step grid.
//! The layout maps pitches to rows; every edit names a pitch, a measure and a step.
use super::*;
use cat_smf_editor::score::{
    harmony::{self, PitchClasses},
    step_notes::{self, Cell, Length, Note},
};
pub(crate) mod commands;
mod controls;
mod gesture;
mod widgets;

const VELOCITY: u8 = 100;
const MIDDLE_C: u8 = 60;

#[derive(Default)]
pub(crate) struct State {
    /// The cursor's pitch; `None` until the grid first shows.
    pitch: Option<u8>,
    /// Scroll the cursor's row into view on the next frame. `center` instead centers the span
    /// from the highest to the lowest note's row, when the grid opens or its rows change.
    reveal: bool,
    center: bool,
    chords: harmony::Chords,
    /// The grid's vertical scroll last frame, and the one the next frame takes after a zoom.
    scroll: f32,
    pending: Option<f32>,
    /// Where a drag on the zoom bar started, from the bar's top.
    drag: Option<f32>,
    gesture: Option<gesture::Gesture>,
}

/// An ordinary (neither Drum nor Drum kit) container's pattern is being edited.
pub(crate) fn active(a: &App) -> bool {
    crate::view::patterns::mode::editing(a, a.track).is_some()
        && a.loaded
            .as_ref()
            .is_some_and(|l| l.patches.get(a.track).is_some_and(|p| !p.sequencer()))
}

/// Rows (pitches, highest first) and, per step of the shown measure, the pitches its harmony offers.
pub(super) struct Layout {
    pub(super) rows: Vec<u8>,
    pub(super) columns: Vec<PitchClasses>,
    /// The pitch class each octave of rows starts from: the key's tonic at the measure start.
    pub(super) tonic: u8,
}

/// Every pitch any step of the shown measure offers, in all octaves, and every pitch
/// already sounding in it. Moving the cursor within the measure keeps the rows.
pub(super) fn layout(a: &mut App) -> Layout {
    let set = a.config.melody_rows(a.patterns.container);
    let measure = playback::measure(a).ok();
    let harmonies = measure.as_ref().and_then(|m| harmonies(a, m));
    let tonic = harmonies
        .as_ref()
        .and_then(|h| h.first())
        .map_or(0, |h| h.tonic);
    let columns: Vec<PitchClasses> = measure.as_ref().map_or_else(Vec::new, |m| {
        harmonies.as_ref().map_or_else(
            || vec![PitchClasses::ALL; m.steps.len()],
            |h| h.iter().map(|h| h.pitches(set)).collect(),
        )
    });
    let offered = if measure.is_some() {
        columns
            .iter()
            .fold(PitchClasses::default(), |all, &c| all.union(c))
    } else {
        PitchClasses::ALL
    };
    let placed: Vec<u8> = measure.as_ref().map_or_else(Vec::new, |m| {
        notes(a)
            .iter()
            .filter(|n| n.start < m.end && n.end.max(n.start + 1) > m.start)
            .map(|n| n.pitch)
            .collect()
    });
    Layout {
        rows: (0..=127)
            .rev()
            .filter(|&p| offered.contains(p) || placed.contains(&p))
            .collect(),
        columns,
        tonic,
    }
}

pub(super) fn rows(a: &mut App) -> Vec<u8> {
    layout(a).rows
}

/// The harmony at each step of `measure`, placed where the pattern sounds in the song.
fn harmonies(a: &mut App, measure: &drums::Measure) -> Option<Vec<harmony::Harmony>> {
    let context = crate::view::patterns::mode::editing(a, a.track)?.clone();
    let origin = crate::view::patterns::timing::pattern_tick(a, context.pattern).ok()?;
    let mut chords = std::mem::take(&mut a.drum_editor.melody.chords);
    let found = (|| {
        let l = a.loaded.as_ref()?;
        let start = l.edit_range(&context).ok()?.0;
        measure
            .steps
            .iter()
            .map(|&s| {
                let tick = origin.checked_add(s.checked_sub(start)?)?;
                Some(harmony::at_with(&l.music, &l.keys, &mut chords, tick))
            })
            .collect()
    })();
    a.drum_editor.melody.chords = chords;
    found
}

/// The next row set; the cursor moves to the nearest pitch still shown.
fn next_rows(a: &mut App, set: Option<harmony::Set>) {
    let container = a.patterns.container;
    let set = set.unwrap_or_else(|| a.config.melody_rows(container).next());
    if let Some(container) = container {
        a.config.select_melody_rows(container, set);
    }
    if let Some(p) = nearest(&rows(a), pitch(a)) {
        a.drum_editor.melody.pitch = Some(p);
    }
    a.drum_editor.melody.reveal = true;
    a.drum_editor.melody.center = true;
}

/// English note name, middle C (60) being C4.
pub(super) fn name(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!(
        "{}{}",
        NAMES[usize::from(pitch % 12)],
        i32::from(pitch / 12) - 1
    )
}

pub(super) fn black(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

/// The shown row nearest to `pitch`, the lower one on a tie.
fn nearest(rows: &[u8], pitch: u8) -> Option<u8> {
    rows.iter().copied().min_by_key(|&p| (p.abs_diff(pitch), p))
}

fn notes(a: &App) -> Vec<Note> {
    a.loaded.as_ref().map_or_else(Vec::new, |l| {
        step_notes::notes(&l.visible().tracks[a.track])
    })
}

pub(super) fn pitch(a: &App) -> u8 {
    a.drum_editor.melody.pitch.unwrap_or(MIDDLE_C)
}

pub(super) fn step(a: &App) -> usize {
    a.drum_editor.field.saturating_sub(2)
}

/// The cells of `pitch` in the shown measure.
pub(super) fn cells(a: &App, notes: &[Note], measure: &drums::Measure, pitch: u8) -> Vec<Cell> {
    let l = a.loaded.as_ref().unwrap();
    l.time.map(a.track).map_or_else(
        || vec![Cell::Empty; measure.steps.len()],
        |map| step_notes::cells(notes, map, l.music.division, measure, pitch),
    )
}

/// The first show puts the cursor in the middle of the pattern's notes, or on C4.
fn sync(a: &mut App) {
    if a.drum_editor.melody.pitch.is_none() {
        let notes = notes(a);
        let pitch = match (
            notes.iter().map(|n| n.pitch).min(),
            notes.iter().map(|n| n.pitch).max(),
        ) {
            (Some(low), Some(high)) => low + (high - low) / 2,
            _ => MIDDLE_C,
        };
        let m = &mut a.drum_editor.melody;
        m.pitch = Some(pitch);
        m.reveal = true;
        m.center = true;
    }
    let rows = rows(a);
    if let Some(pitch) = nearest(&rows, pitch(a)) {
        a.drum_editor.melody.pitch = Some(pitch);
    }
    let steps = playback::measure(a).map_or(16, |m| m.steps.len());
    a.drum_editor.field = a.drum_editor.field.clamp(2, steps + 1);
    a.drum_editor.row = usize::from(pitch(a));
}

fn move_cursor(a: &mut App, rows: isize, steps: isize) {
    let shown = self::rows(a);
    let at = shown.iter().position(|&p| p == pitch(a)).unwrap_or(0);
    if let Some(&p) = shown.get(
        at.saturating_add_signed(rows)
            .min(shown.len().saturating_sub(1)),
    ) {
        a.drum_editor.melody.pitch = Some(p);
    }
    let last = playback::measure(a).map_or(16, |m| m.steps.len()) + 1;
    a.drum_editor.field = a
        .drum_editor
        .field
        .saturating_add_signed(steps)
        .clamp(2, last);
    a.drum_editor.melody.reveal = true;
}

fn octave(a: &mut App, up: bool) {
    let target = if up {
        pitch(a).saturating_add(12).min(127)
    } else {
        pitch(a).saturating_sub(12)
    };
    if let Some(p) = nearest(&rows(a), target) {
        a.drum_editor.melody.pitch = Some(p);
    }
    a.drum_editor.melody.reveal = true;
}

fn toggle(a: &mut App) -> Result<(), String> {
    let measure = playback::measure(a)?;
    let (track, number, pitch, step) = (a.track, a.drum_editor.page + 1, pitch(a), step(a));
    let placing = cells(a, &notes(a), &measure, pitch)
        .get(step)
        .is_some_and(|c| *c == Cell::Empty);
    a.edit_music(|m| step_notes::toggle(m, track, pitch, number, step, VELOCITY))?;
    if placing {
        preview(a, pitch)?;
    }
    Ok(())
}

fn resize(a: &mut App, length: Length) -> Result<(), String> {
    playback::measure(a)?;
    let context = crate::view::patterns::mode::editing(a, a.track)
        .ok_or(message::PATTERN_EDIT_TARGET_NO_LONGER_EXISTS)?;
    let limit = a
        .loaded
        .as_ref()
        .ok_or(message::NO_SONG)?
        .edit_range(context)?
        .1;
    let (track, number, pitch, step) = (a.track, a.drum_editor.page + 1, pitch(a), step(a));
    a.edit_music(|m| step_notes::resize(m, track, pitch, number, step, length, limit))
}

/// A placed note sounds briefly, through the same preview the kit uses.
fn preview(a: &mut App, pitch: u8) -> Result<(), String> {
    let patch = a.loaded.as_ref().ok_or(message::NO_SONG)?.patches[a.track].clone();
    a.audition.stop_key();
    if !a
        .audition
        .preview_key(a.track, &patch, pitch, VELOCITY, Some(0.35), None)
    {
        return Err(a.audition.error().unwrap_or("Preview failed").into());
    }
    a.drum_editor.kit.preview = Some((pitch, std::time::Instant::now()));
    Ok(())
}

/// A pane leaves the keys to the window holding it, which routed them before its panes.
pub(super) fn show(a: &mut App, ctx: &egui::Context, pane: Option<&mut egui::Ui>) {
    sync(a);
    kit::expire_preview(a);
    if pane.is_none() {
        crate::view::commands::route(a, ctx);
    }
    if !super::is_open(a) {
        return;
    }
    sync(a);
    widgets::show(a, ctx, pane);
}

/// A gesture cannot see its release while its pane is hidden.
pub(super) fn suspend(a: &mut App) {
    a.drum_editor.melody.gesture = None;
}
