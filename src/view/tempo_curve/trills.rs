//! Metadata prepared for display; opening the overlay never edits the document.
use crate::app::App;
use cat_smf_editor::{
    edit::{initial_meta, reference_tempo, trill},
    model::Music,
};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct Prepared {
    cached: Option<Cached>,
}

struct Cached {
    key: (u64, u64),
    music: Result<Arc<Music>, String>,
}

pub(super) fn prepare(app: &mut App) -> Result<Arc<Music>, String> {
    let key = (app.song_id, app.generation);
    if app
        .tempo_curve
        .prepared
        .cached
        .as_ref()
        .is_none_or(|c| c.key != key)
    {
        let result = (|| {
            let mut music = app.loaded.as_ref().ok_or("No SMF is loaded")?.music.clone();
            initial_meta::initialize(&mut music)?;
            reference_tempo::initialize(&mut music)?;
            trill::initialize(&mut music)?;
            // Keep the original notes and tempo events until an actual edit.
            Ok(Arc::new(music))
        })();
        app.tempo_curve.prepared.cached = Some(Cached { key, music: result });
    }
    app.tempo_curve
        .prepared
        .cached
        .as_ref()
        .unwrap()
        .music
        .clone()
}
