use super::*;
use cat_smf_editor::score::{drum_kit, note_view::note_on, pattern::Kind};

fn fixture() -> App {
    let mut a = app();
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let track = a.loaded.as_ref().unwrap().music.tracks.len() - 1;
    let patch = Patch {
        kit: Some(cat_smf_editor::drum_kit::Kit {
            layer: 1,
            rows: Some(vec![
                Some(127),
                None,
                Some(49),
                Some(42),
                Some(38),
                Some(36),
            ]),
            muted: vec![],
            swing: Default::default(),
            notes: [36, 38, 42, 49, 127]
                .into_iter()
                .map(|note| drum_kit::Note {
                    note,
                    name: format!("Kit {note}"),
                    velocity: 55,
                })
                .collect(),
        }),
        ..Default::default()
    };
    crate::player::selection::store_track(&mut a, track, &patch).unwrap();
    a.select_track(track);
    a
}

fn event(a: &mut App, c: &egui::Context, k: Key, down: bool) {
    frame(a, c, vec![key(k, down, Modifiers::NONE)], true);
}

#[test]
fn tab_chromatic_starts_at_36_preserves_gaps_and_moves_range_with_held_release() {
    let mut a = fixture();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    event(&mut a, &c, Key::Tab, true);
    event(&mut a, &c, Key::Tab, false);
    assert_eq!(recorder(&a).settings().mode, recording::Mode::Chromatic);
    let pad = recording::Pad::Row { row: 0, column: 0 };
    assert_eq!(super::super::kit::caption(session(&a), pad).0, "Kit 36");
    assert_eq!(
        recorder(&a).pitches(recording::Pad::Row { row: 0, column: 1 }),
        Vec::<u8>::new()
    );
    event(&mut a, &c, Key::C, true);
    assert!(recorder(&a).sounding(38));
    event(&mut a, &c, Key::C, false);
    event(&mut a, &c, Key::Z, true);
    event(&mut a, &c, Key::ArrowUp, true);
    assert!(recorder(&a).sounding(36));
    event(&mut a, &c, Key::Z, false);
    assert!(!recorder(&a).sounding(36));
    event(&mut a, &c, Key::X, true);
    assert!(recorder(&a).sounding(49));
    event(&mut a, &c, Key::X, false);
    event(&mut a, &c, Key::ArrowUp, false);
    for _ in 0..7 {
        event(&mut a, &c, Key::ArrowUp, true);
        event(&mut a, &c, Key::ArrowUp, false);
    }
    event(&mut a, &c, Key::Z, true);
    assert!(recorder(&a).sounding(127));
    event(&mut a, &c, Key::Z, false);
    for _ in 0..12 {
        event(&mut a, &c, Key::ArrowDown, true);
        event(&mut a, &c, Key::ArrowDown, false);
    }
    assert_eq!(session(&a).sound().kit_base, 0);
    frame(
        &mut a,
        &c,
        vec![key(Key::Tab, true, Modifiers::SHIFT)],
        true,
    );
    assert_eq!(recorder(&a).settings().mode, recording::Mode::Note);
    assert_eq!(recorder(&a).pitches(pad), [36]);
}

#[test]
fn kit_capture_take_save_restart_and_single_undo() {
    let mut a = fixture();
    let c = egui::Context::default();
    let work = tempfile::tempdir().unwrap();
    a.config = crate::store::config::Store::open(work.path().join("status.json"));
    open_key(&mut a, &c);
    event(&mut a, &c, Key::Space, true);
    event(&mut a, &c, Key::Z, true);
    rewind(&mut a, Duration::from_millis(100));
    event(&mut a, &c, Key::Z, false);
    event(&mut a, &c, Key::Z, true);
    assert!(recorder(&a).sounding(36));
    rewind(&mut a, Duration::from_millis(100));
    event(&mut a, &c, Key::Z, false);
    assert!(!recorder(&a).sounding(36));
    event(&mut a, &c, Key::Tab, true);
    event(&mut a, &c, Key::Escape, true);
    let music = &a.loaded.as_ref().unwrap().music;
    let catalog = Catalog::read(music).unwrap();
    let take = catalog.patterns[&a.patterns.selected.unwrap()].clone();
    let b = body::extract(&music.tracks[take.track].events, take.range().unwrap()).unwrap();
    assert_eq!(
        b.iter()
            .filter(|e| note_on(e))
            .map(|e| e.data[0])
            .collect::<Vec<_>>(),
        [36, 36]
    );
    let captured = recording::capture::captured(music).unwrap();
    assert_eq!(captured.iter().map(|p| p.notes.len()).sum::<usize>(), 2);
    let path = work.path().join("kit.mid");
    let bytes = cat_smf_editor::smf_io::encode(music).unwrap();
    assert!(super::super::resume::close_for_exit(&mut a));
    a.saved.directory = Some(work.path().into());
    a.saved.save_as(&path).unwrap();
    a.sync_saved_path();
    a.save_session();
    let mut restarted = app();
    restarted.loaded = None;
    restarted.config = crate::store::config::Store::open(work.path().join("status.json"));
    restarted.state_cache =
        cat_smf_editor::score::state_cache::Cache::new(Some(work.path().into()));
    restarted.open_initial(None);
    assert!(restarted.load_error.is_none(), "{:?}", restarted.load_error);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&restarted.loaded.as_ref().unwrap().music).unwrap(),
        bytes
    );
    let c2 = egui::Context::default();
    open_key(&mut restarted, &c2);
    assert!(session(&restarted).sound().patch.kit.is_some());
    assert_eq!(
        recorder(&restarted).settings().mode,
        recording::Mode::Chromatic
    );
    event(&mut restarted, &c2, Key::Z, true);
    assert!(recorder(&restarted).sounding(36));
    a.undo_music().unwrap();
    assert!(Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .values()
        .all(|p| p.kind == Kind::Capture));
}

#[test]
fn kit_phrase_overdub_keeps_other_pitches_and_swing_matches_playback() {
    let mut a = fixture();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    frame(&mut a, &c, vec![key(Key::E, true, Modifiers::CTRL)], true);
    event(&mut a, &c, Key::Z, true);
    rewind(&mut a, Duration::from_millis(100));
    event(&mut a, &c, Key::Z, false);
    rewind(&mut a, Duration::from_secs(3));
    frame(&mut a, &c, vec![], true);
    assert!(phrase::looping(session(&a)));
    event(&mut a, &c, Key::X, true);
    rewind(&mut a, Duration::from_millis(100));
    event(&mut a, &c, Key::X, false);
    rewind(&mut a, Duration::from_secs(3));
    frame(&mut a, &c, vec![], true);
    let phrase::Looper::Looping { notes, .. } = &session(&a).sound().looper else {
        panic!()
    };
    let mut pitches = notes
        .iter()
        .filter(|n| n.2 > 0)
        .map(|n| n.1)
        .collect::<Vec<_>>();
    pitches.sort();
    assert_eq!(pitches, [36, 38]);
    let mut patch = session(&a).sound().patch.clone();
    patch.kit.as_mut().unwrap().swing.insert(
        36,
        cat_smf_editor::drum_kit::Swing {
            enabled: true,
            amount: 75,
        },
    );
    let notes = vec![
        (Duration::from_millis(125), 36, 100),
        (Duration::from_millis(200), 36, 0),
    ];
    let swung = super::super::kit::swung(&patch, 480, 500_000, &notes);
    assert!(swung[0].0 > notes[0].0);
    // A sixteenth at 125 ms moves to 187.5 ms with 75% swing.
    assert_eq!(swung[0].0, Duration::from_micros(187_500));
    assert_eq!(notes[0].0, Duration::from_millis(125));
}

#[test]
fn kit_assignment_edits_follow_without_sticking_held_notes_and_pages_survive_switching() {
    let mut a = fixture();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    event(&mut a, &c, Key::Z, true);
    let track = session(&a).sound().track;
    let mut patch = session(&a).sound().patch.clone();
    patch.kit.as_mut().unwrap().rows.as_mut().unwrap()[5] = Some(35);
    crate::player::selection::store_track(&mut a, track, &patch).unwrap();
    frame(&mut a, &c, vec![], true);
    assert!(recorder(&a).sounding(36));
    event(&mut a, &c, Key::Z, false);
    event(&mut a, &c, Key::Z, true);
    assert!(recorder(&a).sounding(35));
    event(&mut a, &c, Key::Z, false);
    event(&mut a, &c, Key::X, false);
    assert!((0..128).all(|p| !recorder(&a).sounding(p)));
    event(&mut a, &c, Key::ArrowUp, true);
    let mut state = std::mem::take(&mut a.patterns.recording);
    super::super::play::button(
        &mut a,
        state.session.as_mut().unwrap(),
        window::Button::CreateSound(false),
    );
    a.patterns.recording = state;
    assert!(session(&a).sound().patch.kit.is_none());
    event(&mut a, &c, Key::ArrowLeft, true);
    assert!(session(&a).sound().patch.kit.is_some());
    assert_eq!(session(&a).sound().kit_bank, 0);
}

#[test]
fn kit_creation_waits_for_an_armed_take_and_preserves_the_fallback_capture_sound() {
    let mut a = app();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    event(&mut a, &c, Key::Z, true);
    rewind(&mut a, Duration::from_millis(100));
    let kit_patch = fixture().loaded.unwrap().patches.last().unwrap().clone();
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok((kit_patch, a.state_cache.clone()))).unwrap();
    a.patterns.recording.session.as_mut().unwrap().kit_creation = Some(choices::KitCreation {
        song: a.song_id,
        result: rx,
    });
    // Arm before the asynchronous result is consumed.
    let mut state = std::mem::take(&mut a.patterns.recording);
    super::super::play::button(&mut a, state.session.as_mut().unwrap(), window::Button::Rec);
    choices::poll_kit(&mut a, state.session.as_mut().unwrap());
    assert!(state.session.as_ref().unwrap().kit_creation.is_some());
    super::super::play::leave(state.session.as_mut().unwrap());
    choices::poll_kit(&mut a, state.session.as_mut().unwrap());
    a.patterns.recording = state;
    assert!(session(&a).sound().patch.kit.is_some());
    let loaded = a.loaded.as_ref().unwrap();
    let captures = recording::capture::captured(&loaded.music).unwrap();
    assert_eq!(captures.len(), 1);
    let catalog = Catalog::read(&loaded.music).unwrap();
    assert!(loaded.patches[catalog.tracks[&captures[0].container]]
        .kit
        .is_none());
}

#[test]
fn all_128_assigned_notes_are_reachable_by_full_keyboard_pages() {
    let mut a = fixture();
    let track = a.track;
    let mut patch = a.loaded.as_ref().unwrap().patches[track].clone();
    patch.kit.as_mut().unwrap().rows = Some((0..128).rev().map(Some).collect());
    crate::player::selection::store_track(&mut a, track, &patch).unwrap();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    for page in 0..3 {
        let mut offset = 0;
        for (row, count) in recording::ROW_KEYS.into_iter().enumerate() {
            for column in 0..count {
                let pitch = page * 48 + offset;
                let expected = if pitch < 128 {
                    vec![pitch as u8]
                } else {
                    vec![]
                };
                assert_eq!(
                    recorder(&a).pitches(recording::Pad::Row {
                        row: row as u8,
                        column
                    }),
                    expected
                );
                offset += 1;
            }
        }
        event(&mut a, &c, Key::ArrowUp, true);
        event(&mut a, &c, Key::ArrowUp, false);
    }
    assert_eq!(session(&a).sound().kit_bank, 2);
}

#[test]
fn eight_kit_rows_are_all_playable_without_paging_and_follow_edits() {
    let mut a = fixture();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    let track = session(&a).sound().track;
    let mut patch = session(&a).sound().patch.clone();
    patch.kit.as_mut().unwrap().rows = Some((36..44).rev().map(Some).collect());
    crate::player::selection::store_track(&mut a, track, &patch).unwrap();
    frame(&mut a, &c, vec![], true);
    for (column, key) in [
        Key::Z,
        Key::X,
        Key::C,
        Key::V,
        Key::B,
        Key::N,
        Key::M,
        Key::Comma,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            recorder(&a).pitches(recording::Pad::Row {
                row: 0,
                column: column as u8
            }),
            [36 + column as u8]
        );
        event(&mut a, &c, key, true);
        assert!(recorder(&a).sounding(36 + column as u8));
        event(&mut a, &c, key, false);
    }
    event(&mut a, &c, Key::ArrowUp, true);
    assert_eq!(session(&a).sound().kit_bank, 0);
}

#[test]
fn selected_pattern_follows_its_container_and_releases_the_old_keys() {
    use cat_smf_editor::score::pattern::operations;
    let mut a = fixture();
    let kit_track = a.track;
    let mut patterns = vec![];
    a.edit_tracks(|m| {
        patterns.push(operations::create(m, kit_track, None)?);
        let melodic = operations::create_source(m)?;
        patterns.push(operations::create(m, melodic, None)?);
        Ok(())
    })
    .unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    event(&mut a, &ctx, Key::Z, true);
    let old_queue = session(&a).queue().unwrap().clone();
    crate::view::patterns::library::edit(&mut a, patterns[1]).unwrap();
    assert_eq!(session(&a).sound().track, a.track);
    assert!(!recorder(&a).settings().drums);
    assert_eq!(old_queue.lock().unwrap().words.back(), Some(&0x2080_2400));
    event(&mut a, &ctx, Key::Z, false);
    event(&mut a, &ctx, Key::Z, true);
    assert!(recorder(&a).sounding(60));
    crate::view::patterns::library::edit(&mut a, patterns[0]).unwrap();
    assert_eq!(session(&a).sound().track, kit_track);
    assert!(recorder(&a).settings().drums);
    assert_eq!(a.patterns.selected, Some(patterns[0]));
    event(&mut a, &ctx, Key::Z, false);
    event(&mut a, &ctx, Key::Z, true);
    assert!(recorder(&a).sounding(36));
    let drums = cat_smf_editor::score::drums::Settings::default();
    let patch = Patch {
        layers: drums
            .rows
            .iter()
            .map(|r| cat_smf_editor::score::instrument::ResolvedLayer {
                settings: cat_smf_editor::instrument_layers::Layer {
                    id: r.layer,
                    ..Default::default()
                },
                instrument: None,
            })
            .collect(),
        drums: Some(drums),
        ..Default::default()
    };
    crate::view::patterns::library::add_container_with(&mut a, Some(patch)).unwrap();
    assert_eq!(session(&a).sound().track, a.track);
    assert!(session(&a).sound().patch.drums.is_some());
    assert!(recorder(&a).settings().drums);
}

#[test]
fn pattern_follow_waits_for_the_recording_to_finish() {
    use cat_smf_editor::score::pattern::operations;
    let mut a = fixture();
    let original = a.track;
    let mut pattern = 0;
    a.edit_tracks(|m| {
        let track = operations::create_source(m)?;
        pattern = operations::create(m, track, None)?;
        Ok(())
    })
    .unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    event(&mut a, &ctx, Key::Space, true);
    crate::view::patterns::library::edit(&mut a, pattern).unwrap();
    assert_eq!(session(&a).sound().track, original);
    let mut state = std::mem::take(&mut a.patterns.recording);
    super::super::play::leave(state.session.as_mut().unwrap());
    a.patterns.recording = state;
    super::super::following::apply(&mut a);
    assert_eq!(session(&a).sound().track, a.track);
    assert!(!recorder(&a).settings().drums);
}
