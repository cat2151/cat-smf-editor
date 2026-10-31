use super::*;

#[test]
fn title_marks_unsaved_edits_and_clears_after_save_or_restoring_gain() {
    let mut f = fixture("dirty-title");
    let ctx = egui::Context::default();
    let title = |output: egui::FullOutput| {
        output
            .viewport_output
            .into_values()
            .flat_map(|v| v.commands)
            .find_map(|command| match command {
                egui::ViewportCommand::Title(title) => Some(title),
                _ => None,
            })
            .unwrap()
    };
    let clean = "cat-smf-editor · first.mid · 1tracks";
    let dirty = "cat-smf-editor · first.mid(更新) · 1tracks";
    assert_eq!(
        title(ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx))),
        clean
    );
    edit(&mut f.app);
    assert_eq!(
        title(ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx))),
        dirty
    );
    assert_eq!(title(save(&mut f.app, &ctx)), clean);
    for (gain, expected) in [(0.5, dirty), (1.0, clean)] {
        crate::play::mixer::volume::set(&mut f.app, 0, gain).unwrap();
        f.app.remember_loaded().unwrap();
        assert_eq!(
            title(ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx))),
            expected
        );
    }
}

pub(super) fn close_request(app: &mut App, ctx: &egui::Context) -> egui::FullOutput {
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    ctx.run(input, |ctx| {
        crate::view::show(app, ctx);
        app.close(ctx);
    })
}

#[test]
fn unedited_native_close_has_no_confirmation_or_file_writes() {
    let mut f = fixture("clean-native-close");
    let ctx = egui::Context::default();
    let original = std::fs::read(&f.first).unwrap();
    close_request(&mut f.app, &ctx);
    assert!(!f.app.saved.dirty());
    assert!(f.app.config.navigation.departure.is_none());
    // Approved without confirmation; the busy modal is painted before closing.
    assert!(f.app.config.navigation.close_approved);
    assert_eq!(std::fs::read(&f.first).unwrap(), original);
    assert!(!f.root.join("config/backup").exists());
}

#[test]
fn saved_edits_close_without_another_confirmation() {
    let mut f = fixture("saved-close");
    let ctx = egui::Context::default();
    edit(&mut f.app);
    save(&mut f.app, &ctx);
    press(&mut f.app, &ctx, Key::Y);
    assert!(!f.app.saved.dirty());
    close_request(&mut f.app, &ctx);
    assert!(f.app.config.navigation.departure.is_none());
    // Approved without confirmation; the busy modal is painted before closing.
    assert!(f.app.config.navigation.close_approved);
}

#[test]
fn restoring_original_mixer_gain_clears_the_exit_confirmation() {
    let mut f = fixture("restored-gain-close");
    crate::play::mixer::volume::set(&mut f.app, 0, 0.5).unwrap();
    f.app.remember_loaded().unwrap();
    assert!(f.app.saved.dirty());
    crate::play::mixer::volume::set(&mut f.app, 0, 1.0).unwrap();
    assert!(f.app.allow_departure(Action::Close));
    assert!(!f.app.saved.dirty());
    assert!(f.app.config.navigation.departure.is_none());
}

#[test]
fn exit_confirmation_answers_while_the_keyboard_window_is_open() {
    for key in [Key::Y, Key::N, Key::Escape] {
        let mut f = fixture(&format!("close-while-recording-{key:?}"));
        let ctx = egui::Context::default();
        edit(&mut f.app);
        let shift_r = egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::R,
                physical_key: Some(Key::R),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            }],
            ..Default::default()
        };
        let _ = ctx.run(shift_r, |ctx| crate::view::show(&mut f.app, ctx));
        assert!(f.app.patterns.recording.is_open(), "{:?}", f.app.load_error);
        close_request(&mut f.app, &ctx);
        assert!(matches!(
            f.app.config.navigation.departure,
            Some(Action::Close)
        ));
        press(&mut f.app, &ctx, key);
        assert!(
            f.app.config.navigation.departure.is_none(),
            "{key:?} left the exit confirmation open"
        );
        assert_eq!(f.app.config.navigation.close_approved, key != Key::Escape);
    }
}
