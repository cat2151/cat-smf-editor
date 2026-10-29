//! Placement of the curve overlay, with separate floating and maximized geometry.
use eframe::egui;

pub(super) fn window(ctx: &egui::Context, maximized: bool, title: String) -> egui::Window<'static> {
    if crate::view::command_input::preview_active(ctx) {
        return crate::view::command_input::preview_window(ctx, title);
    }
    let window = egui::Window::new(title)
        .id(egui::Id::new("parameter-curve"))
        .collapsible(false);
    if maximized {
        window
            .id(egui::Id::new("parameter-curve-maximized"))
            .title_bar(false)
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
            .fixed_rect(ctx.screen_rect())
    } else {
        window.default_size([900.0, 520.0]).min_height(180.0)
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
