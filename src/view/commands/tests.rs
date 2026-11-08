use super::descriptions::tests::displayed;
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
    let cmd = defs.iter().find(|c| c.id == "track.volume-down").unwrap();
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
    assert_eq!(owner(&a, &c), Screen::Main);
    assert!(candidates(&a, &c).iter().any(|c| c.screen == Screen::Play));
}
#[test]
fn shared_key_and_palette_use_same_layer_target_and_undo_condition() {
    let mut a = app();
    a.track = 1;
    let c = egui::Context::default();
    assert!(!candidates(&a, &c).iter().any(|c| c.id == "app.undo"));
    crate::view::layers::open(&mut a);
    let cmd = candidates(&a, &c)
        .into_iter()
        .find(|c| c.id == "layer.mute")
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
        .find(|c| c.id == "track.volume-down")
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
        Screen::Backup,
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
            .filter(|c| c.id.starts_with("track.entry-"))
            .count(),
        3
    );
    assert!(!definitions(&a, Screen::Effects)
        .iter()
        .any(|c| c.id == "effect.bypass"));
}
fn in_window<R>(
    a: &mut App,
    c: &egui::Context,
    viewport: egui::ViewportId,
    mut f: impl FnMut(&mut App, &egui::Context) -> R,
) -> R {
    let mut input = egui::RawInput {
        viewport_id: viewport,
        ..Default::default()
    };
    input.viewports.insert(
        viewport,
        egui::ViewportInfo {
            focused: Some(true),
            ..Default::default()
        },
    );
    let mut out = None;
    let _ = c.run(input, |c| out = Some(f(a, c)));
    out.unwrap()
}
fn offered(a: &App, c: &egui::Context) -> Vec<(Screen, String)> {
    candidates(a, c)
        .into_iter()
        .map(|c| (c.screen, c.id))
        .collect()
}
fn run_id(a: &mut App, c: &egui::Context, screen: Screen, id: &str) -> bool {
    let command = candidates(a, c)
        .into_iter()
        .find(|x| x.screen == screen && x.id == id)
        .unwrap_or_else(|| panic!("{id}"));
    execute(a, c, &command)
}
#[test]
fn global_commands_have_no_screen_label_and_are_offered_everywhere_but_app_dialogs() {
    let mut a = app();
    let c = egui::Context::default();
    let globals = |a: &App| {
        displayed(candidates(a, &c))
            .into_iter()
            .filter(|c| c.screen == Screen::Global)
            .map(|c| {
                assert_eq!(c.label(), c.name);
                c.id
            })
            .collect::<Vec<_>>()
    };
    let all = globals(&a);
    assert!(all.iter().any(|id| id == "app.quit"));
    a.piano_roll.open = true;
    assert_eq!(owner(&a, &c), Screen::Timeline);
    assert_eq!(globals(&a), all);
    a.piano_roll.open = false;
    a.edit_menu.open = true;
    assert_eq!(owner(&a, &c), Screen::Edit);
    assert_eq!(globals(&a), all);
    // 他の画面から実行しても、通常の終了処理（保存確認）へ進み、その間は Global を出さない。
    assert!(run_id(&mut a, &c, Screen::Global, "app.quit"));
    assert!(!a.config.navigation.close_approved);
    assert_eq!(owner(&a, &c), Screen::Confirm);
    assert!(globals(&a).is_empty());
}
#[test]
fn app_quit_is_offered_in_pane_and_help_windows_and_closes_the_app() {
    let mut a = app();
    let c = egui::Context::default();
    crate::view::patterns::edit_tests::add_container(&mut a);
    let song = crate::view::patterns::workspace::Pane::Song.viewport();
    for viewport in [song, crate::view::help::viewport()] {
        let listed = in_window(&mut a, &c, viewport, |a, c| offered(a, c));
        assert!(
            listed.iter().any(|(_, id)| id == "app.quit"),
            "{viewport:?}"
        );
    }
    in_window(&mut a, &c, song, |a, c| {
        assert_eq!(owner(a, c), Screen::Song);
        assert!(run_id(a, c, Screen::Global, "app.quit"));
    });
    // root から実行したときと同じ保存確認へ進む。
    assert!(!a.config.navigation.close_approved);
    assert_eq!(owner(&a, &c), Screen::Confirm);
}
#[test]
fn every_palette_offers_every_screen_once_and_moves_there_to_run() {
    let mut a = app();
    let c = egui::Context::default();
    crate::view::patterns::edit_tests::add_container(&mut a);
    let song = crate::view::patterns::workspace::Pane::Song.viewport();
    let listed = in_window(&mut a, &c, song, |a, c| {
        assert_eq!(owner(a, c), Screen::Song);
        let ids: Vec<_> = candidates(a, c).into_iter().map(|c| c.id).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
        offered(a, c)
    });
    let has = |screen, id: &str| listed.iter().any(|(s, i)| *s == screen && i == id);
    for (screen, id) in [
        (Screen::Main, "track.add"),
        (Screen::Layers, "layer.next"),
        (Screen::Song, "app.stop"),
        (Screen::Global, "pattern.create"),
    ] {
        assert!(has(screen, id), "{id}");
    }
    // 同じ機能は今いる画面のものだけを出す。
    assert!(!has(Screen::Main, "app.stop"));
    // 開いてすぐ閉じるだけの、他の画面の「閉じる」は出さない。
    assert!(!has(Screen::Timeline, "timeline.close") && !has(Screen::Layers, "layer.close"));
    // pane窓からroot側の画面の機能を選ぶと、root窓へ移って実行し、そこに留まる。
    a.piano_roll.open = true;
    assert!(in_window(&mut a, &c, song, |a, c| run_id(
        a,
        c,
        Screen::Main,
        "layer.open"
    )));
    assert!(!a.piano_roll.open);
    assert!(a.patterns.root_focus);
    assert_eq!(owner(&a, &c), Screen::Layers);
    // 今の画面に重ねて開く組は、下の画面を閉じない。
    a.layer_editor.open = false;
    a.piano_roll.open = true;
    assert_eq!(owner(&a, &c), Screen::Timeline);
    assert!(run_id(&mut a, &c, Screen::Settings, "meta.all"));
    assert!(a.piano_roll.open);
}
#[test]
fn input_windows_offer_only_themselves_and_are_not_left_from_a_pane() {
    let mut a = app();
    let c = egui::Context::default();
    a.mml_input.open = true;
    assert_eq!(owner(&a, &c), Screen::Mml);
    assert!(candidates(&a, &c)
        .iter()
        .all(|x| matches!(x.screen, Screen::Mml | Screen::Global)));
    crate::view::patterns::edit_tests::add_container(&mut a);
    let song = crate::view::patterns::workspace::Pane::Song.viewport();
    assert!(!in_window(&mut a, &c, song, |a, c| run_id(
        a,
        c,
        Screen::Main,
        "layer.open"
    )));
    assert!(a.mml_input.open);
    assert!(!a.layer_editor.open);
}

/// A focused Song pane holds no input, so a palette command of the event list still runs.
#[test]
fn palette_leaves_a_focused_song_pane() {
    use crate::view::dock::{focus, ViewKind};
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    crate::view::patterns::library::add_container_with(&mut a, None).unwrap();
    a.layout.spec = crate::store::layout::parse(
        r#"
[[window]]
column = [{ view = "song" }, { view = "step_sequencer" }]
[window.monitor_placement]
monitor = "center"
left_percent = 0
top_percent = 0
width_percent = 100
height_percent = 100
"#,
    )
    .unwrap();
    focus::set(&mut a, ViewKind::Song);
    let c = egui::Context::default();
    assert_eq!(owner(&a, &c), Screen::Song);
    assert!(
        run_id(&mut a, &c, Screen::Main, "drum.open"),
        "{:?}",
        a.load_error
    );
    assert!(crate::view::drums::is_open(&a));
    assert_eq!(a.load_error, None);
}
