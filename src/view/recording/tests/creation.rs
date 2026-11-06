use super::*;

#[test]
fn a_late_kit_result_cannot_add_a_container_to_another_song() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok((Patch::default(), a.state_cache.clone())))
        .unwrap();
    a.patterns.recording.session.as_mut().unwrap().kit_creation = Some(choices::KitCreation {
        song: a.song_id.wrapping_add(1),
        result: rx,
    });
    frame(&mut a, &ctx, vec![], true);
    assert!(Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .sources
        .is_empty());
    assert!(session(&a).kit_creation.is_none());
    assert!(!a.undo.available());
}

#[test]
fn each_creation_choice_dispatches_its_own_kind() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    for (index, expected) in [
        (0, window::Button::CreateSound(false)),
        (1, window::Button::CreateSound(true)),
        (2, window::Button::NewKit),
    ] {
        let s = a.patterns.recording.session.as_mut().unwrap();
        s.new_sound = Some(index);
        let _ = ctx.run(
            egui::RawInput {
                events: vec![key(Key::Enter, true, Modifiers::NONE)],
                ..Default::default()
            },
            |ctx| {
                assert!(choices::creation_keys(ctx, s));
            },
        );
        assert_eq!(s.clicked.take(), Some(expected));
        assert!(s.new_sound.is_none());
    }
}

#[test]
#[ignore = "requires RESIDENT_PLAYER and installed default kit; no GUI or device output"]
fn installed_default_kit_creation_and_disk_cache_restore() {
    use cat_smf_editor::{
        patch_roots::Table,
        score::{instrument, state_cache::Cache},
    };
    let work = tempfile::tempdir().unwrap();
    let player = std::path::PathBuf::from(std::env::var_os("RESIDENT_PLAYER").unwrap());
    let mut a = app();
    a.player_selection.executable = Some(player.clone());
    a.state_cache = Cache::new(Some(work.path().into()));
    let output = crate::player::state_request::headless(Some(player), "--patch-roots")
        .unwrap()
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut roots: Table = serde_json::from_slice(&output.stdout).unwrap();
    let _actual = crate::player::patch_roots::Assumed::with_table(roots.clone());
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let mut state = std::mem::take(&mut a.patterns.recording);
    let s = state.session.as_mut().unwrap();
    choices::create_kit(&a, s);
    let deadline = Instant::now() + Duration::from_secs(60);
    while s.kit_creation.is_some() {
        choices::poll_kit(&mut a, s);
        assert!(Instant::now() < deadline, "kit worker timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(s.error.is_none(), "{:?}", s.error);
    a.patterns.recording = state;
    let l = a.loaded.as_ref().unwrap();
    let c = Catalog::read(&l.music).unwrap();
    assert_eq!(c.sources.len(), 1);
    assert!(c.patterns.is_empty());
    let track = *c.sources.iter().next().unwrap();
    let original = l.patches[track].clone();
    let slot = original.layers[0].instrument.as_ref().unwrap();
    assert!(slot
        .reference
        .as_ref()
        .unwrap()
        .display
        .ends_with("/DSF/Mind Game.sfz"));
    assert!(!slot.state.is_empty());
    let bytes = cat_smf_editor::smf_io::encode(&l.music).unwrap();
    a.state_cache.save().unwrap();
    let parsed = cat_smf_editor::ffi::parse(&bytes).unwrap();
    let saved = cat_smf_editor::instrument_layers::read(&parsed.tracks[track], &mut roots).unwrap();
    let mut cache = Cache::new(Some(work.path().into()));
    let restored =
        instrument::Patch::resolve(saved, |r| cache.resolve(r, |_| panic!("disk cache miss")))
            .unwrap();
    assert_eq!(restored.kit, original.kit);
    assert_eq!(
        restored.layers[0].instrument.as_ref().unwrap().state,
        slot.state
    );
    assert!(!a.player_selection.kit_open);
}

fn press(a: &mut App, ctx: &egui::Context, keycode: Key, modifiers: Modifiers) {
    frame(
        a,
        ctx,
        vec![
            key(keycode, true, modifiers),
            key(keycode, false, modifiers),
        ],
        true,
    );
}

#[test]
fn creation_lists_three_kinds_and_escape_preserves_the_song_and_keyboard() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    press(&mut a, &ctx, Key::N, Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(session(&a).new_sound, Some(0));
    let mut labels = vec![];
    let output = ctx.run(Default::default(), |ctx| {
        let mut state = std::mem::take(&mut a.patterns.recording);
        choices::creation(ctx, state.session.as_mut().unwrap());
        a.patterns.recording = state;
    });
    for shape in output.shapes {
        if let egui::Shape::Text(text) = shape.shape {
            labels.push(text.galley.text().to_owned());
        }
    }
    for name in ["Init Saw", "Drum", "Drum Kit"] {
        assert!(labels.iter().any(|s| s == name), "{labels:?}");
    }
    press(&mut a, &ctx, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(session(&a).new_sound, Some(1));
    press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
    assert!(a.patterns.recording.is_open());
    assert!(session(&a).new_sound.is_none());
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!a.undo.available());
}

#[test]
fn enter_creates_the_selected_saw_without_a_pattern() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(&mut a, &ctx, Key::N, Modifiers::CTRL | Modifiers::SHIFT);
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(catalog.sources.len(), 1);
    assert!(catalog.patterns.is_empty());
    assert!(session(&a).new_sound.is_none());
    assert!(session(&a).sound().container.is_some());
    assert!(!recorder(&a).settings().drums);
}

#[test]
fn kit_completion_adds_only_a_container_and_keeps_the_keyboard_play_target() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let patch = Patch {
        kit: Some(cat_smf_editor::drum_kit::Kit {
            layer: 1,
            rows: None,
            muted: vec![],
            swing: Default::default(),
            notes: vec![cat_smf_editor::drum_kit::Note {
                note: 36,
                name: "Kick".into(),
                velocity: 100,
            }],
        }),
        ..Default::default()
    };
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok((patch, a.state_cache.clone()))).unwrap();
    a.patterns.recording.session.as_mut().unwrap().kit_creation = Some(choices::KitCreation {
        song: a.song_id,
        result: rx,
    });
    frame(&mut a, &ctx, vec![], true);
    let l = a.loaded.as_ref().unwrap();
    let catalog = Catalog::read(&l.music).unwrap();
    assert_eq!(catalog.sources.len(), 1);
    assert!(catalog.patterns.is_empty());
    assert!(l.patches[*catalog.sources.iter().next().unwrap()]
        .kit
        .is_some());
    assert!(session(&a).sound().patch.kit.is_none());
    assert_eq!(session(&a).sound().track, l.music.tracks.len());
    assert!(!a.player_selection.kit_open);
    a.undo_music().unwrap();
    assert!(Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .sources
        .is_empty());
}
