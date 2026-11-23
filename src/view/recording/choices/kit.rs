//! Load the default kit off the UI thread, committing only a complete container.
use super::*;
use cat_smf_editor::score::state_cache::Cache;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(in crate::view::recording) struct KitCreation {
    pub(in crate::view::recording) song: u64,
    pub(in crate::view::recording) result: Receiver<Result<(Patch, Cache), String>>,
}

pub(in crate::view::recording) fn start(a: &App, s: &mut Session) {
    if s.kit_creation.is_some() {
        return;
    }
    let mut cache = a.state_cache.clone();
    let player = a.player_selection.executable.clone();
    let (tx, rx) = mpsc::channel();
    s.kit_creation = Some(KitCreation {
        song: a.song_id,
        result: rx,
    });
    std::thread::spawn(move || {
        let result =
            crate::player::default_kit::load(&mut cache, player).map(|patch| (patch, cache));
        let _ = tx.send(result);
    });
}

pub(in crate::view::recording) fn poll(a: &mut App, s: &mut Session) {
    let Some(job) = &s.kit_creation else {
        return;
    };
    if job.song != a.song_id {
        s.kit_creation = None;
        return;
    }
    if play::armed(s) {
        return;
    }
    let result = match job.result.try_recv() {
        Ok(result) => result,
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => Err("Default kit worker disconnected".into()),
    };
    s.kit_creation = None;
    match result.and_then(|(patch, cache)| {
        if s.sound().container.is_none() {
            play::leave(s);
            super::super::capture::flush(a, s, true)?;
        }
        a.state_cache.absorb(cache);
        super::super::super::patterns::library::new_container_with(a, Some(patch))
    }) {
        Ok(id) => {
            s.error = None;
            if super::super::phrase::adopt(a, s) {
                super::super::phrase::retune(a, s);
            }
            if let Some(i) = s.sounds.iter().position(|x| x.container == Some(id)) {
                super::super::setup::select(a, s, i);
            }
        }
        Err(error) => {
            if s.recorder.is_none() {
                play::start(a, s);
            }
            s.error = Some(error);
        }
    }
}
