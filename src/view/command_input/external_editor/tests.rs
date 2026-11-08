use crate::store::external_editor::Editor;
use crate::view::{
    commands,
    palette::tests::{app, choose, frame, open, press},
};
use eframe::egui::{self, Key};

#[test]
fn external_editor_keyboard_registration_cancel_ime_and_restore() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("日本語 editor.exe");
    std::fs::write(&executable, b"fixture").unwrap();
    let mut a = app();
    a.config.path = Some(directory.path().join("status.json"));
    a.external_editor = Editor::open(directory.path()).unwrap();
    let ctx = egui::Context::default();
    let ids = |a: &crate::app::App| {
        commands::definitions(a, commands::Screen::Global)
            .into_iter()
            .map(|c| c.id)
            .collect::<Vec<_>>()
    };
    assert!(!ids(&a).contains(&"config.edit-layout".into()));
    open(&mut a, &ctx);
    choose(&mut a, &ctx, "editor.register");
    press(&mut a, &ctx, Key::Enter);
    assert!(a.command_input.is_open());
    frame(&mut a, &ctx, vec![]);
    frame(
        &mut a,
        &ctx,
        vec![egui::Event::Paste(executable.to_string_lossy().into())],
    );
    frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::Ime(egui::ImeEvent::Commit("".into())),
            egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(a.command_input.is_open());
    press(&mut a, &ctx, Key::Enter);
    assert!(!a.command_input.is_open());
    assert_eq!(a.external_editor.executable, Some(executable.clone()));
    a.external_editor = Editor::open(directory.path()).unwrap();
    assert_eq!(a.external_editor.executable, Some(executable.clone()));
    assert!(ids(&a).contains(&"config.edit-layout".into()));
    assert!(ids(&a).contains(&"config.edit-settings".into()));
    assert_eq!(
        ids(&a).contains(&"config.edit-command-palette".into()),
        commands::descriptions::external_path().is_some()
    );
    open(&mut a, &ctx);
    choose(&mut a, &ctx, "editor.register");
    press(&mut a, &ctx, Key::Enter);
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![egui::Event::Text("invalid".into())]);
    press(&mut a, &ctx, Key::Enter);
    assert!(a.command_input.is_open());
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.command_input.is_open());
    assert_eq!(
        Editor::open(directory.path()).unwrap().executable,
        Some(executable)
    );
}

#[test]
fn external_editor_save_clear_and_failure_preserve_other_settings() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("settings.toml");
    let layout = directory.path().join("layout.toml");
    std::fs::write(&settings, "# keep settings").unwrap();
    std::fs::write(&layout, "# keep layout").unwrap();
    let mut editor = Editor::open(directory.path()).unwrap();
    let executable = std::env::current_exe().unwrap();
    editor.save(executable.to_str().unwrap()).unwrap();
    assert!(editor.save("X:/missing/editor.exe").is_err());
    assert!(editor.save("relative.exe").is_err());
    assert_eq!(
        Editor::open(directory.path()).unwrap().executable,
        Some(executable)
    );
    editor.save("").unwrap();
    assert!(Editor::open(directory.path()).unwrap().executable.is_none());
    assert_eq!(
        std::fs::read_to_string(settings).unwrap(),
        "# keep settings"
    );
    assert_eq!(std::fs::read_to_string(layout).unwrap(), "# keep layout");
}

#[test]
fn palette_loads_registration_on_first_use_and_launches_after_closing() {
    let directory = tempfile::tempdir().unwrap();
    let mut a = app();
    a.config.path = Some(directory.path().join("status.json"));
    a.layout = crate::store::layout::Watch::open(directory.path());
    // This file appears after App construction: startup must not decide availability.
    let executable = directory.path().join("missing-editor.exe");
    let preference = directory.path().join("external-editor.json");
    std::fs::write(&preference, serde_json::to_vec(&Some(&executable)).unwrap()).unwrap();
    let ctx = egui::Context::default();
    assert!(a.external_editor.executable.is_none());
    open(&mut a, &ctx);
    assert_eq!(a.external_editor.executable, Some(executable.clone()));
    choose(&mut a, &ctx, "config.edit-layout");
    press(&mut a, &ctx, Key::Enter);
    assert!(!a.palette.is_open());
    assert!(
        a.load_error.is_none(),
        "launch must wait until native focus restoration finishes"
    );
    frame(&mut a, &ctx, vec![]);
    assert!(a.load_error.as_ref().unwrap().contains("layout.toml"));
    a.load_error = None;
    std::fs::write(&preference, "broken").unwrap();
    open(&mut a, &ctx);
    assert!(
        a.load_error.is_none(),
        "opening again must reuse the loaded preference"
    );
    assert_eq!(a.external_editor.executable, Some(executable));
    choose(&mut a, &ctx, "config.edit-settings");
    press(&mut a, &ctx, Key::Escape);
}
