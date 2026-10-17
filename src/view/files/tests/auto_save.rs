use super::*;
use std::time::{Duration, Instant};

fn frame(app: &mut App, ctx: &egui::Context, now: Instant) -> egui::FullOutput {
    ctx.run(Default::default(), |ctx| app.auto_save_at(ctx, now))
}

fn enable(f: &mut Fixture, ctx: &egui::Context) {
    press(&mut f.app, ctx, Key::Comma);
    press(&mut f.app, ctx, Key::S);
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::EnableAutoSave)
    ));
    assert!(!crate::store::auto_save::enabled(
        &f.app.loaded.as_ref().unwrap().music
    ));
    press(&mut f.app, ctx, Key::Y);
    assert!(!f.app.saved.dirty());
}

#[test]
fn auto_save_activation_saves_then_timer_preserves_exact_backups() {
    let mut f = fixture("auto-save-timer");
    let ctx = egui::Context::default();
    let original = std::fs::read(&f.first).unwrap();
    edit(&mut f.app);
    frame(&mut f.app, &ctx, Instant::now() + Duration::from_secs(120));
    assert_eq!(std::fs::read(&f.first).unwrap(), original);
    assert!(f.app.saved.auto_save_deadline().is_none());
    enable(&mut f, &ctx);
    let opted_in = std::fs::read(&f.first).unwrap();
    // A subsequent edit has a fixed deadline, even if more edits arrive.
    crate::play::mixer::volume::set(&mut f.app, 0, 0.5).unwrap();
    let deadline = f.app.saved.auto_save_deadline().unwrap();
    crate::play::mixer::volume::set(&mut f.app, 0, 0.6).unwrap();
    assert_eq!(f.app.saved.auto_save_deadline(), Some(deadline));
    frame(&mut f.app, &ctx, deadline - Duration::from_millis(1));
    assert_eq!(std::fs::read(&f.first).unwrap(), opted_in);
    let output = ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx));
    assert!(output.viewport_output.values().any(|v| v.commands.iter().any(|c| {
        matches!(c, egui::ViewportCommand::Title(t) if t.contains("first.mid(更新)(1分後にauto save)"))
    })));
    frame(&mut f.app, &ctx, deadline);
    assert!(!f.app.saved.dirty());
    assert!(f.app.saved.auto_save_deadline().is_none());
    assert!(f.app.config.navigation.departure.is_none());
    let backups: Vec<_> = std::fs::read_dir(f.root.join("config/backup"))
        .unwrap()
        .map(|e| std::fs::read(e.unwrap().path()).unwrap())
        .collect();
    assert_eq!(backups.len(), 2);
    assert!(backups.contains(&original) && backups.contains(&opted_in));
    let saved = std::fs::read(&f.first).unwrap();
    // Reopen under another name, then restore through the normal startup path.
    let renamed = f.root.join("sources/renamed.mid");
    std::fs::rename(&f.first, &renamed).unwrap();
    f.app.open_mid(renamed.clone());
    assert!(f.app.load_error.is_none());
    f.app.save_session();
    let mut restarted = crate::view::tests::app(f.app.loaded.as_ref().unwrap().music.clone());
    restarted.loaded = None;
    restarted.saved = crate::store::saved::Document::new(Some(f.root.join("config")));
    restarted.config = crate::store::config::Store::open(f.root.join("status.json"));
    restarted.open_initial(None);
    assert!(restarted.load_error.is_none());
    assert_eq!(
        restarted.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(renamed).unwrap()
    );
    crate::play::mixer::volume::set(&mut restarted, 0, 0.7).unwrap();
    assert!(restarted.saved.auto_save_deadline().is_some());
    assert_eq!(
        smf_io::encode(&smf_io::load(&restarted.loaded.as_ref().unwrap().original).unwrap())
            .unwrap(),
        saved
    );
}

#[test]
fn auto_save_waits_for_input_and_pointer_but_close_flushes_before_deadline() {
    let mut f = fixture("auto-save-busy");
    let ctx = egui::Context::default();
    enable(&mut f, &ctx);
    edit(&mut f.app);
    let before = std::fs::read(&f.first).unwrap();
    let output = super::clean_close::close_request(&mut f.app, &ctx);
    assert!(!f.app.saved.dirty());
    assert!(f.app.config.navigation.departure.is_none());
    assert!(!output
        .viewport_output
        .values()
        .any(|v| v.commands.contains(&egui::ViewportCommand::CancelClose)));
    assert_ne!(std::fs::read(&f.first).unwrap(), before);
    crate::play::mixer::volume::set(&mut f.app, 0, 0.5).unwrap();
    let deadline = f.app.saved.auto_save_deadline().unwrap();
    let before = std::fs::read(&f.first).unwrap();
    crate::view::note_edit::begin(
        &mut f.app,
        crate::view::note_edit::Field::Duration,
        0,
        1,
        egui::Rect::from_min_size(egui::pos2(300.0, 200.0), egui::vec2(80.0, 20.0)),
        "240",
    );
    frame(&mut f.app, &ctx, deadline);
    assert_eq!(std::fs::read(&f.first).unwrap(), before);
    f.app.note_edit = None;
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: egui::pos2(20.0, 20.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| f.app.auto_save_at(ctx, deadline),
    );
    assert_eq!(std::fs::read(&f.first).unwrap(), before);
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: egui::pos2(20.0, 20.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| f.app.auto_save_at(ctx, deadline),
    );
    assert_eq!(std::fs::read(&f.first).unwrap(), before);
    frame(&mut f.app, &ctx, deadline);
    assert!(!f.app.saved.dirty());
}

#[test]
fn auto_save_failure_stops_retries_and_manual_save_recovers() {
    for external in [false, true] {
        let mut f = fixture(if external {
            "auto-save-external"
        } else {
            "auto-save-backup"
        });
        let ctx = egui::Context::default();
        enable(&mut f, &ctx);
        edit(&mut f.app);
        let expected = std::fs::read(&f.first).unwrap();
        let backup = f.root.join("config/backup");
        if external {
            std::fs::write(&f.first, b"externally changed").unwrap();
        } else {
            std::fs::rename(&backup, backup.with_file_name("preserved-backup")).unwrap();
            std::fs::write(&backup, b"blocked").unwrap();
        }
        let now = f.app.saved.auto_save_deadline().unwrap();
        frame(&mut f.app, &ctx, now);
        assert!(f.app.saved.dirty());
        assert!(f.app.saved.error.is_some());
        assert!(f.app.saved.auto_save_deadline().is_none());
        assert_eq!(
            std::fs::read(&f.first).unwrap(),
            if external {
                b"externally changed".to_vec()
            } else {
                expected.clone()
            }
        );
        if external {
            std::fs::write(&f.first, expected).unwrap();
        } else {
            std::fs::remove_file(&backup).unwrap();
        }
        frame(&mut f.app, &ctx, now + Duration::from_secs(120));
        assert!(f.app.saved.dirty());
        save(&mut f.app, &ctx);
        press(&mut f.app, &ctx, Key::Y);
        assert!(!f.app.saved.dirty());
        assert!(f.app.saved.error.is_none());
        crate::play::mixer::volume::set(&mut f.app, 0, 0.5).unwrap();
        assert!(f.app.saved.auto_save_deadline().is_some());
    }
}

#[test]
fn auto_save_off_and_undo_cancel_pending_save_and_files_remain_independent() {
    let mut f = fixture("auto-save-off");
    let ctx = egui::Context::default();
    enable(&mut f, &ctx);
    edit(&mut f.app);
    let deadline = f.app.saved.auto_save_deadline().unwrap();
    f.app.undo_music().unwrap();
    assert!(!f.app.saved.dirty());
    assert!(f.app.saved.auto_save_deadline().is_none());
    f.app.toggle_auto_save();
    assert!(f.app.saved.auto_save_deadline().is_none());
    frame(&mut f.app, &ctx, deadline);
    assert!(f.app.saved.dirty());
    save(&mut f.app, &ctx);
    press(&mut f.app, &ctx, Key::Y);
    f.app.open_mid(f.second.clone());
    assert!(!crate::store::auto_save::enabled(
        &f.app.loaded.as_ref().unwrap().music
    ));
    edit(&mut f.app);
    assert!(f.app.saved.auto_save_deadline().is_none());
}

#[test]
fn auto_save_menu_click_and_keyboard_share_file_setting_and_do_not_leak_keys() {
    let mut f = fixture("auto-save-menu");
    let ctx = egui::Context::default();
    // The roll retains its own open state when opening/closing the settings menu.
    f.app.piano_roll.open = true;
    press(&mut f.app, &ctx, Key::Comma);
    let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx));
    let output = ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx));
    let pos = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == "s: このファイルのAuto save" => {
                Some(t.pos + egui::vec2(4.0, 5.0))
            }
            _ => None,
        })
        .unwrap();
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| crate::view::show(&mut f.app, ctx),
        );
    }
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::EnableAutoSave)
    ));
    assert!(!f.app.comma_menu.open);
    press(&mut f.app, &ctx, Key::S);
    assert!(!crate::store::auto_save::enabled(
        &f.app.loaded.as_ref().unwrap().music
    ));
    press(&mut f.app, &ctx, Key::N);
    assert!(!f.app.saved.dirty());
    assert!(f.app.piano_roll.open);
    assert!(f.app.config.navigation.departure.is_none());
    // Typing a comma or s into a note field cannot change the file's setting.
    crate::view::note_edit::begin(
        &mut f.app,
        crate::view::note_edit::Field::Duration,
        0,
        0,
        egui::Rect::from_min_size(egui::pos2(300.0, 200.0), egui::vec2(80.0, 20.0)),
        "480",
    );
    let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx));
    press(&mut f.app, &ctx, Key::Comma);
    press(&mut f.app, &ctx, Key::S);
    assert!(!f.app.comma_menu.open);
    assert!(!crate::store::auto_save::enabled(
        &f.app.loaded.as_ref().unwrap().music
    ));
}
