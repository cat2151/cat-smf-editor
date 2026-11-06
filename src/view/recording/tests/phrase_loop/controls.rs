use super::*;

#[test]
fn the_measure_from_the_first_key_repeats_until_the_switch_turns_it_off() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let start = start_loop(&mut a, &ctx);
    assert_eq!(
        due(&a, start + measure(&a) * 3 + Duration::from_millis(1)),
        vec![word(60, 80)]
    );
    session_queue_clear(&mut a);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(session(&a).sound().looper, Looper::Off));
    assert_eq!(queued(&a), vec![word(60, 0)]);
    assert!(due(&a, start + measure(&a) * 9).is_empty());
}

#[test]
fn the_next_key_over_the_phrase_replaces_it() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let start = start_loop(&mut a, &ctx);
    // The next lap has begun: its Note On for 60 is out, its Note Off is not.
    assert_eq!(
        due(&a, start + measure(&a) * 3 + Duration::from_millis(1)),
        vec![word(60, 80)]
    );
    session_queue_clear(&mut a);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    // The old phrase lets go before the key of the same pitch sounds, at the sweep's second velocity.
    assert_eq!(queued(&a), vec![word(60, 0), word(60, 86)]);
    assert!(due(&a, start + measure(&a) * 9).is_empty());
    let Looper::Capturing { origin, .. } = session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    // Held a quarter, so its Note Off is not due with the Note On however fast the frames run.
    let quarter = measure(&a) / 4;
    rewind(&mut a, quarter);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    let length = measure(&a);
    rewind(&mut a, length);
    let restart = session(&a).clock + origin;
    frame(&mut a, &ctx, vec![], true);
    assert!(phrase::looping(session(&a)));
    // The new phrase repeats the key at the velocity it was played with.
    assert_eq!(
        due(&a, restart + measure(&a) + Duration::from_millis(1)),
        vec![word(60, 86)]
    );
    session_queue_clear(&mut a);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(session(&a).sound().looper, Looper::Off));
    assert_eq!(queued(&a), vec![word(60, 0)]);
}

#[test]
fn a_patch_edit_reaches_the_instrument_while_the_phrase_keeps_repeating() {
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let mut patch = a.loaded.as_ref().unwrap().patches[track].clone();
    crate::view::layers::add(&mut patch).unwrap();
    crate::view::layers::add(&mut patch).unwrap();
    crate::player::selection::store_track(&mut a, track, &patch).unwrap();
    frame(&mut a, &ctx, vec![], true);
    assert!(phrase::looping(session(&a)));
    assert!(a.audition.live_patches.contains(&(track, patch.clone())));
    assert_eq!(
        phrase::display(session(&a)),
        (format!("{} layers", patch.layers.len()), true)
    );
}

#[test]
fn ctrl_t_on_a_layered_container_opens_its_layer_editor() {
    let (mut a, track) = container();
    let mut patch = a.loaded.as_ref().unwrap().patches[track].clone();
    crate::view::layers::add(&mut patch).unwrap();
    crate::view::layers::add(&mut patch).unwrap();
    crate::player::selection::store_track(&mut a, track, &patch).unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    a.track = 0;
    press(&mut a, &ctx, Key::T, Modifiers::CTRL);
    assert!(
        !a.layer_editor.open,
        "nothing to browse until the phrase repeats"
    );
    start_loop(&mut a, &ctx);
    press(&mut a, &ctx, Key::T, Modifiers::CTRL);
    assert!(a.layer_editor.open);
    assert_eq!(a.track, track);
    assert!(phrase::looping(session(&a)));
}

#[test]
fn ctrl_t_browses_the_playing_containers_patch() {
    let (mut a, track) = container();
    a.player_selection.executable = Some(std::env::current_exe().unwrap());
    a.player_selection.launch = Some(refuse);
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    a.track = 0;
    press(&mut a, &ctx, Key::T, Modifiers::CTRL);
    assert!(
        LAUNCHED.load(Ordering::Relaxed),
        "{:?}",
        a.player_selection.error
    );
    assert_eq!(a.track, track);
    assert!(!a.layer_editor.open);
    assert!(phrase::looping(session(&a)));
}

#[test]
fn the_arrow_goes_round_once_while_the_measure_is_captured() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert_eq!(phrase::progress(session(&a)), None);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    assert!(phrase::progress(session(&a)).unwrap() < 0.05);
    let half = measure(&a) / 2;
    rewind(&mut a, half);
    let shown = phrase::progress(session(&a)).unwrap();
    assert!((0.45..0.55).contains(&shown), "{shown}");
    rewind(&mut a, half);
    frame(&mut a, &ctx, vec![], true);
    assert!(phrase::looping(session(&a)));
    assert_eq!(phrase::progress(session(&a)), None);
}

#[test]
fn closing_the_app_with_the_switch_on_reopens_the_keyboard_waiting() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(a.config.phrase_loop);
    assert!(close_for_exit(&mut a));
    assert!(a.config.phrase_loop, "the app's exit keeps the switch");

    // The next start: the keyboard opens without a key, its switch waiting.
    resume(&mut a);
    frame(&mut a, &ctx, vec![], true);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert!(matches!(session(&a).sound().looper, Looper::Waiting));

    // Closing the keyboard itself leaves the next start without it.
    press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.recording.is_open());
    assert!(!a.config.phrase_loop);
    resume(&mut a);
    frame(&mut a, &ctx, vec![], true);
    assert!(!a.patterns.recording.is_open());
}

#[test]
fn the_switch_off_is_kept_for_the_next_start() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(!a.config.phrase_loop);
    assert!(close_for_exit(&mut a));
    resume(&mut a);
    frame(&mut a, &ctx, vec![], true);
    assert!(!a.patterns.recording.is_open());
}

#[test]
fn the_shown_velocity_follows_the_repeat() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    let Looper::Capturing { origin, .. } = session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    let half = measure(&a) / 2;
    rewind(&mut a, half);
    frame(
        &mut a,
        &ctx,
        vec![
            key(Key::Z, false, Modifiers::NONE),
            key(Key::X, true, Modifiers::NONE),
            key(Key::X, false, Modifiers::NONE),
        ],
        true,
    );
    let bar = measure(&a);
    rewind(&mut a, bar);
    frame(&mut a, &ctx, vec![], true);
    assert!(phrase::looping(session(&a)));
    let lap = origin + measure(&a);
    let quarter = measure(&a) / 4;
    let shown = |at| phrase::replayed(session(&a), at);
    assert_eq!(shown(lap - quarter), None);
    assert_eq!(shown(lap + quarter), Some(80));
    assert_eq!(shown(lap + half + quarter), Some(86));
    assert_eq!(shown(lap + measure(&a) * 5 + quarter), Some(80));
}

#[cfg(windows)]
#[test]
fn a_container_named_after_its_patch_follows_a_browsed_patch_and_a_given_name_stays() {
    let (mut a, first) = container();
    for _ in 0..2 {
        a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
            .unwrap();
    }
    let (second, named) = (first + 1, first + 2);
    a.edit_tracks(|m| edit::tracks::rename(&mut m.tracks[named], "Bass"))
        .unwrap();
    a.track = first;
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let selected = |a: &App| session(a).sounds[session(a).selected].name.clone();
    assert_eq!(selected(&a), "Init Saw");

    browse_to(&mut a, first, "Lead");
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(selected(&a), "Lead");
    assert_eq!(phrase::display(session(&a)).0, "Lead");
    // Named after its patch again, it follows the next one too.
    browse_to(&mut a, first, "Pad");
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(selected(&a), "Pad");

    // The same patch on the other container is numbered so the chooser tells them apart.
    browse_to(&mut a, second, "Pad");
    assert_eq!(track_name(&a, second).as_deref(), Some("Pad 2"));
    browse_to(&mut a, named, "Lead");
    assert_eq!(track_name(&a, named).as_deref(), Some("Bass"));
}

#[test]
fn the_loop_a_phrase_writes_is_an_undo_step() {
    use cat_smf_editor::score::pattern::recording::looped;
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    edit_loop(&mut a, 64);
    let tempo = recorder(&a).settings().tempo;
    let edited = looped(&a.loaded.as_ref().unwrap().music, id, tempo).unwrap();
    // The next phrase overwrites the edited Loop; undo brings the edited one back.
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    let length = measure(&a);
    rewind(&mut a, length);
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(looping_pattern(&a), id);
    let music = |a: &App| a.loaded.as_ref().unwrap().music.clone();
    assert_ne!(looped(&music(&a), id, tempo).unwrap(), edited);
    a.undo_music().unwrap();
    assert_eq!(looped(&music(&a), id, tempo).unwrap(), edited);
}

#[test]
fn placing_the_loop_in_the_song_places_an_ordinary_pattern_of_its_body() {
    use cat_smf_editor::score::{
        pattern::{operations, Catalog, Kind},
        scene::{self, arrangement::PatternOrScene},
    };
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    let place = |a: &mut App, measure| {
        a.patterns.workspace.selection_mut(Pane::Song).measure = measure;
        crate::view::patterns::workspace::place(a, Pane::Song, Some(PatternOrScene::Pattern(id)))
            .unwrap();
    };
    place(&mut a, 1);
    frame(&mut a, &ctx, vec![], true);
    assert!(phrase::looping(session(&a)));
    let music = &a.loaded.as_ref().unwrap().music;
    let normal = operations::same_normal(music, id).unwrap().unwrap();
    let c = Catalog::read(music).unwrap();
    assert_eq!(c.patterns[&id].kind, Kind::Loop);
    let song: Vec<_> = scene::read(music)
        .unwrap()
        .song
        .iter()
        .map(|p| p.part)
        .collect();
    assert_eq!(song, vec![PatternOrScene::Pattern(normal)]);
    // Placed again, the Loop reuses that pattern.
    let count = c.patterns.len();
    place(&mut a, 2);
    let music = &a.loaded.as_ref().unwrap().music;
    assert_eq!(Catalog::read(music).unwrap().patterns.len(), count);
    let song: Vec<_> = scene::read(music)
        .unwrap()
        .song
        .iter()
        .map(|p| p.part)
        .collect();
    assert_eq!(song, vec![PatternOrScene::Pattern(normal); 2]);
}
