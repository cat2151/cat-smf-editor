//! Settings and piano windows beside the keyboard; opening one leaves the keyboard focused.
use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Places {
    settings: Option<egui::Pos2>,
    piano: Option<egui::Pos2>,
}

/// Keys of `config.keyboard_windows`.
pub(super) const KEYBOARD: &str = "Keyboard";
const SETTINGS: &str = "Settings";
const PIANO: &str = "Piano";
pub(super) const SETTINGS_TITLE: &str = "Virtual MIDI keyboard · 設定";
pub(super) const PIANO_TITLE: &str = "Virtual MIDI keyboard · Piano";

pub(super) fn saved(a: &App, window: &str) -> Option<egui::Pos2> {
    a.config
        .keyboard_windows
        .get(window)
        .map(|&[x, y]| egui::pos2(x, y))
}

/// Persisted on app exit, rather than rewriting status throughout a drag.
pub(super) fn remember(a: &mut App, window: &str, at: egui::Pos2) {
    if at.is_finite() {
        a.config
            .keyboard_windows
            .insert(window.to_owned(), [at.x, at.y]);
    }
}

/// The next opening places every window afresh.
pub(crate) fn reset(a: &mut App, _: &egui::Context) {
    a.config.keyboard_windows.clear();
    a.config.persist();
}

pub(crate) fn settings_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("virtual-midi-keyboard-settings")
}
pub(crate) fn piano_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("virtual-midi-keyboard-piano")
}

const SETTINGS_SIZE: egui::Vec2 = egui::vec2(420.0, 330.0);

/// Beside the keyboard and the other companion, overlapping the pattern windows least.
fn place(
    a: &App,
    ctx: &egui::Context,
    keyboard: egui::Pos2,
    other: Option<(egui::Pos2, egui::Vec2)>,
    size: egui::Vec2,
) -> egui::Pos2 {
    let screen = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        ctx.input(|i| i.viewport().monitor_size)
            .unwrap_or(egui::vec2(1920.0, 1080.0)),
    );
    let mut covers = window::covers(a, ctx);
    covers.push(egui::Rect::from_min_size(
        keyboard,
        window::SIZE + window::FRAME,
    ));
    covers.extend(other.map(|(at, size)| egui::Rect::from_min_size(at, size + window::FRAME)));
    window::place(screen, &covers, size + window::FRAME)
}

/// The saved place follows where each window is moved, so it reopens there.
pub(super) fn show(a: &mut App, ctx: &egui::Context, s: &mut Session, keyboard: egui::Pos2) {
    let raise = std::mem::take(&mut s.raise);
    let moved = |ctx: &egui::Context| ctx.input(|i| i.viewport().outer_rect).map(|r| r.min);
    if !s.settings_open {
        s.companions.settings = None;
    }
    if !s.piano_open {
        s.companions.piano = None;
    }
    if s.settings_open {
        let other = s.companions.piano.map(|at| (at, piano::SIZE));
        let at = s
            .companions
            .settings
            .or(saved(a, SETTINGS))
            .unwrap_or_else(|| place(a, ctx, keyboard, other, SETTINGS_SIZE));
        s.companions.settings = Some(at);
        let builder = builder(SETTINGS_TITLE, at, SETTINGS_SIZE);
        ctx.show_viewport_immediate(settings_viewport(), builder, |ctx, class| {
            if class != egui::ViewportClass::Embedded {
                if ctx.input(|i| i.viewport().close_requested()) {
                    s.settings_open = false;
                    return;
                }
                if raise {
                    super::super::help::lift(ctx);
                }
                if let Some(at) = moved(ctx) {
                    remember(a, SETTINGS, at);
                }
                super::super::help::observe_recording(a, ctx);
                if s.first_read(ctx) {
                    settings_keys(a, s, &ctx.input(|i| i.events.clone()));
                }
            }
            frame(ctx, class, SETTINGS_TITLE, |ui| settings(ui, s));
        });
    }
    if s.piano_open {
        let other = s.companions.settings.map(|at| (at, SETTINGS_SIZE));
        let at = s
            .companions
            .piano
            .or(saved(a, PIANO))
            .unwrap_or_else(|| place(a, ctx, keyboard, other, piano::SIZE));
        s.companions.piano = Some(at);
        let builder = builder(PIANO_TITLE, at, piano::SIZE);
        ctx.show_viewport_immediate(piano_viewport(), builder, |ctx, class| {
            if class != egui::ViewportClass::Embedded
                && ctx.input(|i| i.viewport().close_requested())
            {
                s.piano_open = false;
                return;
            }
            if class != egui::ViewportClass::Embedded {
                super::super::help::observe_recording(a, ctx);
                if raise {
                    super::super::help::lift(ctx);
                }
                if let Some(at) = moved(ctx) {
                    remember(a, PIANO, at);
                }
            }
            frame(ctx, class, PIANO_TITLE, |ui| piano::show(ui, s));
        });
    }
}

fn builder(title: &str, at: egui::Pos2, size: egui::Vec2) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(title)
        .with_active(false)
        .with_position(at)
        .with_inner_size(size)
}

fn frame(
    ctx: &egui::Context,
    class: egui::ViewportClass,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    if class == egui::ViewportClass::Embedded {
        egui::Window::new(title).show(ctx, body);
    } else {
        egui::CentralPanel::default().show(ctx, body);
    }
}

/// Keys of the focused settings window: rows and values by arrows, plus every window key.
pub(super) fn settings_keys(a: &mut App, s: &mut Session, events: &[egui::Event]) {
    for event in events {
        let &egui::Event::Key {
            key,
            pressed: true,
            repeat,
            modifiers,
            ..
        } = event
        else {
            continue;
        };
        let shift = commands::matches(modifiers, Modifiers::SHIFT);
        let plain = commands::matches(modifiers, Modifiers::NONE);
        match key {
            Key::Escape if plain && !repeat => s.settings_open = false,
            Key::ArrowUp | Key::ArrowDown if plain => {
                s.item = setup::step_item(s, key == Key::ArrowDown)
            }
            Key::ArrowLeft | Key::ArrowRight if plain || shift => {
                let step = if key == Key::ArrowLeft { -1 } else { 1 };
                setup::change(a, s, s.item, step, shift);
            }
            _ if !repeat => {
                play::window_key(a, s, key, modifiers);
            }
            _ => {}
        }
    }
}

fn settings(ui: &mut egui::Ui, s: &mut Session) {
    egui::Grid::new("recording-setup").show(ui, |ui| {
        for item in setup::ITEMS {
            let enabled = setup::enabled(s, item);
            let color = if enabled {
                ui.visuals().text_color()
            } else {
                ui.visuals().weak_text_color()
            };
            let label = egui::RichText::new(item.label()).color(color);
            if ui.selectable_label(item == s.item, label).clicked() && enabled {
                s.item = item;
            }
            ui.colored_label(color, setup::value(s, item));
            ui.label(
                egui::RichText::new(item.keys())
                    .monospace()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.end_row();
        }
    });
}
