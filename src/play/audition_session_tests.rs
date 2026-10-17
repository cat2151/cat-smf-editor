use super::*;

#[test]
fn unprepared_audio_is_not_cached_and_document_gains_stay_local() {
    let mut audio = Audition::disabled();
    assert!(audio.take_prepared().is_none());
    audio.set_gain(0, 0.25);
    assert!(audio.play_track(0, &Patch::default(), &[]));
    let cached = audio.take_prepared().unwrap();
    assert!(cached.is_prepared());
    assert_eq!(audio.gain(0), 0.25);
    assert_eq!(cached.gain(0), 1.0);
    audio.reset();
    assert!(!audio.is_prepared());
    audio.reuse_prepared(cached);
    assert!(audio.is_prepared());
    assert_eq!(audio.gain(0), 0.25);
}

#[test]
#[ignore = "requires default audio output; sends no notes"]
fn native_session_cache_reuses_the_same_voice_without_sounding_notes() {
    let root = std::env::temp_dir().join(format!("cat-smf-session-test-{}", std::process::id()));
    let _library = cat_smf_editor::native_library::load_at(&root).unwrap();
    let mut audio = Audition::default();
    assert!(
        audio.play_track(0, &Patch::default(), &[]),
        "{:?}",
        audio.error()
    );
    let mailbox = audio.voice.as_ref().unwrap().mailbox.clone();
    for _ in 0..3 {
        let cached = audio.take_prepared().unwrap();
        assert!(audio.voice.is_none());
        let mut next = Audition::default();
        next.set_auto_gain_enabled(false);
        next.set_gain(0, 0.25);
        next.reuse_prepared(cached);
        assert!(
            next.play_track(0, &Patch::default(), &[]),
            "{:?}",
            next.error()
        );
        assert!(Arc::ptr_eq(&mailbox, &next.voice.as_ref().unwrap().mailbox));
        next.pump();
        assert!(next.error().is_none(), "{:?}", next.error());
        assert_eq!(next.gain(0), 0.25);
        assert!(mailbox.lock().unwrap().auto_gain_disabled);
        audio = next;
    }
}
