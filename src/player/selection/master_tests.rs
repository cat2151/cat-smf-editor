use super::*;
use cat_smf_editor::smf_io;
use eframe::egui;

fn app() -> App {
    let mut music = super::tests::music();
    music.format = 1;
    crate::view::tests::app(music)
}

#[test]
fn master_store_updates_only_conductor_metadata_and_preserves_track_settings() {
    let mut app = app();
    let original = app.loaded.as_ref().unwrap().music.clone();
    let settings = cat_smf_editor::master_effects::Settings {
        effects: super::tests::settings().effects,
        ..Default::default()
    };
    store_master(&mut app, settings.clone()).unwrap();
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(loaded.patches[0], Settings::default());
    assert_eq!(app.audition.master().effects, settings.effects);
    assert_eq!(
        cat_smf_editor::master_references::read(
            &loaded.music,
            &mut crate::player::patch_roots::Assumed
        )
        .unwrap(),
        crate::test_support::master_saved(app.audition.master())
    );
    assert_eq!(
        super::tests::signature(&loaded.music.tracks[0]),
        super::tests::signature(&original.tracks[0])
    );
    let returned = cat_smf_editor::ffi::parse(&smf_io::encode(&loaded.music).unwrap()).unwrap();
    assert_eq!(
        cat_smf_editor::master_references::read(
            &returned,
            &mut crate::player::patch_roots::Assumed
        )
        .unwrap(),
        crate::test_support::master_saved(app.audition.master())
    );
}

#[test]
fn mx_selects_master_without_changing_track_and_navigation_keeps_track_gains_separate() {
    let mut app = app();
    crate::play::mixer::open(&mut app);
    assert!(app.playback.mixer_master);
    crate::play::mixer::handle_key(&mut app, egui::Key::J);
    assert_eq!(app.audition.gain(0), 1.0);
    crate::play::mixer::handle_key(&mut app, egui::Key::X);
    assert!(app.effect_editor.is_open());
    assert_eq!(app.track, 0);
    crate::play::mixer::handle_key(&mut app, egui::Key::L);
    assert!(!app.playback.mixer_master);
    crate::play::mixer::handle_key(&mut app, egui::Key::J);
    assert_eq!(
        app.audition.gain(0),
        crate::play::track_volume::db_to_gain(-3)
    );
    crate::play::mixer::handle_key(&mut app, egui::Key::H);
    assert!(app.playback.mixer_master);
}

#[test]
fn chain_remove_drops_one_effect_slot_until_the_chain_is_empty() {
    use crate::view::effects::{open, tests::press, Target};
    let ctx = egui::Context::default();
    let remove = |app: &mut App, target, slot| {
        open(app, target, slot);
        press(app, &ctx, egui::Key::Delete, egui::Modifiers::NONE);
    };
    let mut app = app();
    let chained = super::tests::settings();
    store_track(&mut app, 0, &chained).unwrap();
    store_master(
        &mut app,
        cat_smf_editor::master_effects::Settings {
            effects: chained.effects.clone(),
            ..Default::default()
        },
    )
    .unwrap();

    remove(&mut app, Target::Track(0), 0);
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(loaded.patches[0].effects, chained.effects[1..]);
    assert_eq!(
        loaded.patches[0].layers[0].instrument,
        chained.layers[0].instrument
    );
    assert_eq!(
        cat_smf_editor::instrument_layers::read(
            &loaded.music.tracks[0],
            &mut crate::player::patch_roots::Assumed
        )
        .unwrap(),
        loaded.patches[0].saved().unwrap()
    );
    remove(&mut app, Target::Track(0), 0);
    assert!(app.loaded.as_ref().unwrap().patches[0].effects.is_empty());

    remove(&mut app, Target::Master, 1);
    assert_eq!(app.audition.master().effects, chained.effects[..1]);
    remove(&mut app, Target::Master, 0);
    assert!(app.audition.master().effects.is_empty());
    assert_eq!(
        cat_smf_editor::master_references::read(
            &app.loaded.as_ref().unwrap().music,
            &mut crate::player::patch_roots::Assumed
        )
        .unwrap(),
        crate::test_support::master_saved(app.audition.master())
    );
}
