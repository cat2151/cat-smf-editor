use super::*;
use crate::{
    play::{playback::Mode, transport},
    view::{commands, palette::tests::press},
};
use cat_smf_editor::{
    edit,
    model::{Music, Track},
    score::{
        drums,
        instrument::{Patch, ResolvedLayer},
        pattern,
        scene::arrangement::{self, PatternOrScene},
    },
};
use eframe::egui::{self, Key};

fn song() -> (App, usize, u64) {
    let mut a = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    });
    let settings = drums::Settings::default();
    let patch = Patch {
        layers: settings
            .rows
            .iter()
            .map(|r| ResolvedLayer {
                settings: cat_smf_editor::instrument_layers::Layer {
                    id: r.layer,
                    ..Default::default()
                },
                instrument: None,
            })
            .collect(),
        drums: Some(settings),
        effects: vec![],
    };
    library::initialize_with(&mut a, false, patch).unwrap();
    let saw = a.patterns.selected.unwrap();
    a.edit_music(|m| {
        for (tick, status) in [(0, 0x90), (480, 0x80)] {
            edit::insert(
                &mut m.tracks[1],
                cat_smf_editor::model::Event {
                    tick,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![60, 90],
                },
            );
        }
        Ok(())
    })
    .unwrap();
    a.select_track(0);
    a.edit_tracks(|m| {
        let lane = arrangement::add_lane(m, None)?;
        arrangement::place(m, None, lane, 1, Some(PatternOrScene::Pattern(saw)))?;
        arrangement::place(m, None, lane, 2, Some(PatternOrScene::Pattern(saw)))
    })
    .unwrap();
    (a, 2, saw)
}

#[test]
#[ignore = "requires FEEDBACK_SMF_PATH, PATTERN_NATIVE_WORK_DIR and installed Surge; offline only"]
fn acceptance_native_current_smf_notes_drums_pa() {
    let source = std::path::PathBuf::from(std::env::var_os("FEEDBACK_SMF_PATH").unwrap());
    let original = std::fs::read(&source).unwrap();
    let work = tempfile::tempdir_in(std::env::var_os("PATTERN_NATIVE_WORK_DIR").unwrap()).unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let _roots = crate::player::patch_roots::Assumed::with_table(
        serde_json::from_slice(
            &std::fs::read(
                cat_smf_editor::storage_paths::cache_directory()
                    .unwrap()
                    .join("patch-roots-v1.json"),
            )
            .unwrap(),
        )
        .unwrap(),
    );
    let mut a = crate::view::tests::app(edit::tracks::new_song());
    a.config = crate::store::config::Store::open(work.path().join("status.json"));
    a.state_cache =
        cat_smf_editor::score::state_cache::Cache::new(cat_smf_editor::storage_paths::directory());
    a.saved = crate::store::saved::Document::new(Some(work.path().into()));
    a.player_selection.executable = Some(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../cat-plugin-player/target/release/cat-plugin-player.exe"),
    );
    a.load_mid_unchecked(source.clone());
    assert!(a.load_error.is_none(), "{:?}", a.load_error);
    let c = egui::Context::default();
    let key = |a: &mut App, k| {
        let _ = c.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: k,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| commands::route(a, ctx),
        );
    };
    a.select_track(0);
    key(&mut a, Key::P);
    key(&mut a, Key::A);
    let p = a.playback.pending.take().unwrap();
    assert_eq!(
        p.parts.iter().map(|(_, _, s)| s.notes.len()).sum::<usize>(),
        6
    );
    assert_eq!(p.active.end, 4.);
    crate::play::audition::offline::native_probe::verify(p);
    let drum = a
        .loaded
        .as_ref()
        .unwrap()
        .patches
        .iter()
        .position(|p| p.drums.is_some())
        .unwrap();
    a.select_track(drum);
    track_selected(&mut a);
    key(&mut a, Key::I);
    assert!(
        crate::view::drums::is_open(&a),
        "{:?} / {:?}",
        a.load_error,
        a.patterns.error
    );
    key(&mut a, Key::Enter);
    assert!(drums::active(
        &a.loaded.as_ref().unwrap().visible().tracks[drum],
        480,
        0,
        0
    ));
    let id = a.patterns.editing.as_ref().unwrap().pattern;
    crate::play::audition::offline::native_probe::verify(preview::prepare(&a, id).unwrap());
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn acceptance_song_pa_plays_placements_once_and_rebuilds_seek() {
    let (mut a, _, _) = song();
    let c = egui::Context::default();
    press(&mut a, &c, Key::P);
    press(&mut a, &c, Key::A);
    let p = a.playback.pending.as_ref().expect("p/a prepares Song");
    let voiced = p
        .parts
        .iter()
        .filter(|(_, _, s)| !s.notes.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(voiced.len(), 1);
    assert_eq!(voiced[0].0, 1);
    assert_eq!(
        voiced[0]
            .2
            .notes
            .iter()
            .map(|n| n.start)
            .collect::<Vec<_>>(),
        vec![0., 2.]
    );
    let seek = crate::play::selection_playback::skip(
        &a,
        transport::prepare(&a, Mode::AllTracks).unwrap().active,
        2.,
    )
    .unwrap();
    assert_eq!(seek.parts[0].2.notes.len(), 1);
    assert_eq!(seek.active.start_seconds, 2.);
    crate::cursor::moves::at_tick(&mut a, 1920);
    let from_measure = transport::prepare(&a, Mode::AllTracksFromMeasure).unwrap();
    assert_eq!(from_measure.active.mode, Mode::Arrangement(None));
    assert_eq!(from_measure.parts[0].2.notes.len(), 1);
    let repeated = transport::repeat::prepare(&a, (1920, 3840)).unwrap();
    assert_eq!(repeated.active.start_seconds, 2.);
    assert_eq!(repeated.parts[0].2.notes.len(), 1);
}

#[test]
fn acceptance_empty_drum_notes_i_and_palette_open_owned_pattern_grid() {
    for palette in [false, true] {
        let (mut a, drum, saw) = song();
        a.select_track(drum);
        track_selected(&mut a);
        let c = egui::Context::default();
        let before = a.loaded.as_ref().unwrap().music.clone();
        let owner = pattern::Catalog::read(&before)
            .unwrap()
            .tracks
            .iter()
            .find_map(|(&id, &t)| (t == drum).then_some(id))
            .unwrap();
        assert_eq!(a.patterns.container, Some(owner));
        assert!(
            a.patterns.selected.is_none(),
            "empty Drums must not select a Saw pattern"
        );
        if palette {
            let mut query = commands::SearchQuery::default();
            query.update("drum");
            let cmd = commands::candidates(&a, &c)
                .into_iter()
                .find(|cmd| cmd.id == "drums" && cmd.matches_query(&query))
                .expect("Drum sequencer in palette");
            assert!(commands::execute(&mut a, &c, &cmd));
        } else {
            press(&mut a, &c, Key::I);
        }
        assert!(crate::view::drums::is_open(&a), "i opens Drum sequencer");
        assert!(!a.mml_input.open);
        let context = a.patterns.editing.as_ref().unwrap().clone();
        assert_eq!(context.track, drum);
        assert_ne!(context.pattern, saw);
        assert!(a.can_edit_notes(drum));
        assert_eq!(commands::owner(&a, &c), commands::Screen::Drums);
        press(&mut a, &c, Key::Enter);
        let music = &a.loaded.as_ref().unwrap().music;
        assert_eq!(music.tracks[1].events.len(), before.tracks[1].events.len());
        assert!(drums::active(&music.tracks[drum], 480, 0, 0));
        assert_eq!(
            a.patterns.editing.as_ref().unwrap().pattern,
            context.pattern
        );
        press(&mut a, &c, Key::Escape);
        a.undo_music().unwrap();
        assert!(!drums::active(
            &a.loaded.as_ref().unwrap().music.tracks[drum],
            480,
            0,
            0
        ));
        // Reopening an existing body must not create another pattern.
        let count = pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns
            .len();
        press(&mut a, &c, Key::I);
        assert!(crate::view::drums::is_open(&a));
        assert!(!a.mml_input.open);
        assert_eq!(
            pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
                .unwrap()
                .patterns
                .len(),
            count
        );
    }
}
#[test]
fn adding_a_drum_pattern_opens_the_drum_sequencer() {
    let (mut a, drum, _) = song();
    let c = egui::Context::default();
    a.patterns.container = pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .tracks
        .iter()
        .find_map(|(&id, &t)| (t == drum).then_some(id));
    library::add(&mut a, &c);
    assert!(a.patterns.confirming_creation());
    press(&mut a, &c, Key::Enter);
    assert_eq!(a.track, drum);
    assert!(crate::view::drums::is_open(&a));
    assert!(!a.mml_input.open);
}
#[test]
fn song_cell_auditions_and_both_entries_open_the_drum_editor() {
    let (mut a, drum, _) = song();
    let c = super::workspace_tests::context();
    a.patterns.container = pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .tracks
        .iter()
        .find_map(|(&id, &t)| (t == drum).then_some(id));
    library::add(&mut a, &c);
    press(&mut a, &c, Key::Enter);
    let id = a.patterns.selected.unwrap();
    press(&mut a, &c, Key::Escape);
    a.edit_tracks(|m| {
        let lane = arrangement::add_lane(m, None)?;
        arrangement::place(m, None, lane, 3, Some(PatternOrScene::Pattern(id)))
    })
    .unwrap();
    a.select_track(0);
    super::workspace::enable(&mut a);
    let song = super::workspace::Pane::Song;
    super::workspace_tests::row_cell(&mut a, &c, song, 1, 3);
    assert!(crate::view::drums::is_open(&a));
    assert!(a.playback.pending.is_some() || a.playback.active.is_some());
    super::workspace_tests::candidate(&mut a, &c, song, PatternOrScene::Pattern(id));
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert!(crate::view::drums::is_open(&a), "{:?}", a.load_error);
    assert!(!a.mml_input.open);
}
