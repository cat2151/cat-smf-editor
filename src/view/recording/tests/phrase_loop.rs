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
    let n = session(a).phrase.as_ref().unwrap().due(at, &mut out);
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
    assert!(matches!(session(a).looper, Looper::Off));
    press(a, ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(session(a).looper, Looper::Waiting));
    frame(a, ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    let Looper::Capturing { origin, .. } = session(a).looper else {
        panic!("{:?}", session(a).looper)
    };
    let quarter = measure(a) / 4;
    let start = session(a).clock + origin - quarter;
    rewind(a, quarter);
    frame(a, ctx, vec![key(Key::Z, false, Modifiers::NONE)], true);
    assert!(!phrase::looping(session(a)));
    rewind(a, measure(a));
    frame(a, ctx, vec![], true);
    assert!(phrase::looping(session(a)));
    let (on, off) = (word(60, true), word(60, false));
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
        vec![word(60, true)]
    );
    session_queue_clear(&mut a);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(session(&a).looper, Looper::Off));
    assert_eq!(queued(&a), vec![word(60, false)]);
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
        vec![word(60, true)]
    );
    session_queue_clear(&mut a);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    // The old phrase lets go before the key of the same pitch sounds.
    assert_eq!(queued(&a), vec![word(60, false), word(60, true)]);
    assert!(due(&a, start + measure(&a) * 9).is_empty());
    let Looper::Capturing { origin, .. } = session(&a).looper else {
        panic!("{:?}", session(&a).looper)
    };
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
    assert_eq!(
        due(&a, restart + measure(&a) + Duration::from_millis(1)),
        vec![word(60, true)]
    );
    session_queue_clear(&mut a);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(session(&a).looper, Looper::Off));
    assert_eq!(queued(&a), vec![word(60, false)]);
}

fn queued(a: &App) -> Vec<u32> {
    session(a)
        .queue
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .iter()
        .copied()
        .collect()
}

fn session_queue_clear(a: &mut App) {
    session(a).queue.as_ref().unwrap().lock().unwrap().clear();
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
    assert_eq!(a.audition.live_patch.as_ref(), Some(&patch));
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
    assert!(matches!(session(&a).looper, Looper::Waiting));

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
