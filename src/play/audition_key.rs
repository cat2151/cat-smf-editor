//! Live keyboard voices must never replace a song's sequence or progress clock.
use super::*;

impl Audition {
    /// With a `take`, its first piece sounds in place of the track's instruments.
    pub fn play_key(
        &mut self,
        track: usize,
        patch: &Patch,
        pitch: u8,
        take: Option<Arc<crate::play::sung::Take>>,
    ) -> bool {
        let disabled = self.disabled;
        let master = self.master.clone();
        let gains = self.gains.clone();
        let auto_gain_disabled = self.auto_gain_disabled;
        let voice = self.key_voice.get_or_insert_with(|| {
            Box::new(Self {
                disabled,
                ..Self::default()
            })
        });
        voice.set_master(master);
        voice.gains = gains;
        voice.set_auto_gain_enabled(!auto_gain_disabled);
        #[cfg(test)]
        {
            self.auditioned = vec![pitch];
            self.sung = take.clone();
        }
        self.key_active = voice.play_track(track, patch, &[(pitch, 80, None)], take);
        self.key_active
    }

    pub fn key_active(&self) -> bool {
        self.key_active
    }

    pub fn stop_key(&mut self) {
        self.key_active = false;
        if let Some(voice) = &mut self.key_voice {
            voice.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires installed Surge XT and default audio output; emits audio"]
    fn live_key_output_preserves_the_running_sequence_and_clock() {
        // Test binaries do not run main(), which loads the embedded host DLL.
        let cache = cat_smf_editor::storage_paths::cache_directory().unwrap();
        let _library = cat_smf_editor::native_library::load_at(&cache).unwrap();
        let mut audition = Audition::default();
        assert!(
            audition.sequences(
                vec![(
                    0,
                    Patch::default(),
                    Sequence {
                        notes: vec![crate::play::audio_sequence::Note {
                            start: 0.0,
                            duration: 1.0,
                            pitch: 60,
                            velocity: 64,
                        }],
                        end: 2.0,
                    }
                )],
                Vec::new()
            ),
            "{:?}",
            audition.error()
        );
        let clock = audition.progress.clone().unwrap();
        let generation = audition
            .voice
            .as_ref()
            .unwrap()
            .mailbox
            .lock()
            .unwrap()
            .generation;
        assert!(
            audition.play_key(0, &Patch::default(), 67, None),
            "{:?}",
            audition.error()
        );
        assert!(Arc::ptr_eq(&clock, audition.progress.as_ref().unwrap()));
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        while std::time::Instant::now() < deadline {
            audition.pump();
            assert!(audition.error().is_none(), "{:?}", audition.error());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        audition.stop_key();
        assert!(audition.seconds().unwrap() > 0.0);
        assert!(Arc::ptr_eq(&clock, audition.progress.as_ref().unwrap()));
        assert_eq!(
            audition
                .voice
                .as_ref()
                .unwrap()
                .mailbox
                .lock()
                .unwrap()
                .generation,
            generation
        );
        audition.stop();
    }
}
