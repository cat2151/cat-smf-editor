use super::*;

#[test]
#[ignore = "requires installed Surge XT and default audio output; emits a short recorded chord"]
fn recording_keyboard_reaches_the_real_audio_callback_and_creates_a_pattern() {
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let analyzer = crate::play::analysis::Worker::start();
    let mut a = app();
    a.audition = crate::play::audition::Audition::default();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let mut peak = 0f32;
    for events in [
        vec![
            key(Key::Space, true, Modifiers::NONE),
            key(Key::C, true, Modifiers::NONE),
        ],
        vec![key(Key::E, true, Modifiers::NONE)],
        vec![key(Key::C, false, Modifiers::NONE)],
    ] {
        frame(&mut a, &ctx, events, true);
        let until = Instant::now() + Duration::from_millis(200);
        while Instant::now() < until {
            a.audition.pump();
            assert!(a.audition.error().is_none(), "{:?}", a.audition.error());
            if let Some(display) = analyzer.lissajous() {
                for sample in display.samples.iter().flatten() {
                    assert!(sample.is_finite());
                    peak = peak.max(sample.abs());
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let s = a.patterns.recording.session.as_ref().unwrap();
        assert!(s.queue.as_ref().unwrap().lock().unwrap().is_empty());
    }
    assert!(peak > 0.0001, "audio callback stayed silent: {peak}");
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Escape, true, Modifiers::NONE)],
        true,
    );
    assert!(!a.patterns.recording.is_open());
    assert!(!a.audition.key_active());
    let loaded = a.loaded.as_ref().unwrap();
    let c = Catalog::read(&loaded.music).unwrap();
    let p = &c.patterns[&a.patterns.selected.unwrap()];
    let b = body::extract(&loaded.music.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(b.iter().filter(|e| e.status == 0x90).count(), 2);
    assert_eq!(b.iter().filter(|e| e.status == 0x80).count(), 2);
    eprintln!(
        "Native recording: output peak {peak:.6}, {} note events, {} measure(s)",
        b.len(),
        p.meters.len()
    );
    a.audition.reset();
}

#[test]
#[ignore = "requires installed Surge XT and default audio output; emits a short kick"]
fn a_drum_container_sounds_its_kick_lane_through_the_real_audio_callback() {
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let analyzer = crate::play::analysis::Worker::start();
    let mut a = app();
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let track = a.loaded.as_ref().unwrap().music.tracks.len() - 1;
    // Each drum row sounds its default instrument at note 60.
    let patch = &mut a.loaded.as_mut().unwrap().patches[track];
    patch.drums = Some(Default::default());
    patch.layers = (1..=4)
        .map(|id| cat_smf_editor::score::layer_track::ResolvedLayer {
            settings: cat_smf_editor::instrument_layers::Layer {
                id,
                ..Default::default()
            },
            instrument: None,
        })
        .collect();
    a.track = track;
    a.audition = crate::play::audition::Audition::default();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    let mut peak = 0f32;
    let until = Instant::now() + Duration::from_millis(400);
    while Instant::now() < until {
        a.audition.pump();
        assert!(a.audition.error().is_none(), "{:?}", a.audition.error());
        if let Some(display) = analyzer.lissajous() {
            peak = display
                .samples
                .iter()
                .flatten()
                .fold(peak, |p, s| p.max(s.abs()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    assert!(peak > 0.0001, "drum lane stayed silent: {peak}");
    eprintln!("Native drum recording: output peak {peak:.6}");
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Escape, true, Modifiers::SHIFT)],
        true,
    );
    a.audition.reset();
}
