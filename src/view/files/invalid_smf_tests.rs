use super::*;

#[test]
fn invalid_drop_shows_filename_and_japanese_dialog_and_owns_keyboard() {
    for key in [Key::Enter, Key::Escape] {
        let mut f = fixture(&format!("invalid-drop-{key:?}"));
        let ctx = egui::Context::default();
        let path = f.root.join("誤ったファイル.md");
        std::fs::write(&path, b"# markdown, not MIDI").unwrap();
        let original = f.app.loaded.as_ref().unwrap().original.clone();
        let _ = ctx.run(
            egui::RawInput {
                dropped_files: vec![egui::DroppedFile {
                    path: Some(path),
                    ..Default::default()
                }],
                ..Default::default()
            },
            |ctx| crate::view::show(&mut f.app, ctx),
        );
        assert_eq!(f.app.invalid_smf_name.as_deref(), Some("誤ったファイル.md"));
        // egui measures a newly opened modal before painting it on the next frame.
        let output = ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx));
        let text: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape {
                    Some(text.galley.text())
                } else {
                    None
                }
            })
            .collect();
        assert!(text.contains(&"誤ったファイル.md"));
        assert!(text.contains(&"このファイルはスタンダードMIDIファイルとして認識できませんでした。ファイルを見直してください"));
        assert!(!text.iter().any(|s| s.contains("truncated")));
        let row = f.app.cursor_row;
        press(&mut f.app, &ctx, Key::ArrowDown);
        assert_eq!(f.app.cursor_row, row);
        f.app.help_open = true;
        press(&mut f.app, &ctx, key);
        assert!(f.app.invalid_smf_name.is_none());
        assert!(f.app.load_error.is_none());
        assert!(f.app.help_open);
        assert_eq!(f.app.loaded.as_ref().unwrap().original, original);
    }
}

#[test]
fn malformed_mid_after_discard_keeps_pending_edits_and_dismisses_error() {
    let mut f = fixture("malformed-mid-dialog");
    let ctx = egui::Context::default();
    edit(&mut f.app);
    std::fs::write(&f.second, b"MThd\0\0").unwrap();
    f.app.open_mid(f.second.clone());
    press(&mut f.app, &ctx, Key::N);
    assert_eq!(f.app.invalid_smf_name.as_deref(), Some("second.mid"));
    assert!(f.app.saved.dirty());
    press(&mut f.app, &ctx, Key::Escape);
    assert!(f.app.invalid_smf_name.is_none());
    assert!(f.app.saved.dirty());
}
