//! Load the default kit off the UI thread, committing only a complete container.
use super::*;
use cat_smf_editor::{
    drum_kit::Kit,
    patch_reference::PatchReference,
    score::{instrument::ResolvedLayer, state_cache::Cache},
};
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
        let result = (|| {
            let output =
                crate::player::state_request::headless(player.clone(), "--default-drum-kit")?
                    .output()
                    .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).into_owned());
            }
            let (reference, kit): (PatchReference, Kit) =
                serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
            reference.validate()?;
            kit.validate()?;
            let instrument = cache.resolve(&reference, |r| {
                crate::player::state_request::generate(r, player)
            })?;
            let patch = Patch {
                kit: Some(kit),
                layers: vec![ResolvedLayer {
                    settings: Default::default(),
                    instrument: Some(instrument),
                }],
                ..Default::default()
            };
            patch.validate()?;
            Ok((patch, cache))
        })();
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
    let result = match job.result.try_recv() {
        Ok(result) => result,
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => Err("Default kit worker disconnected".into()),
    };
    s.kit_creation = None;
    match result.and_then(|(patch, cache)| {
        a.state_cache.absorb(cache);
        super::super::super::patterns::library::new_container_with(a, Some(patch))
    }) {
        Ok(_) => s.error = None,
        Err(error) => s.error = Some(error),
    }
}
