//! A single lyric row halfway between the treble and bass staves.
use super::{notation, score::Staff};
use eframe::egui::{self, Color32};
use std::collections::BTreeMap;

pub(super) fn draw(
    staff: &Staff,
    painter: &egui::Painter,
    lyrics: &BTreeMap<u64, String>,
    x: impl Fn(u64) -> f32,
    color: Color32,
) {
    let y = (staff.y(notation::TREBLE_BOTTOM) + staff.y(notation::BASS_BOTTOM + 8)) / 2.0;
    for (&tick, text) in lyrics {
        // The note head's center is 0.62 gaps to the right of its time coordinate.
        let center = egui::pos2(x(tick) + 0.62 * staff.gap, y);
        let galley = painter.layout_no_wrap(
            text.clone(),
            egui::FontId::proportional(1.3 * staff.gap),
            color,
        );
        let rect = egui::Rect::from_center_size(center, galley.size());
        if painter.clip_rect().intersects(rect) {
            painter.galley(rect.min, galley, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lyrics_center_on_note_heads_and_between_staves_at_each_zoom() {
        let ctx = egui::Context::default();
        for gap in [4.5, 9.0, 18.0] {
            let staff = Staff {
                top: 15.0,
                high: 50,
                low: 20,
                gap,
            };
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw(
                        &staff,
                        ui.painter(),
                        &BTreeMap::from([(480, "きゃ".into())]),
                        |_| 100.0,
                        Color32::WHITE,
                    );
                });
            });
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == "きゃ" => Some(text),
                    _ => None,
                })
                .expect("lyric drawn");
            let center = text.pos + text.galley.size() / 2.0;
            assert!((center.x - (100.0 + 0.62 * gap)).abs() < 0.01);
            assert!((center.y - staff.y(notation::MIDDLE_C)).abs() < 0.01);
        }
    }
}
