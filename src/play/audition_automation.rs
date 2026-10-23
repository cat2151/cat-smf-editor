use super::{Audition, Part, Voice};
use crate::play::audio_sequence::Sequence;
use cat_smf_editor::score::instrument::Patch;
use std::sync::Arc;

impl Audition {
    pub fn restore_automated_baselines(&mut self) {
        if !self.voice.as_ref().is_some_and(|voice| voice.automated()) {
            return;
        }
        let patches = self.voice.as_ref().unwrap().patches.clone();
        let previous = self.voice.take();
        self.progress = None;
        match Voice::open(&patches, previous, &self.master, self.auto_gain_disabled) {
            Ok(voice) => self.voice = Some(voice),
            Err(error) => self.error = Some(format!("Audio baseline restoration: {error}")),
        }
    }
    pub fn sequences_automated(
        &mut self,
        parts: Vec<(usize, Patch, Sequence)>,
        mut takes: Vec<(usize, Arc<crate::play::sung::Take>)>,
        automation: &crate::play::automation::Automation,
        origin: f64,
    ) -> bool {
        #[cfg(test)]
        {
            self.takes = takes.clone();
        }
        if parts.is_empty() {
            self.stop();
            return true;
        }
        // A track without notes, such as a conductor track, only spans time.
        let patches = parts
            .iter()
            .filter(|(track, _, sequence)| {
                !sequence.notes.is_empty() || automation.owner(*track).is_some()
            })
            .map(|(track, patch, _)| (*track, patch.clone()))
            .collect::<Vec<_>>();
        if automation.active() && !self.disabled {
            let previous = self.voice.take();
            self.progress = None;
            self.error = None;
            match Voice::open_automated(
                &patches,
                previous,
                &self.master,
                self.auto_gain_disabled,
                automation,
            ) {
                Ok(voice) => self.voice = Some(voice),
                Err(error) => {
                    self.error = Some(format!("Audio automation: {error}"));
                    return false;
                }
            }
        } else if !self.prepare(&patches) {
            return false;
        }
        let rate = self.rate();
        self.send(
            parts
                .into_iter()
                .map(|(track, _, sequence)| {
                    let plan = automation.owner(track).map(|plan| {
                        Arc::new(crate::play::automation::RuntimePlan::new(
                            plan, &sequence, origin, rate,
                        ))
                    });
                    let take = takes
                        .iter()
                        .position(|(t, _)| *t == track)
                        .map(|i| takes.swap_remove(i).1);
                    (
                        track,
                        Part {
                            timeline: Some(Arc::new(crate::play::event_timeline::Timeline::new(
                                &sequence, origin, rate,
                            ))),
                            automation: plan,
                            sequence: Some(Arc::new(sequence)),
                            take,
                            ..Part::default()
                        },
                    )
                })
                .collect(),
        );
        true
    }
}
