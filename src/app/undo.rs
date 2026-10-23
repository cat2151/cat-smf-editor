//! In-memory SMF snapshots; playback caches are rebuilt when undoing.
use super::App;
use cat_smf_editor::model::Music;
use std::collections::VecDeque;

const LIMIT: usize = 100;

#[derive(Default)]
pub struct History {
    entries: VecDeque<Music>,
    volume_gesture: Option<usize>,
}

impl History {
    pub fn available(&self) -> bool {
        !self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.volume_gesture = None;
    }

    pub(crate) fn push(&mut self, music: Music) {
        self.volume_gesture = None;
        if self.entries.len() == LIMIT {
            self.entries.pop_front();
        }
        self.entries.push_back(music);
    }

    pub(crate) fn record_volume(&mut self, before: Music, track: usize, dragging: bool) {
        if !dragging || self.volume_gesture != Some(track) {
            self.push(before);
        }
        self.volume_gesture = dragging.then_some(track);
    }

    pub(crate) fn end_volume_gesture(&mut self) {
        self.volume_gesture = None;
    }
}

impl App {
    pub fn undo_music(&mut self) -> Result<(), String> {
        let Some(music) = self.undo.entries.back().cloned() else {
            return Ok(());
        };
        // Prepare everything before changing the document, audio or history cursor.
        let sound = self.resolve_sound(&music)?;
        let loaded = self.loaded.as_ref().ok_or("No SMF is loaded")?;
        self.saved.remember(&loaded.original, &music)?;
        self.audition.reset();
        self.playback.active = None;
        self.playback.pending = None;
        self.playback.repeat = None;
        self.audition.clear_gains();
        for (track, gain) in sound.volumes.into_iter().enumerate() {
            self.audition.set_gain(track, gain);
        }
        self.audition.set_master(sound.master);
        self.loaded.as_mut().unwrap().patches = sound.patches;
        self.install_music(music, false);
        // Reject delayed browser selections prepared against the state we just left.
        self.song_id = self.song_id.wrapping_add(1);
        self.undo.end_volume_gesture();
        self.undo.entries.pop_back();
        Ok(())
    }

    /// Commit only after encoding succeeds. The SMF includes track output gains.
    pub(crate) fn apply_music(&mut self, music: Music) -> Result<(), String> {
        self.commit_music(music, true)
    }

    pub(crate) fn apply_volume_music(&mut self, music: Music) -> Result<(), String> {
        self.commit_music(music, false)
    }

    fn commit_music(&mut self, music: Music, restart: bool) -> Result<(), String> {
        let loaded = self.loaded.as_ref().ok_or("No SMF is loaded")?;
        self.saved.remember(&loaded.original, &music)?;
        self.install_music(music, restart);
        Ok(())
    }

    fn install_music(&mut self, music: Music, restart: bool) {
        let restore_parameters = self.loaded.as_ref().is_some_and(|loaded| {
            use cat_smf_editor::edit::parameter_curve;
            match (
                parameter_curve::read(&loaded.music),
                parameter_curve::read(&music),
            ) {
                (Ok(old), Ok(new)) => old.curves.iter().any(|(track, curve)| {
                    !curve.points.is_empty()
                        && new.curves.get(track).is_none_or(|next| {
                            next.points.is_empty() || next.target != curve.target
                        })
                }),
                _ => false,
            }
        });
        let session = self.session();
        let resume = restart
            .then(|| crate::play::transport::timeline::capture(self))
            .flatten();
        let loaded = self.loaded.as_mut().unwrap();
        loaded.music = music;
        loaded.refresh();
        self.generation += 1;
        if restore_parameters && restart {
            self.audition.restore_automated_baselines();
        }
        if let Some(session) = session {
            self.restore_position(&session);
        }
        if let Some(resume) = resume {
            crate::play::transport::timeline::resume(self, resume);
        }
    }
}

#[cfg(test)]
mod sound_tests;
#[cfg(test)]
mod tests;
