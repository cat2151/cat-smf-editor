use super::*;

pub(super) fn show(a: &mut App, ctx: &egui::Context, s: &mut Session) {
    let color = if s.take.started() {
        egui::Color32::from_rgb(0xf9, 0x26, 0x72)
    } else {
        egui::Color32::from_gray(90)
    };
    egui::Modal::new(egui::Id::new("virtual-midi-keyboard"))
        .frame(egui::Frame::popup(&ctx.style()).stroke(egui::Stroke::new(2.0_f32, color)))
        .show(ctx, |ui| {
            ui.heading("Virtual MIDI keyboard");
            if s.ready {
                ui.label(&s.sounds[s.selected].name);
                ui.horizontal(|ui| {
                    for (key, pitch) in KEYS {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(48.0, 130.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            rect,
                            3.0,
                            if s.take.held(pitch) {
                                color
                            } else {
                                egui::Color32::from_gray(210)
                            },
                        );
                        ui.painter().text(
                            rect.center_bottom() - egui::vec2(0.0, 18.0),
                            egui::Align2::CENTER_CENTER,
                            key.name(),
                            egui::FontId::monospace(18.0),
                            egui::Color32::BLACK,
                        );
                    }
                });
            } else {
                for (index, sound) in s.sounds.iter().enumerate() {
                    if ui
                        .selectable_label(index == s.selected, &sound.name)
                        .clicked()
                    {
                        s.selected = index;
                    }
                }
                if ui.button("Enter: Select").clicked() {
                    prepare(a, s);
                }
            }
            if let Some(error) = &s.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
}
