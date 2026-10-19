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
    fn empty_startup_requests_dialog_once_and_cancel_allows_manual_reopen() {
        let ctx = egui::Context::default();
        let mut app = App::empty();
        app.audition = crate::play::audition::Audition::disabled();
        app.open_initial(None);
        assert!(std::mem::take(&mut app.file_open_requested));
        complete(&mut app, None);
        for _ in 0..2 {
            let output = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
            assert!(!app.file_open_requested);
            assert!(!output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::epaint::Shape::Text(text)
                    if text.galley.text().contains("Stopped") || text.galley.text() == "Tracks")
            }));
        }
        assert!(app.loaded.is_none());
        press(&mut app, &ctx, Modifiers::CTRL, false);
        assert!(app.file_open_requested);
    }

    #[test]
    fn failed_startup_load_requests_dialog_and_keeps_error() {
        for explicit in [false, true] {
            let mut app = App::empty();
            app.audition = crate::play::audition::Audition::disabled();
            let path = PathBuf::from("X:/missing-cat-smf-editor-startup/song.mid");
            app.config.navigation.last_mid = Some(path.clone());
            app.open_initial(explicit.then_some(path));
            assert!(app.loaded.is_none());
            assert!(app.file_open_requested);
            assert!(app.load_error.is_some());
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
