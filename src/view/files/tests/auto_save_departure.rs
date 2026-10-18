use super::*;

fn enable(f: &mut Fixture, ctx: &egui::Context) {
    press(&mut f.app, ctx, Key::Comma);
    press(&mut f.app, ctx, Key::S);
    press(&mut f.app, ctx, Key::Y);
    assert!(f.app.saved.auto_save_enabled());
}

#[test]
fn manual_save_and_file_switch_flush_without_confirmation_and_reopen_stays_enabled() {
    let mut f = fixture("auto-save-departure");
    let ctx = egui::Context::default();
    enable(&mut f, &ctx);
    edit(&mut f.app);
    save(&mut f.app, &ctx);
    assert!(!f.app.saved.dirty());
    assert!(f.app.config.navigation.departure.is_none());
    assert!(crate::store::auto_save::enabled(
        &smf_io::load(&f.first).unwrap()
    ));
    crate::play::mixer::volume::set(&mut f.app, 0, 0.5).unwrap();
    f.app.open_mid(f.second.clone());
    assert!(f.app.config.navigation.departure.is_none());
    assert_eq!(
        f.app.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&f.second).unwrap()
    );
    let saved = smf_io::load(&f.first).unwrap();
    assert_eq!(crate::play::track_volume::read(&saved.tracks[0]), Ok(0.5));
    assert!(!f.app.saved.auto_save_enabled());
    f.app.open_mid(f.first.clone());
    assert!(f.app.saved.auto_save_enabled());
    crate::play::mixer::volume::set(&mut f.app, 0, 0.6).unwrap();
    super::open::complete(&mut f.app, Some(f.second.clone()));
    assert!(f.app.config.navigation.departure.is_none());
    assert_eq!(f.app.playback.last, crate::play::playback::Mode::AllTracks);
    assert!(f.app.playback.pending.is_some());
    assert_eq!(
        crate::play::track_volume::read(&smf_io::load(&f.first).unwrap().tracks[0]),
        Ok(0.6)
    );
}

#[test]
fn immediate_exit_after_activation_has_no_prompt_or_extra_backup() {
    let mut f = fixture("auto-save-immediate-exit");
    let ctx = egui::Context::default();
    enable(&mut f, &ctx);
    let before = std::fs::read(&f.first).unwrap();
    super::clean_close::close_request(&mut f.app, &ctx);
    assert!(f.app.config.navigation.departure.is_none());
    // Approved without confirmation; the busy modal is painted before closing.
    assert!(f.app.config.navigation.close_approved);
    assert_eq!(std::fs::read(&f.first).unwrap(), before);
    assert_eq!(
        std::fs::read_dir(f.root.join("config/backup"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn failed_exit_blocks_close_and_supports_retry_discard_and_resume() {
    for key in [Key::Y, Key::N, Key::Escape] {
        let mut f = fixture(&format!("auto-save-close-failure-{key:?}"));
        let ctx = egui::Context::default();
        enable(&mut f, &ctx);
        edit(&mut f.app);
        let original = std::fs::read(&f.first).unwrap();
        std::fs::write(&f.first, b"external change").unwrap();
        let output = super::clean_close::close_request(&mut f.app, &ctx);
        assert!(output
            .viewport_output
            .values()
            .any(|v| v.commands.contains(&egui::ViewportCommand::CancelClose)));
        assert!(matches!(
            f.app.config.navigation.departure,
            Some(Action::Close)
        ));
        assert!(f.app.config.navigation.departure_error.is_some());
        assert!(f.app.saved.dirty());
        assert_eq!(std::fs::read(&f.first).unwrap(), b"external change");
        if key == Key::Y {
            std::fs::write(&f.first, &original).unwrap();
        }
        let output = press(&mut f.app, &ctx, key);
        assert!(f.app.config.navigation.departure.is_none());
        assert_eq!(f.app.config.navigation.close_approved, key != Key::Escape);
        assert_eq!(
            output
                .viewport_output
                .values()
                .any(|v| v.commands.contains(&egui::ViewportCommand::Close)),
            key != Key::Escape
        );
        if key == Key::Y {
            assert!(!f.app.saved.dirty());
            assert!(f.app.saved.error.is_none());
            assert_ne!(std::fs::read(&f.first).unwrap(), original);
        } else {
            assert_eq!(std::fs::read(&f.first).unwrap(), b"external change");
            assert!(f.app.saved.dirty());
        }
    }
}

#[test]
fn failed_switch_and_manual_save_keep_document_until_retry_or_cancel() {
    for switching in [false, true] {
        let mut f = fixture(&format!("auto-save-action-failure-{switching}"));
        let ctx = egui::Context::default();
        enable(&mut f, &ctx);
        edit(&mut f.app);
        let before = std::fs::read(&f.first).unwrap();
        std::fs::write(&f.first, b"external").unwrap();
        if switching {
            f.app.open_mid(f.second.clone());
        } else {
            save(&mut f.app, &ctx);
        }
        assert!(f.app.config.navigation.departure_error.is_some());
        assert_eq!(
            f.app.loaded.as_ref().unwrap().original,
            std::fs::canonicalize(&f.first).unwrap()
        );
        press(&mut f.app, &ctx, Key::Escape);
        assert!(f.app.config.navigation.departure.is_none());
        assert!(f.app.saved.dirty());
        std::fs::write(&f.first, before).unwrap();
        if switching {
            f.app.open_mid(f.second.clone());
        } else {
            save(&mut f.app, &ctx);
        }
        assert!(f.app.config.navigation.departure.is_none());
        assert!(!f.app.saved.dirty());
        assert!(crate::store::auto_save::enabled(
            &smf_io::load(&f.first).unwrap()
        ));
    }
}
