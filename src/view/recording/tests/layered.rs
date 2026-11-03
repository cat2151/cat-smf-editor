//! Phrases of several containers repeating together.
use super::*;
use crate::play::audition::live::word;
use phrase::Looper;
use std::time::Instant;

fn press(a: &mut App, ctx: &egui::Context, k: Key, m: Modifiers) {
    frame(a, ctx, vec![key(k, true, m), key(k, false, m)], true);
}
fn measure(a: &App) -> Duration {
    Duration::from_micros(u64::from(recorder(a).settings().tempo) * 4)
}
/// An app with two containers, the first selected.
fn two() -> App {
    let mut a = app();
    for _ in 0..2 {
        a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
            .unwrap();
    }
    a.track = a.loaded.as_ref().unwrap().music.tracks.len() - 2;
    a
}
/// Moves the session clock so that `at` of session time is now.
fn at(a: &mut App, at: Duration) {
    a.patterns.recording.session.as_mut().unwrap().clock = Instant::now() - at;
}
fn looper(a: &App, i: usize) -> &Looper {
    &session(a).sounds[i].looper
}
/// Session time of the first repeat of container `i`.
fn start(a: &App, i: usize) -> Duration {
    let Looper::Looping { start, .. } = looper(a, i) else {
        panic!("{:?}", looper(a, i))
    };
    *start
}
fn origin(a: &App, i: usize) -> Duration {
    let Looper::Capturing { origin, .. } = looper(a, i) else {
        panic!("{:?}", looper(a, i))
    };
    *origin
}
/// The phrase words container `i` would play by `at`, session time.
fn due(a: &App, i: usize, at: Duration) -> Vec<u32> {
    let mut out = [0; 64];
    let s = session(a);
    let n = s.sounds[i].live.phrase.due(s.clock + at, &mut out);
    out[..n].to_vec()
}
/// `k` held for an eighth of the measure from `from`, then the measure completed.
fn phrase(a: &mut App, ctx: &egui::Context, k: Key, from: Duration) {
    at(a, from);
    frame(a, ctx, vec![key(k, true, Modifiers::NONE)], true);
    let eighth = measure(a) / 8;
    at(a, from + eighth);
    frame(a, ctx, vec![key(k, false, Modifiers::NONE)], true);
    at(a, origin(a, session(a).selected) + measure(a));
    frame(a, ctx, vec![], true);
}

/// The first container repeating `z` from session time zero.
fn first_loop(a: &mut App, ctx: &egui::Context) -> Duration {
    open_key(a, ctx);
    press(a, ctx, Key::E, Modifiers::CTRL);
    phrase(a, ctx, Key::Z, Duration::ZERO);
    start(a, 0)
}

#[test]
fn another_container_plays_over_the_repeating_phrase_from_its_bar_line() {
    let mut a = two();
    let ctx = egui::Context::default();
    let grid = first_loop(&mut a, &ctx);
    let bar = measure(&a);
    press(&mut a, &ctx, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(session(&a).selected, 1);
    assert!(matches!(looper(&a, 0), Looper::Looping { .. }));
    assert!(matches!(looper(&a, 1), Looper::Off));
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(looper(&a, 1), Looper::Waiting));
    // Half way through a lap of the first phrase.
    phrase(&mut a, &ctx, Key::X, grid + bar * 2 + bar / 2);
    assert_eq!(start(&a, 1), grid + bar * 3);
    assert_eq!(start(&a, 0), grid);
    // Both repeat: the first from its own laps, the second from the half of its measure.
    let from = |a: &App, i| due(a, i, grid + bar * 9 + bar * 3 / 4).first().copied();
    assert_eq!(from(&a, 0), Some(word(60, 80)));
    assert!(from(&a, 1).is_some());
    let words = due(&a, 1, grid + bar * 9 + bar * 3 / 4);
    assert!(words.iter().all(|w| (w >> 8) & 0x7f == 62), "{words:x?}");
}

#[test]
fn a_key_a_sixteenth_early_starts_on_the_next_bar_line() {
    let mut a = two();
    let ctx = egui::Context::default();
    let grid = first_loop(&mut a, &ctx);
    let bar = measure(&a);
    press(&mut a, &ctx, Key::ArrowRight, Modifiers::NONE);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    at(&mut a, grid + bar * 3 - bar / 32);
    frame(&mut a, &ctx, vec![key(Key::X, true, Modifiers::NONE)], true);
    assert_eq!(origin(&a, 1), grid + bar * 3);
    // Earlier than a sixteenth, the key belongs to the lap it is in.
    let mut a = two();
    let grid = first_loop(&mut a, &ctx);
    press(&mut a, &ctx, Key::ArrowRight, Modifiers::NONE);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    at(&mut a, grid + bar * 3 - bar / 8);
    frame(&mut a, &ctx, vec![key(Key::X, true, Modifiers::NONE)], true);
    assert_eq!(origin(&a, 1), grid + bar * 2);
}

#[test]
fn ctrl_e_stops_only_the_playing_containers_phrase() {
    let mut a = two();
    let ctx = egui::Context::default();
    let grid = first_loop(&mut a, &ctx);
    let bar = measure(&a);
    press(&mut a, &ctx, Key::ArrowRight, Modifiers::NONE);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    phrase(&mut a, &ctx, Key::X, grid + bar * 2 + bar / 2);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(matches!(looper(&a, 1), Looper::Off));
    assert!(due(&a, 1, grid + bar * 20).is_empty());
    assert!(matches!(looper(&a, 0), Looper::Looping { .. }));
    assert!(!due(&a, 0, grid + bar * 20).is_empty());
    assert!(
        a.config.phrase_loop,
        "the first container's switch is still on"
    );
}

#[test]
fn each_container_keeps_the_mode_it_was_left_with() {
    use cat_smf_editor::score::pattern::recording::Mode;
    let mut a = two();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let mode = |a: &App| recorder(a).settings().mode;
    press(&mut a, &ctx, Key::Tab, Modifiers::NONE);
    assert_eq!(mode(&a), Mode::Triad);
    press(&mut a, &ctx, Key::ArrowRight, Modifiers::NONE);
    press(&mut a, &ctx, Key::Tab, Modifiers::NONE);
    assert_eq!(mode(&a), Mode::Seventh);
    press(&mut a, &ctx, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(mode(&a), Mode::Triad);
    // The next opening plays each container as it was left.
    press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
    a.track += 1;
    open_key(&mut a, &ctx);
    assert_eq!(mode(&a), Mode::Seventh);
}

#[test]
fn to_song_puts_every_repeating_loop_on_its_own_lane_at_the_first_bar() {
    use cat_smf_editor::score::{pattern::Catalog, scene};
    let mut a = two();
    let ctx = egui::Context::default();
    let grid = first_loop(&mut a, &ctx);
    press(&mut a, &ctx, Key::ArrowRight, Modifiers::NONE);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    let half = measure(&a) / 2;
    phrase(&mut a, &ctx, Key::X, grid + half);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let m = &a.loaded.as_ref().unwrap().music;
    let song = scene::read(m).unwrap().song;
    let c = Catalog::read(m).unwrap();
    assert_eq!(song.len(), 2);
    assert!(song.iter().all(|p| p.measure == 1));
    assert_ne!(song[0].lane, song[1].lane);
    let tracks: Vec<usize> = song
        .iter()
        .map(|p| match p.part {
            scene::arrangement::PatternOrScene::Pattern(id) => c.patterns[&id].track,
            _ => panic!(),
        })
        .collect();
    assert_ne!(tracks[0], tracks[1]);
    assert!(matches!(looper(&a, 0), Looper::Off));
    assert!(matches!(looper(&a, 1), Looper::Off));
    a.undo_music().unwrap();
    assert!(scene::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .song
        .is_empty());
}
