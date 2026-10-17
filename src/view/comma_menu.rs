use crate::{app::App, store::config::ProgramTarget as Target};
use eframe::egui;

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// `,t` was pressed: the overlay lists the program change targets.
    program: bool,
}

pub fn open(app: &mut App) {
    app.comma_menu = State {
        open: true,
        program: false,
    };
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if app.edit_menu.open || ctx.wants_keyboard_input() {
        return;
    }
    if !app.comma_menu.open {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Comma)) {
            open(app);
            swallow(ctx);
        }
        return;
    }
    for key in pressed(ctx) {
        match (app.comma_menu.program, key) {
            (_, egui::Key::Escape) => app.comma_menu.open = false,
            (false, egui::Key::S) => app.toggle_auto_save(),
            (false, egui::Key::M) => {
                let maximized = !app.config.tempo_curve_maximized;
                app.config.select_tempo_curve_maximized(maximized);
            }
            (false, egui::Key::R) => {
                let maximized = !app.config.piano_roll_maximized;
                app.config.select_piano_roll_maximized(maximized);
            }
            (false, egui::Key::I) => {
                app.config
                    .select_meta_text_icons(!app.config.meta_text_icons);
            }
            (false, egui::Key::A) => {
                app.config
                    .select_meta_text_visible(!app.config.meta_text_visible);
            }
            (false, egui::Key::T) => app.comma_menu.program = true,
            (false, egui::Key::F) => crate::view::analysis::toggle(app),
            (false, egui::Key::P) => crate::view::analysis::reset_window(app),
            (true, egui::Key::A) => choose(app, Target::Auto),
            (true, egui::Key::C) => choose(app, Target::Current),
            _ => {}
        }
    }
    // The menu owns the keyboard while open, so its letters never reach other shortcuts.
    swallow(ctx);
}

/// Plain key presses of this frame.
pub(crate) fn pressed(ctx: &egui::Context) -> Vec<egui::Key> {
    ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                    ..
                } => Some(*key),
                _ => None,
            })
            .collect()
    })
}

pub(crate) fn swallow(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        i.events
            .retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_)))
    });
}

fn choose(app: &mut App, target: Target) {
    app.config.select_program_target(target);
    app.comma_menu.open = false;
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.comma_menu.open {
        return;
    }
    let mut open = true;
    egui::Window::new("help")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            if app.comma_menu.program {
                let current = app.config.program_target;
                for (target, label) in [
                    (Target::Auto, "a: auto"),
                    (Target::Current, "c: current track"),
                ] {
                    if ui.selectable_label(current == target, label).clicked() {
                        choose(app, target);
                    }
                }
            } else {
                let mut auto_save = app
                    .loaded
                    .as_ref()
                    .is_some_and(|loaded| crate::store::auto_save::enabled(&loaded.music));
                if ui
                    .add_enabled(
                        app.loaded.is_some(),
                        egui::Checkbox::new(&mut auto_save, "s: このファイルのAuto save"),
                    )
                    .changed()
                {
                    app.toggle_auto_save();
                }
                ui.separator();
                let mut maximized = app.config.tempo_curve_maximized;
                if ui
                    .checkbox(&mut maximized, "m: Tempo curve always maximized")
                    .changed()
                {
                    app.config.select_tempo_curve_maximized(maximized);
                }
                let mut maximized = app.config.piano_roll_maximized;
                if ui
                    .checkbox(&mut maximized, "r: Piano roll always maximized")
                    .changed()
                {
                    app.config.select_piano_roll_maximized(maximized);
                }
                let mut icons = app.config.meta_text_icons;
                if ui.checkbox(&mut icons, "i: Meta text icons").changed() {
                    app.config.select_meta_text_icons(icons);
                }
                let mut visible = app.config.meta_text_visible;
                if ui
                    .checkbox(&mut visible, "a: All tracks meta text")
                    .changed()
                {
                    app.config.select_meta_text_visible(visible);
                }
                if ui.button("t: program change target").clicked() {
                    app.comma_menu.program = true;
                }
                ui.separator();
                let mut analysis = app.analysis.is_open();
                if ui
                    .checkbox(&mut analysis, "f: Spectrum / Lissajous")
                    .changed()
                {
                    crate::view::analysis::toggle(app);
                }
                if ui.button("p: Reset spectrum window position").clicked() {
                    crate::view::analysis::reset_window(app);
                }
            }
        });
    if !open {
        app.comma_menu.open = false;
    }
}

#[cfg(test)]
mod tests;
