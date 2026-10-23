use super::*;

#[test]
fn native_selection_cancel_preserves_edits_and_selection_uses_departure_confirmation() {
    let mut f = fixture("native-selection");
    let ctx = egui::Context::default();
    edit(&mut f.app);
    f.app.cursor_row = Some(1);
    let original = f.app.loaded.as_ref().unwrap().original.clone();
    crate::view::files::open::complete(&mut f.app, None);
    assert_eq!(f.app.cursor_row, Some(1));
    assert!(f.app.saved.dirty());
    assert!(f.app.config.navigation.departure.is_none());
    crate::view::files::open::complete(&mut f.app, Some(f.second.clone()));
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::LoadAndPlay(_))
    ));
    assert_eq!(f.app.loaded.as_ref().unwrap().original, original);
    press(&mut f.app, &ctx, Key::Escape);
    assert!(f.app.saved.dirty());
    crate::view::files::open::complete(&mut f.app, Some(f.second.clone()));
    press(&mut f.app, &ctx, Key::N);
    crate::view::opening::tests::finish(&mut f.app, &ctx);
    assert_eq!(
        f.app.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&f.second).unwrap()
    );
    assert_eq!(f.app.playback.last, crate::play::playback::Mode::AllTracks);
    assert!(f.app.playback.active.is_some());
    assert_eq!(
        smf_io::load(&f.first).unwrap().tracks[0].events[0].data[0],
        60
    );
}

#[test]
fn native_selection_starts_all_tracks_only_after_a_successful_load() {
    let mut f = fixture("native-autoplay");
    let ctx = egui::Context::default();
    crate::view::files::open::complete(&mut f.app, Some(f.second.clone()));
    assert!(crate::view::opening::pending(&f.app));
    assert!(f.app.playback.pending.is_none());
    crate::view::opening::tests::finish(&mut f.app, &ctx);
    assert_eq!(f.app.playback.last, crate::play::playback::Mode::AllTracks);
    assert!(f.app.playback.active.is_some());
    crate::play::transport::stop(&mut f.app);
    crate::view::files::open::complete(&mut f.app, None);
    assert!(f.app.playback.pending.is_none());
    let invalid = f.root.join("invalid.mid");
    std::fs::write(&invalid, b"invalid MIDI").unwrap();
    crate::view::files::open::complete(&mut f.app, Some(invalid));
    crate::view::opening::tests::finish(&mut f.app, &ctx);
    assert!(f.app.load_error.is_some());
    assert!(f.app.playback.pending.is_none());
}
