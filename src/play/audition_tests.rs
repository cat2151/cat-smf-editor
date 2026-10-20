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
        audition.sequences(
            vec![
                (0, Patch::default(), sequence(0.25)),
                (1, ep.clone(), sequence(0.5))
            ],
            Vec::new()
        ),
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
            &[(60, 80, Some(0.25)), (67, 70, Some(0.5))],
            None
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
#[test]
fn an_audition_with_a_take_sounds_its_piece_through_the_mixer_until_stopped() {
    use cat_smf_editor::voicevox::score::{one_note, LEAD_FRAMES};
    let score = one_note(60, 4);
    let total = score.total_frames() as usize * 512;
    let samples = (0..total).flat_map(|n| [n as f32, 0.0]).collect();
    let take = crate::play::sung::piece(&score, Arc::new(samples), 48000, 0.0).unwrap();
    let mut velocities = [0; 128];
    velocities[60] = 80;
    let command = |generation, take: Option<crate::play::sung::Take>| Command {
        generation,
        parts: Arc::new(vec![Part {
            velocities,
            durations: [u64::MAX; 128],
            take: take.map(Arc::new),
            ..Part::default()
        }]),
        gains: Arc::new(vec![1.0]),
        auto_gain_disabled: true,
        ..Command::default()
    };
    let failed = std::sync::atomic::AtomicBool::new(false);
    let mut renders = vec![crate::play::layer_render::Render::new(Vec::new(), None)];
    let mut mixer = mix::Mixer::new(48000, 2, 1, None);
    let playing = command(1, Some(take));
    let head = LEAD_FRAMES as usize * 512;
    for block in 0..2 {
        let out = mixer.block(&mut renders, &playing, 1024, &failed).to_vec();
        for (k, frame) in out.chunks(2).enumerate() {
            assert_eq!(frame[0], (head + block * 1024 + k) as f32, "block {block}");
        }
    }
    let stopped = command(2, None);
    let out = mixer.block(&mut renders, &stopped, 1024, &failed);
    assert!(out.iter().all(|s| *s == 0.0));
    assert!(!failed.load(std::sync::atomic::Ordering::Relaxed));
}
