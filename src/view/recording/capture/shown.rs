//! A phrase shown and auditioned as its Capture pattern in the editor, and the editor's view
//! put back when the Capture window closes.
use super::*;
use crate::view::patterns::{self, navigation};
use cat_smf_editor::score::pattern::editing::Context;

/// What the editor showed before the first phrase.
pub(super) struct Back {
    track: usize,
    note_mode: bool,
    editing: Option<Context>,
    raw_context: Option<Context>,
    selected: Option<u64>,
    container: Option<u64>,
    navigation: navigation::State,
    tick: u64,
}

impl Back {
    pub(super) fn of(a: &App) -> Self {
        Self {
            track: a.track,
            note_mode: a.note_mode,
            editing: a.patterns.editing.clone(),
            raw_context: a.patterns.raw_context.clone(),
            selected: a.patterns.selected,
            container: a.patterns.container,
            navigation: a.patterns.navigation.clone(),
            tick: crate::play::playback::location(a).0,
        }
    }

    pub(super) fn restore(self, a: &mut App) {
        crate::play::transport::stop(a);
        crate::view::drums::close(a);
        a.note_mode = self.note_mode;
        a.set_track(self.track);
        a.patterns.editing = self.editing;
        a.patterns.raw_context = self.raw_context;
        a.patterns.edit_song = a.song_id;
        a.patterns.selected = self.selected;
        a.patterns.container = self.container;
        a.patterns.navigation = self.navigation;
        a.generation = a.generation.wrapping_add(1);
        patterns::refresh(a);
        navigation::present(a);
        crate::cursor::moves::at_tick(a, self.tick);
    }
}

/// A Capture pattern of the list is previewing.
pub(super) fn auditioning(a: &App, phrases: &[Captured]) -> bool {
    a.playback
        .pending
        .as_ref()
        .map(|p| &p.active)
        .or(a.playback.active.as_ref())
        .is_some_and(|active| {
            matches!(active.mode, crate::play::playback::Mode::Pattern(id)
            if phrases.iter().any(|p| p.pattern == id))
        })
}

/// The transport's pattern clock, including preparation and seeks.
fn clock(a: &App) -> Option<(u64, &cat_smf_editor::score::audition_timing::TempoMap, f64)> {
    let pending = a.playback.pending.as_ref();
    let active = pending.map(|p| &p.active).or(a.playback.active.as_ref())?;
    let crate::play::playback::Mode::Pattern(id) = active.mode else {
        return None;
    };
    let elapsed = if pending.is_some() {
        0.0
    } else {
        a.audition.seconds()?
    };
    Some((
        id,
        active.tempos.get(active.clock_track)?,
        active.start_seconds + elapsed,
    ))
}

/// The phrase at the transport's pattern-relative tick.
pub(super) fn position(a: &App, phrases: &[Captured]) -> Option<(usize, u64)> {
    let (id, tempo, seconds) = clock(a)?;
    let tick = tempo.tick_after(0, seconds);
    phrases.iter().enumerate().find_map(|(i, p)| {
        (p.pattern == id && (p.range.0..p.range.1).contains(&tick)).then(|| (i, tick - p.range.0))
    })
}

/// The current capture, or the countdown across the gap to the next capture.
pub(super) fn caption(a: &App, phrases: &[Captured]) -> Option<String> {
    let (id, tempo, seconds) = clock(a)?;
    let tick = tempo.tick_after(0, seconds);
    if let Some(p) = phrases
        .iter()
        .find(|p| p.pattern == id && (p.range.0..p.range.1).contains(&tick))
    {
        return Some(format!("#{}", p.sequence));
    }
    let previous = phrases
        .iter()
        .filter(|p| p.pattern == id && p.range.1 <= tick)
        .max_by_key(|p| p.range.1)?;
    let next = phrases
        .iter()
        .filter(|p| p.pattern == id && p.range.0 > tick)
        .min_by_key(|p| p.range.0)?;
    let wait = (tempo.seconds_at(next.range.0) - seconds).ceil().max(1.0) as u64;
    Some(format!(
        "#{} to #{} wait {wait}",
        previous.sequence, next.sequence
    ))
}
