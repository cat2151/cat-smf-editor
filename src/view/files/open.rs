//! Keep the native dialog at the eframe boundary so it can own the editor window.
use crate::app::App;
use eframe::egui;
#[cfg(test)]
use egui::Key;
use std::path::PathBuf;

pub fn open_dialog(app: &mut App, ctx: &egui::Context, frame: &eframe::Frame) {
    if !std::mem::take(&mut app.file_open_requested) {
        return;
    }
    // A synchronous modal dialog suspends the transport pump, so release voices first.
    crate::play::transport::stop(app);
    let mut dialog = rfd::FileDialog::new()
        .set_parent(frame)
        .set_title("Open MIDI file")
        .add_filter("Standard MIDI File", &["mid", "midi", "smf"])
        .add_filter("All files", &["*"]);
    if let Some(directory) = app
        .loaded
        .as_ref()
        .and_then(|loaded| loaded.original.parent())
    {
        dialog = dialog.set_directory(directory);
    }
    complete(app, dialog.pick_file());
    super::super::comma_menu::swallow(ctx);
    ctx.request_repaint();
}

pub(super) fn complete(app: &mut App, selected: Option<PathBuf>) {
    if let Some(path) = selected {
        app.open_mid_and_play(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Modifiers;

    #[test]
    fn empty_startup_creates_new_song_instead_of_dialog() {
        let ctx = egui::Context::default();
        let mut app = App::empty();
        app.audition = crate::play::audition::Audition::disabled();
        app.open_initial(None);
        assert!(!app.file_open_requested);
        assert!(app.saved.is_untitled());
        assert_eq!(app.loaded.as_ref().unwrap().name, "untitled.mid");
        assert!(app.load_error.is_none());
        press(&mut app, &ctx, Modifiers::CTRL, false);
        assert!(app.file_open_requested);
    }

    #[test]
    fn missing_argument_requests_dialog_and_keeps_error() {
        let mut app = App::empty();
        app.audition = crate::play::audition::Audition::disabled();
        let path = PathBuf::from("X:/missing-cat-smf-editor-startup/song.mid");
        app.open_initial(Some(path));
        assert!(app.loaded.is_none());
        assert!(app.file_open_requested);
        assert!(app.load_error.is_some());
        assert!(app.missing_last_mid.is_none());
    }

    #[test]
    fn missing_last_mid_asks_new_song_or_quit() {
        for (key, creates) in [(Key::N, true), (Key::Q, false)] {
            let ctx = egui::Context::default();
            let mut app = App::empty();
            app.audition = crate::play::audition::Audition::disabled();
            let path = PathBuf::from("X:/missing-cat-smf-editor-startup/song.mid");
            app.config.navigation.last_mid = Some(path.clone());
            app.config.analysis_open = true;
            app.open_initial(None);
            crate::view::analysis::restore(&mut app);
            assert!(!app.analysis.is_open());
            assert!(app.loaded.is_none());
            assert!(!app.file_open_requested);
            assert_eq!(app.missing_last_mid, Some(path));
            // A modal's first pass is invisible.
            let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
            let output = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
            assert!(output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::epaint::Shape::Text(text)
                    if text.galley.text().contains("song.mid"))
            }));
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ctx| crate::view::show(&mut app, ctx),
            );
            assert_eq!(app.missing_last_mid.is_none(), creates);
            assert_eq!(app.saved.is_untitled() && app.loaded.is_some(), creates);
            assert_eq!(app.config.navigation.close_approved, !creates);
            assert_eq!(app.analysis.is_open(), creates);
        }
    }

    fn press(app: &mut App, ctx: &egui::Context, modifiers: Modifiers, repeat: bool) {
        let mut events = vec![egui::Event::Key {
            key: Key::O,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers,
        }];
        if !repeat {
            events.insert(
                0,
                egui::Event::Key {
                    key: Key::O,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers,
                },
            );
        }
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        );
    }

    #[test]
    fn shortcut_does_not_open_history_or_leak_into_other_editors() {
        let ctx = egui::Context::default();
        let mut app = App::empty();
        app.audition = crate::play::audition::Audition::disabled();
        app.help_open = true;
        press(&mut app, &ctx, Modifiers::CTRL, false);
        assert!(!app.file_open_requested);
        app.help_open = false;
        app.layer_editor.open = true;
        press(&mut app, &ctx, Modifiers::CTRL, false);
        assert!(!app.file_open_requested);
        app.layer_editor.open = false;
        press(&mut app, &ctx, Modifiers::CTRL | Modifiers::SHIFT, false);
        assert!(!app.file_open_requested);
        press(&mut app, &ctx, Modifiers::CTRL, true);
        assert!(!app.file_open_requested);
        press(&mut app, &ctx, Modifiers::CTRL, false);
        assert!(app.file_open_requested);
        assert!(!app.recent.open && !app.playback.open);
        app.file_open_requested = false;
        press(
            &mut app,
            &ctx,
            Modifiers {
                command: true,
                ..Modifiers::CTRL
            },
            false,
        );
        assert!(app.file_open_requested);
        app.file_open_requested = false;
        app.recent.open = true;
        press(&mut app, &ctx, Modifiers::CTRL, false);
        assert!(!app.file_open_requested);
    }
}
