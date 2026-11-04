//! Settings and piano windows beside the keyboard; opening one leaves the keyboard focused.
use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Places {
    pub(super) settings: Option<egui::Pos2>,
    piano: Option<egui::Pos2>,
    /// What the shown rows of the settings window take, measured last frame.
    settings_size: Option<egui::Vec2>,
}
impl Places {
    pub(super) fn settings_size(&self) -> egui::Vec2 {
        self.settings_size.unwrap_or(SETTINGS_SIZE)
    }
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

/// Until the rows are first measured.
const SETTINGS_SIZE: egui::Vec2 = egui::vec2(420.0, 380.0);

/// Beside the keyboard and the other companion, overlapping the pattern windows least.
pub(super) fn place(
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
        let size = s.companions.settings_size();
        let builder = builder(ctx, SETTINGS_TITLE, at, size);
        let _window_span = crate::frame_trace::span("settings");
        ctx.show_viewport_immediate(settings_viewport(), builder, |ctx, class| {
            let _span = crate::frame_trace::span("settings/ui");
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
                let input = super::modal::begin(a, ctx);
                if s.first_read(ctx) {
                    settings_keys(a, s, &ctx.input(|i| i.events.clone()));
                }
                frame(ctx, class, SETTINGS_TITLE, |ui| {
                    s.companions.settings_size = Some(settings(ui, s));
                });
                super::modal::end(a, ctx, input);
                return;
            }
            frame(ctx, class, SETTINGS_TITLE, |ui| {
                settings(ui, s);
            });
        });
    }
    if s.piano_open {
        let other = s
            .companions
            .settings
            .map(|at| (at, s.companions.settings_size()));
        let at = s
            .companions
            .piano
            .or(saved(a, PIANO))
            .unwrap_or_else(|| place(a, ctx, keyboard, other, piano::SIZE));
        s.companions.piano = Some(at);
        let builder = builder(ctx, PIANO_TITLE, at, piano::SIZE);
        let _window_span = crate::frame_trace::span("piano");
        ctx.show_viewport_immediate(piano_viewport(), builder, |ctx, class| {
            let _span = crate::frame_trace::span("piano/ui");
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
                let input = super::modal::begin(a, ctx);
                frame(ctx, class, PIANO_TITLE, |ui| piano::show(ui, s));
                super::modal::end(a, ctx, input);
                return;
            }
            frame(ctx, class, PIANO_TITLE, |ui| piano::show(ui, s));
        });
    }
}

fn builder(
    ctx: &egui::Context,
    title: &str,
    at: egui::Pos2,
    size: egui::Vec2,
) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(title)
        .with_active(false)
        .with_position(at)
        .with_inner_size(size)
        .with_visible(window::shown(ctx))
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

/// The tint behind each group of rows.
fn group_color(group: setup::Group) -> egui::Color32 {
    match group {
        setup::Group::Take => egui::Color32::from_rgb(0xf9, 0x26, 0x72),
        setup::Group::Pitch => egui::Color32::from_rgb(0xa6, 0xe2, 0x2e),
    }
}

/// The central panel's margin around the rows.
const MARGIN: f32 = 8.0;

/// The shown rows, each group on its own tint; returns the window size they take.
pub(super) fn settings(ui: &mut egui::Ui, s: &mut Session) -> egui::Vec2 {
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    let visuals = ui.visuals().clone();
    let rows: Vec<_> = setup::ITEMS
        .into_iter()
        .filter(|&i| setup::shown(s, i))
        .collect();
    // Painted under the rows once the group extents are known.
    let backs: Vec<_> = rows
        .iter()
        .map(|_| ui.painter().add(egui::Shape::Noop))
        .collect();
    let mut spans: Vec<(setup::Group, egui::Rect)> = vec![];
    let grid = egui::Grid::new("recording-setup").show(ui, |ui| {
        for (n, &item) in rows.iter().enumerate() {
            if n > 0 && rows[n - 1].group() != item.group() {
                ui.allocate_space(egui::vec2(0.0, 6.0));
                ui.end_row();
            }
            let enabled = setup::enabled(s, item);
            let color = if enabled {
                visuals.text_color()
            } else {
                visuals.weak_text_color()
            };
            let label = egui::RichText::new(item.label()).color(color);
            let name = ui.selectable_label(item == s.item, label);
            if name.clicked() && enabled {
                s.item = item;
            }
            let value = match setup::choices(s, item) {
                Some((values, chosen)) => {
                    ui.horizontal(|ui| {
                        for (i, v) in values.into_iter().enumerate() {
                            let color = match (i == chosen, enabled) {
                                (true, true) => visuals.strong_text_color(),
                                (true, false) | (false, true) => visuals.weak_text_color(),
                                (false, false) => visuals.weak_text_color().gamma_multiply(0.5),
                            };
                            ui.colored_label(color, v);
                        }
                    })
                    .response
                }
                None => ui.colored_label(color, setup::value(s, item)),
            };
            let keys = ui.label(
                egui::RichText::new(item.keys())
                    .monospace()
                    .color(visuals.weak_text_color()),
            );
            let row = name.rect.union(value.rect).union(keys.rect);
            match spans.last_mut() {
                Some((group, rect)) if *group == item.group() => *rect = rect.union(row),
                _ => spans.push((item.group(), row)),
            }
            ui.end_row();
        }
    });
    let width = grid.response.rect.x_range();
    for ((group, rect), back) in spans.into_iter().zip(backs) {
        let rect = egui::Rect::from_x_y_ranges(width, rect.y_range()).expand2(egui::vec2(4.0, 2.0));
        let color = group_color(group);
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height()));
        ui.painter().set(
            back,
            egui::Shape::Vec(vec![
                egui::Shape::rect_filled(rect, 3.0, color.gamma_multiply(0.12)),
                egui::Shape::rect_filled(bar, 1.0, color),
            ]),
        );
    }
    // The panel itself fills the window, so only the rows tell what they need.
    (grid.response.rect.max.to_vec2() + egui::Vec2::splat(MARGIN)).ceil()
}

/// Gives each open keyboard window to the editor once its native window exists.
pub(super) fn adopt(ctx: &egui::Context, s: &mut Session) {
    let open = [
        (window::TITLE, true),
        (companions::SETTINGS_TITLE, s.settings_open),
        (companions::PIANO_TITLE, s.piano_open),
        (capture::TITLE, s.capture.is_some()),
    ];
    // A closed companion gets a new native window when it reopens.
    s.owned
        .retain(|t| open.iter().any(|(title, shown)| title == t && *shown));
    // Reopened at startup, the keyboard opens before `origin` is the foreground window.
    #[cfg(windows)]
    if s.owner.is_none() && ctx.input_for(s.origin, |i| i.focused) {
        s.owner = super::super::analysis::owner::editor();
    }
    #[cfg(windows)]
    if let Some(owner) = s.owner {
        for (title, shown) in open {
            if shown
                && !s.owned.contains(&title)
                && super::super::analysis::owner::adopt_by(owner, title)
            {
                s.owned.push(title);
            }
        }
    }
}
