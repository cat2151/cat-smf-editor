//! Timeline zoom numbers and page buttons shared by the piano roll and the tempo curve.
use eframe::egui;

/// Thickness of the zoom bars along the top and left edges.
pub(crate) const BAR: f32 = 24.0;
/// Length of a page button along its scrollbar.
pub(crate) const BUTTON: f32 = 16.0;
/// Zoom factor of a left click on a zoom bar; a right click divides by it.
pub(crate) const CLICK: f32 = 3.0 / 2.0;
/// Share of the visible length that a selected range fills after zooming to it.
pub(crate) const FIT: f32 = 0.9;
/// Share of the visible length that one page button click scrolls.
pub(crate) const PAGE: f32 = 0.5;
const WHEEL: f64 = 0.01;
const MAX: f32 = 4096.0;

/// Zoom factor for a wheel delta.
pub(crate) fn wheel(delta: f32) -> f32 {
    (delta * WHEEL as f32).exp()
}

/// [`wheel`] for axes zoomed in `f64`.
pub(crate) fn wheel_f64(delta: f64) -> f64 {
    (delta * WHEEL).exp()
}

/// Keeps a timeline zoom within its allowed range.
pub(crate) fn clamp(zoom: f32) -> f32 {
    zoom.clamp(f32::MIN_POSITIVE, MAX)
}

/// Scroll after zooming from `from` to `to` that centers, within `length`,
/// the content that was `anchor` px from the visible start.
pub(crate) fn rescroll(scroll: f32, anchor: f32, from: f32, to: f32, length: f32) -> f32 {
    ((scroll + anchor) * to / from - length / 2.0).max(0.0)
}

/// Draws a page button at each end of `bar`; returns -1 for the start one and +1 for
/// the end one when clicked.
pub(crate) fn page_buttons(
    ui: &mut egui::Ui,
    bar: egui::Rect,
    vertical: bool,
    salt: &str,
) -> Option<f32> {
    let mut clicked = None;
    let axis = usize::from(vertical);
    for end in [false, true] {
        let mut rect = bar;
        if end {
            rect.min[axis] = (rect.max[axis] - BUTTON).max(rect.min[axis]);
        } else {
            rect.max[axis] = (rect.min[axis] + BUTTON).min(rect.max[axis]);
        }
        let response = ui.interact(
            rect,
            ui.id().with((salt, vertical, end)),
            egui::Sense::click(),
        );
        let visuals = ui.style().interact(&response);
        ui.painter()
            .rect_filled(rect.shrink(1.0), 2.0, visuals.bg_fill);
        let direction = if end { 1.0 } else { -1.0 };
        let along = if vertical {
            egui::vec2(0.0, direction)
        } else {
            egui::vec2(direction, 0.0)
        };
        let across = egui::vec2(-along.y, along.x);
        let center = rect.center();
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                center + along * 4.0,
                center - along * 3.0 + across * 3.0,
                center - along * 3.0 - across * 3.0,
            ],
            visuals.fg_stroke.color,
            egui::Stroke::NONE,
        ));
        if response.clicked() {
            clicked = Some(direction);
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    #[test]
    fn wheel_factor_is_exp_of_a_hundredth_of_the_delta() {
        for delta in [-120.0f32, -40.0, -1.5, 0.0, 0.3, 40.0, 120.0] {
            assert_eq!(super::wheel(delta), (delta * 0.01).exp());
            assert_eq!(
                super::wheel_f64(f64::from(delta)),
                (f64::from(delta) * 0.01).exp()
            );
        }
    }
}
