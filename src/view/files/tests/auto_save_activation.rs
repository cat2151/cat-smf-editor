use super::*;

fn request(f: &mut Fixture, ctx: &egui::Context) {
    press(&mut f.app, ctx, Key::Comma);
    press(&mut f.app, ctx, Key::S);
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::EnableAutoSave)
    ));
    assert!(!f.app.comma_menu.open);
    assert!(!crate::store::auto_save::enabled(
        &f.app.loaded.as_ref().unwrap().music
    ));
}

#[test]
fn activation_cancel_keeps_edits_and_history_without_writing() {
    for key in [Key::N, Key::Escape] {
        let mut f = fixture(&format!("activation-cancel-{key:?}"));
        let ctx = egui::Context::default();
        edit(&mut f.app);
        let before = smf_io::encode(&f.app.loaded.as_ref().unwrap().music).unwrap();
        let original = std::fs::read(&f.first).unwrap();
        request(&mut f, &ctx);
        press(&mut f.app, &ctx, key);
        assert!(f.app.config.navigation.departure.is_none());
        assert_eq!(
            smf_io::encode(&f.app.loaded.as_ref().unwrap().music).unwrap(),
            before
        );
        assert_eq!(std::fs::read(&f.first).unwrap(), original);
        assert!(!f.root.join("config/backup").exists());
        f.app.undo_music().unwrap();
        assert!(!f.app.saved.dirty());
        assert!(!f.app.undo.available());
    }
}

#[test]
fn activation_failure_keeps_off_and_can_cancel_or_retry_without_losing_edits() {
    for cancel in [false, true] {
        let mut f = fixture(&format!("activation-failed-{cancel}"));
        let ctx = egui::Context::default();
        edit(&mut f.app);
        let original = std::fs::read(&f.first).unwrap();
        let edited = smf_io::encode(&f.app.loaded.as_ref().unwrap().music).unwrap();
        std::fs::create_dir_all(f.root.join("config")).unwrap();
        std::fs::write(f.root.join("config/backup"), b"blocked").unwrap();
        request(&mut f, &ctx);
        press(&mut f.app, &ctx, Key::Y);
        assert!(f.app.config.navigation.departure_error.is_some());
        assert!(!f.app.saved.auto_save_enabled());
        assert_eq!(
            smf_io::encode(&f.app.loaded.as_ref().unwrap().music).unwrap(),
            edited
        );
        assert_eq!(std::fs::read(&f.first).unwrap(), original);
        assert!(f.app.saved.dirty());
        if cancel {
            press(&mut f.app, &ctx, Key::Escape);
            assert!(f.app.config.navigation.departure.is_none());
            assert!(!crate::store::auto_save::enabled(
                &f.app.loaded.as_ref().unwrap().music
            ));
            f.app.undo_music().unwrap();
            assert!(!f.app.saved.dirty());
        } else {
            std::fs::remove_file(f.root.join("config/backup")).unwrap();
            press(&mut f.app, &ctx, Key::Y);
            assert!(f.app.config.navigation.departure.is_none());
            assert!(!f.app.saved.dirty());
            assert!(f.app.saved.auto_save_enabled());
            let mut restored = smf_io::load(&f.first).unwrap();
            assert!(crate::store::auto_save::enabled(&restored));
            restored.tracks[0].events.remove(0);
            assert_eq!(smf_io::encode(&restored).unwrap(), edited);
            let backups: Vec<_> = std::fs::read_dir(f.root.join("config/backup"))
                .unwrap()
                .map(|entry| std::fs::read(entry.unwrap().path()).unwrap())
                .collect();
            assert_eq!(backups, vec![original]);
        }
    }
}
