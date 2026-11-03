use super::*;
use crate::play::audition::live::word;
use phrase::Looper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

fn press(a: &mut App, ctx: &egui::Context, k: Key, m: Modifiers) {
    frame(a, ctx, vec![key(k, true, m), key(k, false, m)], true);
}
/// The phrase words the audio thread would play by `at`.
fn due(a: &App, at: Instant) -> Vec<u32> {
    let mut out = [0; 64];
    let n = session(a).sound().live.phrase.due(at, &mut out);
    out[..n].to_vec()
}
fn measure(a: &App) -> Duration {
    Duration::from_micros(u64::from(recorder(a).settings().tempo) * 4)
}
/// An app whose only container is selected, so the keyboard plays it.
fn container() -> (App, usize) {
    let mut a = app();
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let track = a.loaded.as_ref().unwrap().music.tracks.len() - 1;
    a.track = track;
    (a, track)
}
/// Ctrl+E, then `z` held for a quarter of the measure; one measure later the phrase repeats.
/// Returns where its first lap starts.
fn start_loop(a: &mut App, ctx: &egui::Context) -> Instant {
    assert!(matches!(session(a).sound().looper, Looper::Off));
    press(a, ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(session(a).sound().looper, Looper::Waiting));
    frame(a, ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    let Looper::Capturing { origin, .. } = session(a).sound().looper else {
        panic!("{:?}", session(a).sound().looper)
    };
    let quarter = measure(a) / 4;
    let start = session(a).clock + origin - quarter;
    rewind(a, quarter);
    frame(a, ctx, vec![key(Key::Z, false, Modifiers::NONE)], true);
    assert!(!phrase::looping(session(a)));
    rewind(a, measure(a));
    frame(a, ctx, vec![], true);
    assert!(phrase::looping(session(a)));
    let (on, off) = (word(60, 80), word(60, 0));
    assert_eq!(
        due(a, start + measure(a) * 2 + quarter * 2),
        vec![on, off, on, off, on, off]
    );
    start
}

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

fn queued(a: &App) -> Vec<u32> {
    session(a)
        .queue()
        .unwrap()
        .lock()
        .unwrap()
        .words
        .iter()
        .copied()
        .collect()
}

fn session_queue_clear(a: &mut App) {
    session(a).queue().unwrap().lock().unwrap().words.clear();
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

static LAUNCHED: AtomicBool = AtomicBool::new(false);
fn refuse(_: &std::path::Path, _: &str) -> std::io::Result<Option<std::process::Child>> {
    LAUNCHED.store(true, Ordering::Relaxed);
    Err(std::io::Error::other("no player in tests"))
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

/// Patterns of the song and its Song placements as (measure, pattern).
fn song(a: &App) -> (Vec<u64>, Vec<(u64, u64)>) {
    use cat_smf_editor::score::{pattern::Catalog, scene, scene::arrangement::PatternOrScene};
    let m = &a.loaded.as_ref().unwrap().music;
    let patterns = Catalog::read(m).unwrap().patterns.keys().copied().collect();
    let placed = scene::read(m)
        .unwrap()
        .song
        .iter()
        .map(|p| match p.part {
            PatternOrScene::Pattern(id) => (p.measure, id),
            PatternOrScene::Scene(id) => (p.measure, id),
        })
        .collect();
    (patterns, placed)
}

/// Notes of pattern `id` as (start, end, pitch) from its start, and its bar count.
fn bar(a: &App, id: u64) -> (usize, Vec<(u64, u64, u8)>) {
    use cat_smf_editor::score::{
        note_view::{note_off, note_on},
        pattern::{body, Catalog},
    };
    let m = &a.loaded.as_ref().unwrap().music;
    let p = Catalog::read(m).unwrap().patterns[&id].clone();
    let events = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    let notes = events
        .iter()
        .filter(|e| note_on(e))
        .map(|e| {
            let off = events
                .iter()
                .find(|o| note_off(o) && o.data[0] == e.data[0] && o.tick > e.tick)
                .unwrap();
            (e.tick - p.tick, off.tick - p.tick, e.data[0])
        })
        .collect();
    (p.meters.len(), notes)
}

#[test]
fn ctrl_enter_takes_the_repeating_measure_to_the_start_of_an_empty_song() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let before = song(&a);
    assert!(before.1.is_empty());
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let (patterns, placed) = song(&a);
    assert_eq!(patterns.len(), before.0.len() + 1);
    let id = *patterns.iter().find(|p| !before.0.contains(p)).unwrap();
    let quarter = u64::from(a.loaded.as_ref().unwrap().music.division);
    // The key was held a quarter of wall-clock time, plus the frames' own time.
    let (bars, notes) = bar(&a, id);
    assert_eq!((bars, notes.len(), notes[0].0, notes[0].2), (1, 1, 0, 60));
    assert!(
        (quarter..quarter + quarter / 4).contains(&notes[0].1),
        "{notes:?}"
    );
    assert_eq!(placed, vec![(1, id)]);
    // The keyboard stays open with the phrase stopped, and now plays the new container.
    assert!(!phrase::any_looping(session(&a)));
    assert!(!a.config.phrase_loop);
    assert!(session(&a).sounds.iter().all(|s| s.container.is_some()));
    a.undo_music().unwrap();
    assert_eq!(song(&a), before);
}

#[test]
fn to_song_appends_after_the_song_and_repeats_what_it_added() {
    use cat_smf_editor::score::scene::{arrangement, arrangement::PatternOrScene};
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    a.edit_tracks(|m| {
        let id = cat_smf_editor::score::pattern::operations::create(m, track, None)?;
        let lane = arrangement::add_lane(m, None)?;
        arrangement::place(m, None, lane, 1, Some(PatternOrScene::Pattern(id)))
    })
    .unwrap();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let before = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let (patterns, placed) = song(&a);
    assert_eq!(patterns.len(), before.0.len() + 1);
    let id = *patterns.iter().find(|p| !before.0.contains(p)).unwrap();
    assert_eq!(placed.len(), 2);
    assert!(placed.contains(&(2, id)));
    let m = &a.loaded.as_ref().unwrap().music;
    let range = (
        arrangement::measure_tick(m, None, 2).unwrap(),
        arrangement::measure_tick(m, None, 3).unwrap(),
    );
    assert_eq!(a.playback.repeat, Some(range));
    assert!(a.playback.pending.is_some());
    assert!(!phrase::any_looping(session(&a)));
    let cell = a.patterns.workspace.selection(Pane::Song);
    assert_eq!(
        (cell.measure, cell.choice),
        (2, Some(PatternOrScene::Pattern(id)))
    );
}

#[test]
fn ctrl_enter_again_after_to_song_changes_nothing() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let first = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    assert_eq!(song(&a), first);
}

#[test]
fn to_song_of_an_unchanged_loop_places_its_earlier_copy() {
    use cat_smf_editor::score::scene::arrangement::PatternOrScene;
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    let mut copy = 0;
    edit_music(&mut a, |m| {
        let track = cat_smf_editor::score::pattern::Catalog::read(m)
            .unwrap()
            .patterns[&id]
            .track;
        copy = cat_smf_editor::score::pattern::operations::create(m, track, Some(id)).unwrap();
    });
    let before = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let (patterns, placed) = song(&a);
    assert_eq!(patterns, before.0);
    assert_eq!(placed, vec![(1, copy)]);
    assert_eq!(
        a.patterns.workspace.selection(Pane::Song).choice,
        Some(PatternOrScene::Pattern(copy))
    );
}

#[test]
fn ctrl_enter_does_nothing_until_the_phrase_repeats() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let before = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    assert_eq!(song(&a), before);
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

/// A browsed patch of `name` on the playing container's first layer, as the browser applies it.
#[cfg(windows)]
fn browse_to(a: &mut App, track: usize, name: &str) {
    use cat_smf_editor::{chain_settings::PluginState, host::PluginKey};
    let slot = crate::test_support::referenced(PluginState {
        reference: None,
        plugin: PluginKey {
            format: "CLAP".into(),
            id: name.into(),
            name: name.into(),
            vendor: "fixture".into(),
            bundle_path: "X:/plugins/fixture.clap".into(),
        },
        state: vec![1],
        preset_name: Some(format!("X:/presets/{name}.fxp")),
        bypass: false,
    });
    let open = cat_smf_editor::resident_ipc::Open::Patch { track };
    crate::player::resident::apply(a, open, slot).unwrap();
}
#[cfg(windows)]
fn track_name(a: &App, track: usize) -> Option<String> {
    a.loaded.as_ref().unwrap().music.tracks[track].name()
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

/// The Loop pattern repeating now.
fn looping_pattern(a: &App) -> u64 {
    let Looper::Looping { pattern, .. } = session(a).sound().looper else {
        panic!("{:?}", session(a).sound().looper)
    };
    pattern
}

#[test]
fn the_repeating_measure_is_the_loop_pattern_shown_behind_the_keyboard() {
    use cat_smf_editor::score::{
        pattern::Catalog, pattern::Kind, scene::arrangement::PatternOrScene,
    };
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    a.edit_tracks(|m| {
        cat_smf_editor::score::pattern::operations::create(m, track, None).map(|_| ())
    })
    .unwrap();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        (c.patterns[&id].kind, c.patterns[&id].track),
        (Kind::Loop, track)
    );
    assert_eq!(a.patterns.editing.as_ref().map(|e| e.pattern), Some(id));
    assert_eq!(a.track, track);
    let song = a.patterns.workspace.selection(Pane::Song).choice;
    assert_eq!(song, Some(PatternOrScene::Pattern(id)));
    // The Loop leads its container's patterns in the Song window's list.
    let listed = crate::view::patterns::workspace::choices(&a, Pane::Song);
    assert_eq!(listed[0].0, PatternOrScene::Pattern(id));
    assert_eq!(listed.len(), 2);
}

/// An edit of the song through the event list's own path, which keeps the keyboard open.
fn edit_music(a: &mut App, edit: impl FnOnce(&mut cat_smf_editor::model::Music)) {
    let mut music = a.loaded.as_ref().unwrap().music.clone();
    edit(&mut music);
    a.apply_music(music).unwrap();
}

/// One quarter of `pitch` from the bar start as the Loop pattern, as an edit would leave it.
fn edit_loop(a: &mut App, pitch: u8) {
    let quarter = measure(a) / 4;
    let tempo = recorder(a).settings().tempo;
    edit_music(a, |m| {
        let source = Some(m.tracks.len() - 1);
        cat_smf_editor::score::pattern::recording::repeat(
            m,
            source,
            tempo,
            &[(Duration::ZERO, pitch, 100), (quarter, pitch, 0)],
            &[],
            false,
            false,
        )
        .unwrap();
    });
}

#[test]
fn an_edit_of_the_loop_pattern_sounds_from_the_next_lap() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let start = start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    edit_loop(&mut a, 64);
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(looping_pattern(&a), id);
    let lap = measure(&a);
    assert!(due(&a, start + lap * 3 - Duration::from_millis(1)).is_empty());
    assert_eq!(
        due(&a, start + lap * 3 + Duration::from_millis(1)),
        vec![word(64, 100)]
    );
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
fn deleting_the_loop_pattern_stops_the_phrase_and_the_switch_waits() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let start = start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    edit_music(&mut a, |m| {
        cat_smf_editor::score::pattern::operations::delete(m, id).unwrap()
    });
    frame(&mut a, &ctx, vec![], true);
    assert!(matches!(session(&a).sound().looper, Looper::Waiting));
    assert!(due(&a, start + measure(&a) * 9).is_empty());
}

#[test]
fn to_song_places_an_ordinary_copy_and_the_loop_pattern_stays_shown() {
    use cat_smf_editor::score::pattern::{Catalog, Kind};
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    frame(&mut a, &ctx, vec![], true);
    assert!(a.patterns.recording.is_open());
    let (_, placed) = song(&a);
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(placed.len(), 1);
    assert_ne!(placed[0].1, id);
    assert_eq!(c.patterns[&placed[0].1].kind, Kind::Normal);
    assert_eq!(bar(&a, placed[0].1), bar(&a, id));
    assert_eq!(a.patterns.editing.as_ref().map(|e| e.pattern), Some(id));
}

#[test]
fn a_pad_held_by_the_mouse_over_the_repeating_phrase_keeps_sounding() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let pad = cat_smf_editor::score::pattern::recording::Pad::Row { row: 0, column: 0 };
    let at = ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("pad-rect", pad))))
        .unwrap()
        .center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(
        &mut a,
        &ctx,
        vec![egui::Event::PointerMoved(at), button(true)],
        true,
    );
    assert!(recorder(&a).held(pad));
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![], true);
        assert!(recorder(&a).held(pad), "{:?}", session(&a).sound().looper);
    }
    frame(&mut a, &ctx, vec![button(false)], true);
    assert!(!recorder(&a).held(pad));
}

/// Pitches started by `words`.
fn started(words: &[u32], notes: &[(Duration, u8, u8)]) -> Vec<u8> {
    let mut out: Vec<u8> = notes
        .iter()
        .filter(|n| n.2 > 0 && words.contains(&word(n.1, n.2)))
        .map(|n| n.1)
        .collect();
    out.sort();
    out.dedup();
    out
}

#[test]
fn a_drum_phrase_goes_on_under_the_next_which_replaces_only_the_lanes_it_plays() {
    let (mut a, track) = container();
    a.loaded.as_mut().unwrap().patches[track].drums = Some(Default::default());
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let (kick, snare) = (36, 38);
    let length = measure(&a);
    let quarter = length / 4;
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    rewind(&mut a, quarter);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    rewind(&mut a, length);
    frame(&mut a, &ctx, vec![], true);
    let Looper::Looping { start, notes, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    let (start, kicks) = (*start, notes.clone());
    // The phrase's laps, wherever the test moves the session clock.
    let first = session(&a).clock + start;
    let lap = |_: &App, n: u32| first + length * n;
    // Three quarters through the first lap, the snare.
    rewind(&mut a, length / 2);
    due(&a, Instant::now());
    frame(&mut a, &ctx, vec![key(Key::A, true, Modifiers::NONE)], true);
    let Looper::Capturing { origin, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    assert_eq!(*origin, start);
    rewind(&mut a, quarter / 2);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::A, false, Modifiers::NONE)],
        true,
    );
    // The kick goes on under the capture, and the lap after it already has the snare.
    let heard = due(&a, lap(&a, 2) - Duration::from_millis(1));
    let Looper::Capturing { notes: taken, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    let mut both = kicks.clone();
    both.extend(taken.iter().copied());
    assert_eq!(started(&heard, &both), vec![kick, snare]);
    rewind(&mut a, length / 2);
    frame(&mut a, &ctx, vec![], true);
    let Looper::Looping {
        start: kept, notes, ..
    } = &session(&a).sound().looper
    else {
        panic!("{:?}", session(&a).sound().looper)
    };
    assert_eq!(*kept, start);
    let mut pitches: Vec<u8> = notes.iter().filter(|n| n.2 > 0).map(|n| n.1).collect();
    pitches.sort();
    assert_eq!(pitches, vec![kick, snare]);
    let notes = notes.clone();
    let lap3 = due(&a, lap(&a, 3));
    let later = due(&a, lap(&a, 3) + length - Duration::from_millis(1));
    assert!(!lap3.is_empty() || !later.is_empty());
    assert_eq!(started(&later, &notes), vec![kick, snare]);
}

#[test]
fn a_melodic_phrase_still_stops_under_the_next_key() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    frame(&mut a, &ctx, vec![key(Key::X, true, Modifiers::NONE)], true);
    let Looper::Capturing { under, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    assert!(under.is_none());
    assert!(!session(&a).sound().live.phrase.playing());
}
