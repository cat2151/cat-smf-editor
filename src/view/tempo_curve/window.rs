//! Placement of the curve overlay, with separate floating and maximized geometry.
use eframe::egui;

pub(super) fn window(ctx: &egui::Context, maximized: bool) -> egui::Window<'static> {
    let window = egui::Window::new("Tempo curve").collapsible(false);
    if maximized {
        window
            .id(egui::Id::new("tempo-curve-maximized"))
            .title_bar(false)
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
            .fixed_rect(ctx.screen_rect())
    } else {
        window.default_size([900.0, 480.0]).min_height(180.0)
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
                        let shown = window(ctx, true)
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
