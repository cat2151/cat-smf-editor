use super::*;
use cat_smf_editor::{edit, score::pattern::body};
use std::time::Duration;
mod loop_takes;
mod mouse;
mod native;
mod play;
mod preparing;
mod transport;

/// A key whose physical position matches its US letter, as on the measured Japanese keyboard.
fn key(key: Key, pressed: bool, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers,
    }
}
fn frame(a: &mut App, c: &egui::Context, events: Vec<egui::Event>, focused: bool) {
    let _ = c.run(
        egui::RawInput {
            events,
            focused,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000., 700.),
            )),
            ..Default::default()
        },
        |ctx| {
            let input = modal::begin(a, ctx);
            // The embedded keyboard shares this input; native windows each get their own.
            if !a.patterns.recording.is_open() {
                commands::route(a, ctx);
            }
            egui::CentralPanel::default().show(ctx, |_| {});
            modal::end(a, ctx, input);
            show(a, ctx);
        },
    );
}
/// Shift+R, leaving the window playing.
fn open_key(a: &mut App, ctx: &egui::Context) {
    frame(a, ctx, vec![key(Key::R, true, Modifiers::SHIFT)], true);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert!(!a.patterns.recording.preparing(), "{:?}", session(a).error);
}
/// Keys pressed in the settings window.
fn settings_keys(a: &mut App, keys: &[(Key, Modifiers)]) {
    let mut state = std::mem::take(&mut a.patterns.recording);
    let events: Vec<_> = keys.iter().map(|&(k, m)| key(k, true, m)).collect();
    companions::settings_keys(a, state.session.as_mut().unwrap(), &events);
    a.patterns.recording = state;
}
fn session(a: &App) -> &Session {
    a.patterns.recording.session.as_ref().unwrap()
}
fn recorder(a: &App) -> &Recorder {
    session(a).recorder.as_ref().unwrap()
}
fn app() -> App {
    crate::view::tests::app(edit::tracks::new_song())
}

#[test]
fn keyboard_recording_commit_round_trip_and_single_undo() {
    let mut a = app();
    let ctx = egui::Context::default();
    let original = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    open_key(&mut a, &ctx);
    frame(
        &mut a,
        &ctx,
        vec![
            key(Key::Space, true, Modifiers::NONE),
            key(Key::Z, true, Modifiers::NONE),
            key(Key::C, true, Modifiers::NONE),
        ],
        true,
    );
    assert!(recorder(&a).sounding(60) && recorder(&a).sounding(64));
    let s = session(&a);
    let queued = s
        .queue
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .iter()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(queued, vec![0x2090_3c50, 0x2090_4050]);
    // Opening and playing have not changed the document or Undo history.
    assert_eq!(
        original,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!a.undo.available());
    a.patterns.recording.session.as_mut().unwrap().clock -= Duration::from_millis(500);
    frame(
        &mut a,
        &ctx,
        vec![
            key(Key::Z, false, Modifiers::SHIFT),
            key(Key::Escape, true, Modifiers::NONE),
        ],
        true,
    );
    assert!(!a.patterns.recording.is_open());
    assert!(!a.audition.live_active());
    let music = &a.loaded.as_ref().unwrap().music;
    let c = Catalog::read(music).unwrap();
    assert_eq!(c.patterns.len(), 1);
    let id = a.patterns.selected.unwrap();
    let p = &c.patterns[&id];
    let b = body::extract(&music.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(b.iter().filter(|e| e.status == 0x90).count(), 2);
    assert_eq!(b.iter().filter(|e| e.status == 0x80).count(), 2);
    assert!(b
        .iter()
        .filter(|e| e.status == 0x80)
        .all(|e| e.tick - p.tick >= 480));
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    let work = tempfile::tempdir().unwrap();
    let path = work.path().join("recorded.mid");
    let recorded = cat_smf_editor::smf_io::encode(music).unwrap();
    a.config = crate::store::config::Store::open(work.path().join("status.json"));
    a.saved.directory = Some(work.path().into());
    a.saved.save_as(&path).unwrap();
    a.sync_saved_path();
    a.save_session();
    let mut restarted = app();
    restarted.loaded = None;
    restarted.config = crate::store::config::Store::open(work.path().join("status.json"));
    restarted.state_cache =
        cat_smf_editor::score::state_cache::Cache::new(Some(work.path().into()));
    restarted.open_initial(None);
    assert!(restarted.load_error.is_none(), "{:?}", restarted.load_error);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&restarted.loaded.as_ref().unwrap().music).unwrap(),
        recorded
    );
    assert_eq!(restarted.patterns.selected, Some(id));
    assert!(!restarted.patterns.recording.is_open());
    a.undo_music().unwrap();
    assert_eq!(
        original,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!a.undo.available());
}

#[test]
fn empty_cancel_repeat_and_focus_loss_do_not_stick_notes_or_leak_keys() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    frame(
        &mut a,
        &ctx,
        vec![
            key(Key::Escape, true, Modifiers::NONE),
            key(Key::I, true, Modifiers::NONE),
        ],
        true,
    );
    assert!(!a.patterns.recording.is_open());
    assert!(!a.audition.live_active());
    assert!(!a.mml_input.open && !a.undo.available());
    open_key(&mut a, &ctx);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    let mut repeat = key(Key::Z, true, Modifiers::NONE);
    if let egui::Event::Key { repeat: r, .. } = &mut repeat {
        *r = true;
    }
    frame(
        &mut a,
        &ctx,
        vec![repeat, key(Key::J, true, Modifiers::SHIFT)],
        true,
    );
    let s = session(&a);
    assert_eq!(s.queue.as_ref().unwrap().lock().unwrap().len(), 1);
    frame(&mut a, &ctx, vec![egui::Event::WindowFocused(false)], false);
    let s = session(&a);
    assert!(!recorder(&a).sounding(60));
    assert_eq!(
        s.queue.as_ref().unwrap().lock().unwrap().back(),
        Some(&0x2080_3c00)
    );
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Escape, true, Modifiers::NONE)],
        true,
    );
    assert!(!a.patterns.recording.is_open());
}

#[test]
fn ambiguous_container_choice_is_keyboard_operable_and_cancel_is_atomic() {
    let mut a = app();
    let mut first = 0;
    let mut second = 0;
    a.edit_tracks(|m| {
        first = cat_smf_editor::score::pattern::operations::create_source(m)?;
        second = cat_smf_editor::score::pattern::operations::create_source(m)?;
        Ok(())
    })
    .unwrap();
    a.track = 0;
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    assert!(session(&a).settings_open);
    assert_eq!(session(&a).item, setup::Item::Sound);
    frame(&mut a, &ctx, vec![key(Key::P, true, Modifiers::CTRL)], true);
    assert!(!a.patterns.recording.preparing());
    assert_eq!(
        a.patterns.recording.session.as_ref().unwrap().sounds
            [a.patterns.recording.session.as_ref().unwrap().selected]
            .track,
        second
    );
    frame(
        &mut a,
        &ctx,
        vec![
            key(Key::Space, true, Modifiers::NONE),
            key(Key::Z, true, Modifiers::NONE),
            key(Key::Escape, true, Modifiers::NONE),
        ],
        true,
    );
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(c.patterns.values().next().unwrap().track, second);
    assert!(c.patterns.values().all(|p| p.track != first));
}

#[test]
fn entry_binding_is_shared_and_text_editors_keep_their_input() {
    let a = app();
    for s in [Screen::Main, Screen::Scene, Screen::Song] {
        let defs = commands::definitions(&a, s);
        let c = defs.iter().find(|c| c.id == "record-pattern").unwrap();
        assert_eq!(c.keys, vec![(Key::R, Modifiers::SHIFT)]);
        assert_eq!(
            defs.iter()
                .filter(|c| c.keys.contains(&(Key::R, Modifiers::SHIFT)))
                .count(),
            1
        );
    }
    for s in [Screen::Mml, Screen::ArrangementInput, Screen::Browser] {
        assert!(!commands::definitions(&a, s)
            .iter()
            .any(|c| c.id == "record-pattern"));
    }
}
