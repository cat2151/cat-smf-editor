use super::*;
use cat_smf_editor::score::{
    pattern::{operations, Catalog},
    scene::arrangement::{self as grid, PatternOrScene},
};

#[test]
fn pattern_song_starts_before_ui_and_keeps_playing_after_editor_restore() {
    for drum in [false, true] {
        for note_mode in [false, true] {
            for edited in [false, true] {
                let (root, path) = fixture();
                let (mut source_app, source, id) = crate::view::patterns::edit_tests::fixture();
                if drum {
                    let patch = cat_smf_editor::score::instrument::Patch {
                        kit: None,
                        drums: Some(Default::default()),
                        layers: (1..=4)
                            .map(|id| cat_smf_editor::score::instrument::ResolvedLayer {
                                settings: cat_smf_editor::instrument_layers::Layer {
                                    id,
                                    ..Default::default()
                                },
                                instrument: None,
                            })
                            .collect(),
                        effects: vec![],
                    };
                    crate::player::selection::store_track(&mut source_app, source, &patch).unwrap();
                }
                let mut music = source_app.loaded.unwrap().music;
                let lane = grid::add_lane(&mut music, None).unwrap();
                grid::place(&mut music, None, lane, 2, Some(PatternOrScene::Pattern(id))).unwrap();
                let midi = smf_io::encode(&music).unwrap();
                std::fs::write(&path, &midi).unwrap();
                let mut config = Store::open(root.path().join("status.json"));
                let session = config.session.as_mut().unwrap();
                session.track = source + 1;
                session.note_mode = note_mode;
                session.edited_pattern = edited.then_some(id);
                config.persist();

                // Both last-document restore and an explicit startup file select Song.
                for mid in [None, Some(path.clone())] {
                    let mut app = boot(root.path(), mid);
                    let active = app.playback.active.as_ref().unwrap_or_else(|| {
                        panic!(
                            "drum={drum}, notes={note_mode}, edited={edited}: {:?}",
                            app.load_error
                        )
                    });
                    assert_eq!(active.mode, playback::Mode::Arrangement(None));
                    let notes = &active
                        .sources
                        .iter()
                        .find(|(t, _)| *t == source)
                        .unwrap()
                        .1
                        .notes;
                    assert!(!notes.is_empty());
                    assert!(notes.iter().all(|n| n.start >= 2.0));
                    assert!(active.sources.iter().all(|(t, _)| *t == source));
                    app.audition.set_test_seconds(0.1);
                    assert!(!app.startup_pending(&egui::Context::default()));
                    assert_eq!(
                        app.playback.active.as_ref().unwrap().mode,
                        playback::Mode::Arrangement(None)
                    );
                    assert_eq!(app.audition.seconds(), Some(0.1));
                    assert!(crate::view::patterns::mode::song_window(&app));
                    assert!(app.note_mode);
                    let ctx = egui::Context::default();
                    // The keyboard waits for the main window's first frame.
                    let _ = ctx.run(egui::RawInput::default(), |_| {});
                    let mut input = egui::RawInput {
                        viewport_id: crate::view::patterns::workspace::Pane::Song.viewport(),
                        ..Default::default()
                    };
                    input.viewports.insert(
                        crate::view::patterns::workspace::Pane::Song.viewport(),
                        egui::ViewportInfo {
                            outer_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(800.0, 600.0),
                            )),
                            focused: Some(true),
                            ..Default::default()
                        },
                    );
                    let _ = ctx.run(input, |ctx| {
                        crate::view::presentation::windows::shown(
                            ctx,
                            crate::view::patterns::workspace::Pane::Song.viewport(),
                        );
                        crate::view::recording::show(&mut app, ctx)
                    });
                    assert!(app.patterns.recording.is_open());
                    assert_eq!(
                        app.playback.active.as_ref().unwrap().mode,
                        playback::Mode::Arrangement(None)
                    );
                    assert_eq!(app.audition.seconds(), Some(0.1));
                    assert!(app.load_error.is_none(), "{:?}", app.load_error);
                    assert!(!app.saved.dirty());
                    assert_eq!(std::fs::read(&path).unwrap(), midi);
                }
            }
        }
    }
}

#[test]
fn unplaced_patterns_start_an_empty_song_instead_of_unrelated_tracks() {
    let (root, path) = fixture();
    let mut music = smf_io::load(&path).unwrap();
    let source = operations::create_source(&mut music).unwrap();
    operations::create(&mut music, source, None).unwrap();
    assert!(!Catalog::read(&music).unwrap().patterns.is_empty());
    std::fs::write(&path, smf_io::encode(&music).unwrap()).unwrap();
    let app = boot(root.path(), Some(path));
    assert_eq!(app.playback.last, playback::Mode::Arrangement(None));
    assert!(app.playback.active.as_ref().is_none_or(|active| active
        .sources
        .iter()
        .all(|(_, sequence)| sequence.notes.is_empty())));
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
}
