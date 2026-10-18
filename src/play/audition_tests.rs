use super::*;
#[test]
fn master_auto_gain_defaults_on_and_selection_resets_preserve_the_switch() {
    let mut audition = Audition::disabled();
    assert!(audition.auto_gain_enabled());
    audition.set_auto_gain_enabled(false);
    audition.reset();
    audition.stop();
    assert!(!audition.auto_gain_enabled());
    audition.set_auto_gain_enabled(true);
    assert!(audition.auto_gain_enabled());
}
#[test]
#[ignore = "requires installed Surge XT and default audio output; emits audio"]
fn default_device_plays_two_patches_in_one_stream() {
    let mut audition = Audition::default();
    let ep = Patch::default();
    let sequence = |duration| Sequence {
        notes: vec![crate::play::audio_sequence::Note {
            start: 0.05,
            duration,
            pitch: 60,
            velocity: 80,
        }],
        end: duration + 0.05,
    };
    assert!(
        audition.sequences(vec![
            (0, Patch::default(), sequence(0.25)),
            (1, ep.clone(), sequence(0.5))
        ]),
        "{:?}",
        audition.error()
    );
    assert_eq!(audition.voice.as_ref().unwrap().natives.len(), 2);
    assert_eq!(audition.voice.as_ref().unwrap().patches[1].1, ep);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(650);
    while std::time::Instant::now() < deadline {
        audition.pump();
        assert!(audition.error().is_none(), "{:?}", audition.error());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    audition.stop();
}
#[test]
#[ignore = "requires installed Surge XT and default audio output; emits audio"]
fn default_device_auditions_duration_and_can_stop() {
    let mut audition = Audition::default();
    assert!(
        audition.play_track(
            0,
            &Patch::default(),
            &[(60, 80, Some(0.25)), (67, 70, Some(0.5))]
        ),
        "{:?}",
        audition.error()
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(650);
    while std::time::Instant::now() < deadline {
        audition.pump();
        assert!(audition.error().is_none(), "{:?}", audition.error());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    audition.stop();
}
