//! The Capture list: one row per phrase with its sequence, length, notes and container; a click
//! auditions the row and its button takes it as a pattern.
use super::*;

const ROLL: egui::Vec2 = egui::vec2(260.0, 30.0);
const NOTE: egui::Color32 = egui::Color32::from_rgb(0x66, 0xd9, 0xef);
const SOUNDING: egui::Color32 = egui::Color32::from_rgb(0xa6, 0xe2, 0x2e);
const TAKE: &str = "pattern化";

/// The notes of `p` across `rect`, lowest pitch at the bottom.
fn roll(painter: &egui::Painter, rect: egui::Rect, p: &Captured, color: egui::Color32) {
    painter.rect_filled(rect, 2.0, egui::Color32::from_gray(30));
    let low = p.notes.iter().map(|n| n.pitch).min().unwrap_or(60);
    let high = p.notes.iter().map(|n| n.pitch).max().unwrap_or(60);
    let rows = f32::from(high - low + 1).max(12.0);
    let length = p.end().max(1) as f32;
    let row = rect.height() / rows;
    for n in &p.notes {
        let x = rect.left() + rect.width() * n.start as f32 / length;
        let width = (rect.width() * n.length as f32 / length).max(2.0);
        let y = rect.bottom() - row * f32::from(n.pitch - low + 1);
        let note = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, row.max(2.0)));
        painter.rect_filled(note.intersect(rect), 0.0, color);
    }
}

/// A row's listening part, or its pattern button.
pub(super) enum Click {
    Listen(usize),
    Take(usize),
}

/// `sounding` lights the selected row; lengths are timed at `bpm`.
pub(super) fn list(ui: &mut egui::Ui, s: &Session, sounding: bool, bpm: f64) -> Option<Click> {
    let b = s.capture.as_ref()?;
    let name = |c: u64| {
        s.sounds
            .iter()
            .find(|sound| sound.container == Some(c))
            .map_or("-", |sound| sound.name.as_str())
    };
    let mut clicked = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (i, p) in b.phrases.iter().enumerate() {
            let selected = i == b.selected;
            let egui::InnerResponse {
                inner: take,
                response,
            } = ui.horizontal(|ui| {
                ui.monospace(format!("#{}", p.sequence));
                let quarters = p.end() as f64 / f64::from(s.division.max(1));
                let seconds = (quarters * 60.0 / bpm) as u64;
                ui.monospace(format!("{:>2}:{:02}", seconds / 60, seconds % 60));
                let (rect, _) = ui.allocate_exact_size(ROLL, egui::Sense::hover());
                let color = if selected && sounding { SOUNDING } else { NOTE };
                roll(ui.painter(), rect, p, color);
                ui.label(name(p.container));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.button(TAKE).on_hover_text("Enter")
                })
                .inner
            });
            let row = response.rect.expand2(egui::vec2(4.0, 1.0));
            if selected {
                ui.painter().rect_stroke(
                    row,
                    2.0,
                    egui::Stroke::new(2.0_f32, ui.visuals().selection.stroke.color),
                    egui::StrokeKind::Inside,
                );
                response.scroll_to_me(None);
            }
            // Created after the button, so the row would take the button's clicks.
            let listen = egui::Rect::from_min_max(
                row.min,
                egui::pos2(take.rect.left() - ui.spacing().item_spacing.x, row.max.y),
            );
            let response = ui.interact(listen, ui.id().with(("capture", i)), egui::Sense::click());
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new(("capture-row", i)), (listen, take.rect))
            });
            if take.clicked() {
                clicked = Some(Click::Take(i));
            } else if response.clicked() {
                clicked = Some(Click::Listen(i));
            }
        }
    });
    clicked
}
