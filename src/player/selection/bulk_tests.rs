use crate::player::bulk::*;
use crate::{
    player::selection::tests::{music, settings, signature},
    view::tests::app,
};
use cat_smf_editor::model::Music;

pub(crate) fn identical() -> Music {
    let mut music = music();
    music.tracks[0]
        .events
        .retain(|event| !cat_smf_editor::score::note_view::note_on(event));
    music.tracks[1].events.insert(
        0,
        cat_smf_editor::model::Event {
            delta: 0,
            tick: 0,
            status: 0xc3,
            meta_type: 0,
            data: vec![19],
        },
    );
    let track = music.tracks[1].clone();
    music.tracks.push(track);
    music
}

#[test]
fn prompt_requires_multiple_note_tracks_with_known_identical_programs() {
    let mut song = identical();
    assert!(same_program(&song));
    song.tracks[2]
        .events
        .retain(|e| e.status & 0xf0 != 0xc0 && !(e.status == 0xff && e.meta_type == 8));
    assert!(!same_program(&song));
    assert!(!same_program(&music()));
}

#[test]
fn t_on_conductor_goes_bulk_when_auto_and_errors_when_current() {
    let mut app = app(identical());
    request(&mut app);
    assert!(!app.player_selection.no_notes_error);
    assert!(app
        .player_selection
        .error
        .as_deref()
        .unwrap()
        .contains("executable"));
    app.player_selection.error = None;
    app.config.program_target = crate::store::config::ProgramTarget::Current;
    request(&mut app);
    assert!(app.player_selection.no_notes_error);
    assert!(app
        .player_selection
        .error
        .as_deref()
        .unwrap()
        .contains("ノートがありません"));
}

#[test]
fn bulk_import_changes_instruments_and_keeps_conductor_effects_and_notes() {
    let mut app = app(identical());
    let before = app.loaded.as_ref().unwrap().music.clone();
    let settings = settings();
    accept(&mut app, &settings).unwrap();
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(loaded.patches[0].layers[0].instrument, None);
    for track in [1, 2] {
        assert_eq!(
            loaded.patches[track].layers[0].instrument,
            settings.layers[0].instrument
        );
        assert!(loaded.patches[track].effects.is_empty());
        assert_eq!(
            signature(&loaded.music.tracks[track]),
            signature(&before.tracks[track])
        );
    }
}
