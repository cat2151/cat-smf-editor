use crate::app::App;
use eframe::egui;

pub(crate) fn viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("context-help")
}

/// Opens help above every other window, leaving the keyboard focus where it is.
pub(crate) fn summon(a: &mut App) {
    a.help_open = true;
    a.help.raise = true;
}

/// Puts the current native window above every normal window without activating it.
pub(crate) fn lift(ctx: &egui::Context) {
    for level in [egui::WindowLevel::AlwaysOnTop, egui::WindowLevel::Normal] {
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
    }
}

pub(crate) fn restore(a: &mut App) {
    a.help_open = a.config.help_open;
    a.help.geometry = a.config.help_window;
}

fn remember(a: &mut App) {
    let changed = a.config.help_open != a.help_open;
    a.config.help_open = a.help_open;
    a.config.help_window = a.help.geometry;
    // Geometry changes are kept in memory during dragging, and flushed on exit.
    if changed {
        a.config.persist();
    }
}

pub(crate) fn show(a: &mut App, ctx: &egui::Context) {
    if !a.help_open {
        remember(a);
        return;
    }
    if a.help.geometry.is_some_and(|w| {
        !super::placement::visible(w, ctx.native_pixels_per_point().unwrap_or(1.0))
    }) {
        a.help.geometry = None;
        ctx.send_viewport_cmd_to(
            viewport(),
            egui::ViewportCommand::OuterPosition(ctx.input(|i| {
                i.viewport()
                    .outer_rect
                    .map_or(egui::pos2(40.0, 40.0), |r| r.min + egui::vec2(40.0, 40.0))
            })),
        );
    }
    let mut builder = egui::ViewportBuilder::default()
        .with_title("ヘルプ")
        .with_active(false)
        .with_inner_size([620.0, 680.0])
        .with_min_inner_size([240.0, 160.0]);
    if let Some([x, y, w, h]) = a.help.geometry {
        builder = builder.with_position([x, y]).with_inner_size([w, h]);
    }
    ctx.show_viewport_immediate(viewport(), builder, |ctx, class| {
        let close = class != egui::ViewportClass::Embedded
            && ctx.input(|i| {
                i.viewport().close_requested()
                    || i.key_pressed(egui::Key::Escape)
                    || i.key_pressed(egui::Key::F1)
            });
        if close {
            a.help_open = false;
        }
        if class == egui::ViewportClass::Embedded {
            return; // This UI requires a native backend; never replace the editor with help.
        }
        if std::mem::take(&mut a.help.raise) {
            lift(ctx);
        }
        ctx.input(|i| {
            if let (Some(outer), Some(inner)) = (i.viewport().outer_rect, i.viewport().inner_rect) {
                a.help.geometry = Some([outer.min.x, outer.min.y, inner.width(), inner.height()]);
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| super::body(a, ui));
        });
        ctx.request_repaint_after_for(
            std::time::Duration::from_millis(100),
            egui::ViewportId::ROOT,
        );
    });
    remember(a);
}
