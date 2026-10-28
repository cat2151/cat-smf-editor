//! What the overview draws, as playback sounds it: one roll per track and the minimap of all of them.
use super::{kind, Kind};
use crate::app::App;
use crate::view::piano_roll::{self, Layout};

/// Unzoomed height of one track's pane.
pub(super) const PANE_HEIGHT: f32 = 60.0;

pub(super) struct Panes {
    /// Every track on one roll, with the meter of the current track.
    pub overview: Layout,
    /// One roll per track, without the other tracks' notes.
    pub layouts: Vec<Layout>,
    pub kinds: Vec<Kind>,
}

impl Panes {
    pub(super) fn read(app: &App, tick: u64) -> Option<Self> {
        let loaded = app.loaded.as_ref()?;
        let sources = crate::play::playback::performance(app);
        let tracks: Vec<usize> = (0..sources.len()).collect();
        let overview = piano_roll::layout_from(loaded, &sources, &tracks, app.track, tick);
        let layouts = tracks
            .iter()
            .map(|&track| {
                let mut layout = piano_roll::layout_from(loaded, &sources, &[track], track, tick);
                layout.references.clear();
                piano_roll::panes::fit_height(&mut layout, PANE_HEIGHT);
                layout
            })
            .collect();
        let kinds = tracks.iter().map(|&t| kind(&sources[t].events)).collect();
        Some(Self {
            overview,
            layouts,
            kinds,
        })
    }
}
