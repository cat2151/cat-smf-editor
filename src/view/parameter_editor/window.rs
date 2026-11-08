//! Where the curve is drawn: its floating window, with separate floating and maximized
//! geometry, or a pane.
use crate::app::App;
use eframe::egui;

pub(super) fn window(ctx: &egui::Context, maximized: bool, title: String) -> egui::Window<'static> {
    if crate::view::command_input::preview_active(ctx) {
        return crate::view::command_input::preview_window(ctx, title);
    }
    let window = crate::view::presentation::window(
        ctx,
        crate::view::commands::Screen::Timeline,
        egui::Id::new(if maximized {
            "parameter-curve-maximized"
        } else {
            "parameter-curve"
        }),
        title,
    )
    .collapsible(false);
    if maximized {
        window
            .title_bar(false)
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
            .fixed_rect(ctx.screen_rect())
    } else {
        window.default_size([900.0, 520.0]).min_height(180.0)
    }
}

/// Draws the curve as a pane; the window holding the pane already routed its keys.
pub(crate) fn pane(app: &mut App, ui: &mut egui::Ui) {
    if app.parameter_editor.picker.is_some() || !app.parameter_editor.open || !docked(app, ui.ctx())
    {
        return;
    }
    let ctx = ui.ctx().clone();
    super::curve::show(app, &ctx, Some(ui));
}

/// The curve lives in a pane, except inside a Views preview, which always shows it floating.
pub(super) fn docked(app: &App, ctx: &egui::Context) -> bool {
    !crate::view::command_input::preview_active(ctx)
        && crate::view::dock::focus::placed(app, crate::view::dock::ViewKind::ParameterCurve)
}

/// A hidden dock window drops the curve's gestures.
pub(crate) fn suspend(app: &mut App) {
    let s = &mut app.parameter_editor;
    s.viewport.suspend();
    s.repeat_drag = Default::default();
    if std::mem::take(&mut s.drag) {
        s.draft = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximized_window_fills_the_viewport_after_each_resize() {
        let ctx = egui::Context::default();
        for size in [egui::vec2(1500.0, 900.0), egui::vec2(1100.0, 700.0)] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            for _ in 0..3 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::TopBottomPanel::top("transport").show(ctx, |ui| {
                            ui.label("Transport");
                        });
                        egui::SidePanel::left("tracks").show(ctx, |ui| {
                            ui.label("Tracks");
                        });
                        egui::CentralPanel::default().show(ctx, |_| {});
                        let shown = window(ctx, true, "Parameter curve".into())
                            .show(ctx, |ui| {
                                ui.allocate_space(ui.available_size());
                            })
                            .unwrap();
                        assert_eq!(shown.response.rect, screen);
                    },
                );
            }
        }
    }
}
