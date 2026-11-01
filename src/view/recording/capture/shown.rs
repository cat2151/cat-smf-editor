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
    workspace_open: bool,
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
            workspace_open: a.patterns.workspace_open,
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
        a.patterns.workspace_open = self.workspace_open;
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

/// Opens `p`'s Capture pattern as any pattern opens and previews it from the phrase's bar.
pub(super) fn audition(a: &mut App, p: &Captured) -> Result<(), String> {
    patterns::library::edit(a, p.pattern)?;
    patterns::preview::start(a, p.pattern)?;
    let track = a.track;
    crate::play::transport::timeline::seek(a, track, p.range.0);
    Ok(())
}

/// A Capture pattern of the list is previewing.
pub(super) fn auditioning(a: &App, phrases: &[Captured]) -> bool {
    crate::play::transport::timeline::running(a)
        && a.patterns
            .preview
            .is_some_and(|id| phrases.iter().any(|p| p.pattern == id))
}
