use super::*;
use crate::view::recent::tests::{music, press};
use cat_smf_editor::model::Event;
use eframe::egui::{Key, Modifiers};

fn idle(app: &mut App, ctx: &egui::Context) {
    app.recent.prefetch.last_input = Some(Instant::now() - IDLE);
    let _ = ctx.run(egui::RawInput::default(), |ctx| update(app, ctx));
}
fn finish(app: &mut App, ctx: &egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.recent.prefetch.work.is_some() {
        idle(app, ctx);
        assert!(Instant::now() < deadline, "worker did not finish");
        std::thread::sleep(Duration::from_millis(2));
    }
}
struct Fixture {
    root: tempfile::TempDir,
    app: App,
    ctx: egui::Context,
}
fn fixture(files: &[(&str, &[u8])]) -> (Fixture, Vec<PathBuf>) {
    let root = tempfile::tempdir().unwrap();
    let source = music();
    let mut app = crate::view::tests::app(source.clone());
    app.recent.prefetch.directory = Some(root.path().join("wav"));
    let mut paths = Vec::new();
    for (name, bytes) in files.iter().rev() {
        let path = root.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        let path = std::fs::canonicalize(path).unwrap();
        app.recent.history.record(&path, &source);
        paths.insert(0, path);
    }
    let ctx = egui::Context::default();
    (Fixture { root, app, ctx }, paths)
}
fn smf() -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&music()).unwrap()
}

#[test]
fn only_the_open_overlay_renders_the_first_measure_named_by_the_smf_hash() {
    let bytes = smf();
    let (mut f, paths) = fixture(&[("a.mid", &bytes)]);
    idle(&mut f.app, &f.ctx);
    assert!(f.app.recent.prefetch.work.is_none());
    f.app.recent.open = true;
    idle(&mut f.app, &f.ctx);
    assert!(f.app.recent.prefetch.work.is_some());
    assert_eq!(status(&f.app, &paths[0]), "準備中…");
    finish(&mut f.app, &f.ctx);
    let wav = f
        .root
        .path()
        .join("wav")
        .join(format!("{}.wav", hash(&bytes)));
    let (rate, samples) = crate::play::wav::read(&wav).unwrap();
    // 4/4 at 120 BPM: one measure is two seconds.
    assert_eq!((rate, samples.len()), (48000, 2 * 48000 * CHANNELS));
    assert_eq!(status(&f.app, &paths[0]), "準備済み");
    idle(&mut f.app, &f.ctx);
    assert!(f.app.recent.prefetch.work.is_none());
    assert!(f.app.playback.active.is_none() && f.app.playback.pending.is_none());
}

#[test]
fn closing_the_overlay_cancels_and_a_known_hash_is_not_rendered_again() {
    let bytes = smf();
    let (mut f, paths) = fixture(&[("a.mid", &bytes), ("copy.mid", &bytes)]);
    f.app.recent.open = true;
    idle(&mut f.app, &f.ctx);
    assert!(f.app.recent.prefetch.work.is_some());
    f.app.recent.open = false;
    idle(&mut f.app, &f.ctx);
    assert!(f.app.recent.prefetch.work.is_none());
    f.app.recent.open = true;
    idle(&mut f.app, &f.ctx);
    finish(&mut f.app, &f.ctx);
    // The same SMF under another name reuses the WAV.
    idle(&mut f.app, &f.ctx);
    assert!(f.app.recent.prefetch.work.is_none());
    assert_eq!(status(&f.app, &paths[1]), "準備済み");
}

#[test]
fn failed_files_are_skipped_until_they_change_and_the_cursor_file_goes_first() {
    let bytes = smf();
    let (mut f, paths) = fixture(&[("bad.mid", b"invalid"), ("good.mid", &bytes)]);
    press(&mut f.app, &f.ctx, Key::P, Modifiers::CTRL);
    press(&mut f.app, &f.ctx, Key::J, Modifiers::NONE);
    assert_eq!(f.app.recent.listen.as_ref(), Some(&paths[1]));
    assert!(f.app.recent.preview.is_none());
    idle(&mut f.app, &f.ctx);
    assert_eq!(f.app.recent.prefetch.work.as_ref().unwrap().path, paths[1]);
    finish(&mut f.app, &f.ctx);
    assert!(f.app.recent.preview.is_some());
    assert!(f.app.recent.listen.is_none());
    idle(&mut f.app, &f.ctx);
    finish(&mut f.app, &f.ctx);
    assert_eq!(status(&f.app, &paths[0]), "準備失敗");
    idle(&mut f.app, &f.ctx);
    assert!(f.app.recent.prefetch.work.is_none());
    std::fs::write(&paths[0], &bytes).unwrap();
    idle(&mut f.app, &f.ctx);
    finish(&mut f.app, &f.ctx);
    assert_eq!(status(&f.app, &paths[0]), "準備済み");
    assert!(f.app.load_error.is_none());
}

#[test]
fn first_measure_starts_at_the_first_note_and_follows_the_meter() {
    let mut source = music();
    let events = &mut source.tracks[0].events;
    events.insert(
        0,
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x58,
            data: vec![3, 2, 24, 8],
        },
    );
    for event in events
        .iter_mut()
        .filter(|e| e.status != 0xff || e.meta_type == 0x2f)
    {
        event.tick += 480;
    }
    let app = crate::view::tests::app(source);
    assert_eq!(first_measure(&app), Some((480, 480 + 1440)));
}

#[test]
#[ignore = "requires installed default instrument; opens plugins but no audio stream"]
fn native_render_writes_audible_first_measure() {
    let library = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(library.path()).unwrap();
    let bytes = smf();
    let (mut f, _) = fixture(&[("a.mid", &bytes)]);
    f.app.audition = crate::play::audition::Audition::default();
    f.app.recent.open = true;
    let started = Instant::now();
    let mut slowest = Duration::ZERO;
    let deadline = started + Duration::from_secs(60);
    idle(&mut f.app, &f.ctx);
    while f.app.recent.prefetch.work.is_some() {
        let frame = Instant::now();
        idle(&mut f.app, &f.ctx);
        slowest = slowest.max(frame.elapsed());
        assert!(Instant::now() < deadline, "render did not finish");
        std::thread::sleep(Duration::from_millis(2));
    }
    let wav = f
        .root
        .path()
        .join("wav")
        .join(format!("{}.wav", hash(&bytes)));
    let (rate, samples) = crate::play::wav::read(&wav).unwrap();
    let peak = samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
    println!(
        "rate={rate} seconds={} peak={peak} total={:?} slowest UI frame={slowest:?}",
        samples.len() as f64 / CHANNELS as f64 / f64::from(rate),
        started.elapsed()
    );
    assert!(peak > 0.001, "rendered measure is silent");
    assert_eq!(samples.len(), 2 * rate as usize * CHANNELS);
}
