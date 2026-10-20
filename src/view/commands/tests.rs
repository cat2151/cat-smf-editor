use super::*;
use cat_smf_editor::model::{Music, Track};
fn app() -> App {
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }; 3],
    })
}
fn press(a: &mut App, c: &egui::Context, key: Key, m: Modifiers) {
    let _ = c.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: m,
            }],
            ..Default::default()
        },
        |c| route(a, c),
    );
}
#[test]
fn closed_mixer_enumeration_is_pure_and_execution_uses_current_track() {
    let mut a = app();
    a.track = 2;
    let c = egui::Context::default();
    let gain = a.audition.gain(2);
    let defs = candidates(&a, &c);
    let cmd = defs.iter().find(|c| c.id == "volume-down").unwrap();
    assert!(!a.playback.mixer_open);
    assert_eq!(a.audition.gain(2), gain);
    assert!(execute(&mut a, &c, cmd));
    assert!(a.playback.mixer_open);
    assert!(!a.playback.mixer_master);
    assert_eq!(a.playback.mixer_track, 2);
    assert!(a.audition.gain(2) < gain);
    assert_eq!(a.audition.gain(1), 1.0);
}
#[test]
fn closed_keys_are_not_direct_and_dialogs_block_reachability() {
    let mut a = app();
    let c = egui::Context::default();
    press(&mut a, &c, Key::J, Modifiers::NONE);
    assert!(!a.playback.mixer_open);
    a.config.navigation.departure = Some(crate::view::files::Action::New);
    assert!(candidates(&a, &c)
        .iter()
        .all(|c| c.screen == Screen::Confirm));
    a.config.navigation.departure = None;
    a.help_open = true;
    assert!(candidates(&a, &c).iter().all(|c| c.screen == Screen::Help));
}
#[test]
fn shared_key_and_palette_use_same_layer_target_and_undo_condition() {
    let mut a = app();
    a.track = 1;
    let c = egui::Context::default();
    assert!(!candidates(&a, &c).iter().any(|c| c.id == "undo"));
    crate::view::layers::open(&mut a);
    let cmd = candidates(&a, &c)
        .into_iter()
        .find(|c| c.id == "layer-mute")
        .unwrap();
    assert!(execute(&mut a, &c, &cmd));
    assert!(
        a.loaded.as_ref().unwrap().patches[1].layers[0]
            .settings
            .mute
    );
    press(&mut a, &c, Key::M, Modifiers::NONE);
    assert!(
        !a.loaded.as_ref().unwrap().patches[1].layers[0]
            .settings
            .mute
    );
    assert!(a.loaded.as_ref().unwrap().patches[0]
        .layers
        .iter()
        .all(|l| !l.settings.mute));
}

#[test]
fn stale_candidate_never_substitutes_another_track_or_generation() {
    let mut a = app();
    let c = egui::Context::default();
    let cmd = candidates(&a, &c)
        .into_iter()
        .find(|c| c.id == "volume-down")
        .unwrap();
    a.track = 1;
    assert!(!execute(&mut a, &c, &cmd));
    assert!(!a.playback.mixer_open);
    a.track = 0;
    a.generation += 1;
    assert!(!execute(&mut a, &c, &cmd));
    assert_eq!(a.audition.gain(0), 1.0);
}

#[test]
fn owner_definitions_have_unique_bindings_and_dynamic_track_choices() {
    let a = app();
    for screen in [
        Screen::Main,
        Screen::Settings,
        Screen::Program,
        Screen::Edit,
        Screen::Go,
        Screen::Tracks,
        Screen::Split,
        Screen::Play,
        Screen::Mixer,
        Screen::Layers,
        Screen::Effects,
        Screen::Recent,
        Screen::Confirm,
        Screen::Note,
        Screen::Lyrics,
        Screen::Mml,
        Screen::Trill,
        Screen::Timeline,
        Screen::Analysis,
    ] {
        let defs = definitions(&a, screen);
        let mut keys = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for def in defs {
            assert!(ids.insert(def.id.clone()), "{screen:?}: {}", def.id);
            for key in def.keys {
                assert!(!keys.contains(&key), "duplicate key {screen:?} {key:?}");
                keys.push(key);
            }
        }
    }
    assert_eq!(
        definitions(&a, Screen::Tracks)
            .iter()
            .filter(|c| c.id.starts_with("track-"))
            .count(),
        3
    );
    assert!(!definitions(&a, Screen::Effects)
        .iter()
        .any(|c| c.id == "effect-bypass"));
}
