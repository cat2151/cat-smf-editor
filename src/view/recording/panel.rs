//! The phrase loop switch, and while the phrase repeats, an LED patch display with browse.
use super::phrase::{self, Looper};
use super::window::{Button, BUTTON};
use super::*;

const LED_ON: egui::Color32 = egui::Color32::from_rgb(0xff, 0x40, 0x20);
const LED_BACK: egui::Color32 = egui::Color32::from_rgb(0x1a, 0x06, 0x04);
const WAITING: egui::Color32 = egui::Color32::from_rgb(0xe6, 0xdb, 0x74);
/// The repeating phrase, on the switch and on its pattern card.
pub(crate) const LOOPING: egui::Color32 = egui::Color32::from_rgb(0xa6, 0xe2, 0x2e);
const LED_SIZE: egui::Vec2 = egui::vec2(220.0, 26.0);
const BROWSE: egui::Vec2 = egui::vec2(64.0, 26.0);
const TO_SONG: &str = "鳴っているLoopをSongの末尾に並べてA-B repeat  Ctrl+Enter";

/// The arrow head on the ring at `angle` clockwise from the right, pointing clockwise.
fn arrow(center: egui::Pos2, radius: f32, angle: f32) -> Vec<egui::Pos2> {
    let out = egui::vec2(angle.cos(), angle.sin());
    let along = egui::vec2(-out.y, out.x);
    let tip = center + out * radius;
    vec![
        tip - out * 5.0 - along,
        tip + out * 5.0 - along,
        tip + along * 5.0,
    ]
}

/// A ring with an arrow head, lit while the mode is on; the head goes round once while the
/// measure is captured.
pub(super) fn switch(ui: &mut egui::Ui, s: &mut Session) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(BUTTON, BUTTON), egui::Sense::click());
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Phrase loop  Ctrl+E");
    let (back, fill) = match s.sound().looper {
        Looper::Off => (egui::Color32::from_gray(75), egui::Color32::from_gray(150)),
        Looper::Waiting | Looper::Capturing { .. } => (egui::Color32::from_gray(210), WAITING),
        Looper::Looping { .. } => (egui::Color32::from_gray(210), LOOPING),
    };
    let p = ui.painter();
    p.rect_filled(rect.shrink(1.0), 3.0, back);
    let center = rect.center();
    let radius = BUTTON * 0.28;
    if !matches!(s.sound().looper, Looper::Off) {
        p.circle_filled(center, radius + 2.5, egui::Color32::from_gray(40));
    }
    p.circle_stroke(center, radius, egui::Stroke::new(3.0_f32, fill));
    let angle = phrase::progress(s).unwrap_or(0.0) * std::f32::consts::TAU;
    p.add(egui::Shape::convex_polygon(
        arrow(center, radius, angle),
        fill,
        egui::Stroke::NONE,
    ));
    if response.clicked() {
        s.clicked = Some(Button::Phrase);
    }
}

/// Takes the repeating phrases to the Song; dim until a phrase repeats.
pub(super) fn to_song(ui: &mut egui::Ui, s: &mut Session) {
    let button = egui::Button::new("to song");
    let response = ui
        .add_enabled(phrase::any_looping(s), button)
        .on_hover_text(TO_SONG)
        .on_disabled_hover_text(TO_SONG);
    if response.clicked() {
        s.clicked = Some(Button::ToSong);
    }
}

/// Sound controls share a row while the phrase repeats, clear of the expression controls.
pub(super) fn sound(ui: &mut egui::Ui, s: &mut Session) {
    ui.horizontal(|ui| {
        // Capturing a replacement phrase hides the controls; held pads must not move.
        ui.set_min_height(LED_SIZE.y);
        if !phrase::looping(s) {
            return;
        }
        led(ui, s);
        if s.sound().container.is_some() {
            let response = ui
                .add_sized(BROWSE, egui::Button::new("Effects"))
                .on_hover_text("Ctrl+X");
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("keyboard-effects"), response.rect));
            if response.clicked() {
                s.clicked = Some(Button::Effects);
            }
        }
    });
}

fn led(ui: &mut egui::Ui, s: &mut Session) {
    let (text, layered) = phrase::display(s);
    let browse = !layered && s.sound().container.is_some();
    let rect = ui.allocate_exact_size(LED_SIZE, egui::Sense::hover()).0;
    let sense = if layered && s.sound().container.is_some() {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let response = ui.interact(rect, ui.id().with("phrase-led"), sense);
    let response = if sense == egui::Sense::click() {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    let p = ui.painter();
    p.rect_filled(rect, 3.0, LED_BACK);
    p.rect_stroke(
        rect,
        3.0,
        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(20)),
        egui::StrokeKind::Inside,
    );
    let font = egui::FontId::monospace(16.0);
    let inner = rect.shrink2(egui::vec2(8.0, 2.0));
    let clip = p.with_clip_rect(inner);
    // A dim wider copy under the text glows like a lit segment.
    for offset in [-0.8, 0.8] {
        clip.text(
            inner.left_center() + egui::vec2(offset, 0.0),
            egui::Align2::LEFT_CENTER,
            &text,
            font.clone(),
            LED_ON.gamma_multiply(0.35),
        );
    }
    clip.text(
        inner.left_center(),
        egui::Align2::LEFT_CENTER,
        &text,
        font,
        LED_ON,
    );
    if response.clicked() {
        s.clicked = Some(Button::Browse);
    }
    if browse {
        let button = egui::Button::new("browse");
        if ui
            .add_sized(BROWSE, button)
            .on_hover_text("Ctrl+T")
            .clicked()
        {
            s.clicked = Some(Button::Browse);
        }
    }
}
