use super::*;
use crate::play::audition::live::word;
use phrase::Looper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static LAUNCHED: AtomicBool = AtomicBool::new(false);

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

fn refuse(_: &std::path::Path, _: &str) -> std::io::Result<Option<std::process::Child>> {
    LAUNCHED.store(true, Ordering::Relaxed);
    Err(std::io::Error::other("no player in tests"))
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

/// The Loop pattern repeating now.
fn looping_pattern(a: &App) -> u64 {
    let Looper::Looping { pattern, .. } = session(a).sound().looper else {
        panic!("{:?}", session(a).sound().looper)
    };
    pattern
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

mod controls;

mod song;

mod patterns;

mod drums;
