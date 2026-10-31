use super::*;
use cat_smf_editor::score::{
    drums,
    note_view::note_name,
    pattern::recording::{pitches, Layout, Mode, Pad, Range, ROW_KEYS},
};
use std::time::Duration;

const RECORDING: egui::Color32 = egui::Color32::from_rgb(0xf9, 0x26, 0x72);
const REC_IDLE: egui::Color32 = egui::Color32::from_rgb(0xa0, 0x18, 0x48);
const BUTTON: f32 = 26.0;
const TONIC: egui::Color32 = egui::Color32::from_rgb(0xe6, 0xdb, 0x74);
const PAD: f32 = 46.0;
/// Horizontal stagger of the `z`, `a`, `q` and `1` rows, in pads.
const STAGGER: [f32; 4] = [1.25, 0.75, 0.5, 0.0];

pub(super) fn pitch_class(pitch: u8) -> &'static str {
    [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ][usize::from(pitch % 12)]
}

/// Root position from the root, e.g. `Dm7`; `None` for an unnamed stack.
pub(super) fn chord_name(notes: &[u8]) -> Option<String> {
    let root = *notes.first()?;
    let above: Vec<u8> = notes[1..].iter().map(|&p| p - root).collect();
    let quality = match above.as_slice() {
        [4, 7] => "",
        [3, 7] => "m",
        [3, 6] => "dim",
        [4, 8] => "aug",
        [4, 7, 11] => "M7",
        [4, 7, 10] => "7",
        [3, 7, 10] => "m7",
        [3, 6, 10] => "m7b5",
        [3, 7, 11] => "mM7",
        [3, 6, 9] => "dim7",
        _ => return None,
    };
    Some(format!("{}{quality}", pitch_class(root)))
}

/// Pad caption and whether its note or chord root is the tonic.
fn caption(t: &Settings, pad: Pad) -> (String, bool) {
    let notes = pitches(t, pad);
    if t.drums {
        let lane = notes
            .first()
            .and_then(|p| drums::NOTES.iter().position(|n| n == p));
        return (lane.map_or("", |i| drums::NAMES[i]).into(), false);
    }
    match (pad, t.mode.chord_notes()) {
        (Pad::Row { column, .. }, Some(tones)) => {
            let mut root = *t;
            root.mode = if tones == 3 {
                Mode::Triad
            } else {
                Mode::Seventh
            };
            let stack = pitches(&root, Pad::Row { row: 0, column });
            let tonic = stack.first().is_some_and(|p| p % 12 == t.key().tonic);
            (chord_name(&stack).unwrap_or_default(), tonic)
        }
        (Pad::Row { .. }, None) => notes.first().map_or((String::new(), false), |&p| {
            (note_name(p), p % 12 == t.key().tonic)
        }),
        (Pad::Letter(_) | Pad::Pitch(_), _) => (
            notes.first().map_or(String::new(), |&p| note_name(p)),
            false,
        ),
    }
}

/// The windows whose keys sound by mouse, each holding at most one pad.
#[derive(Clone, Copy)]
pub(super) enum Pointer {
    Pads,
    Piano,
}

/// The key under the pointer while a press that began on one of `keys` is held.
pub(super) fn pointed(ui: &egui::Ui, keys: &[(Pad, egui::Response)]) -> Option<Pad> {
    if !keys.iter().any(|(_, r)| r.is_pointer_button_down_on()) {
        return None;
    }
    let at = ui.input(|i| i.pointer.interact_pos())?;
    keys.iter()
        .find(|(_, r)| r.rect.contains(at))
        .map(|&(pad, _)| pad)
}

fn pad(ui: &mut egui::Ui, s: &Session, pad: Pad, label: &str) -> egui::Response {
    let (caption, tonic) = caption(s.settings(), pad);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(PAD, PAD), egui::Sense::drag());
    let fill = if s.recorder.as_ref().is_some_and(|r| r.held(pad)) {
        RECORDING
    } else if tonic {
        TONIC
    } else {
        egui::Color32::from_gray(210)
    };
    let p = ui.painter();
    p.rect_filled(rect.shrink(1.0), 3.0, fill);
    p.text(
        rect.left_top() + egui::vec2(5.0, 3.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(15.0),
        egui::Color32::BLACK,
    );
    p.text(
        rect.center_bottom() - egui::vec2(0.0, 9.0),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(11.0),
        egui::Color32::BLACK,
    );
    response
}

/// Position within the lap, and a short light on each beat (count-in included).
fn lap(ui: &mut egui::Ui, s: &Session) {
    let Some(r) = s.recorder.as_ref() else {
        return;
    };
    let Range::Loop { measures } = r.settings().range else {
        return;
    };
    let width = PAD * 14.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 10.0), egui::Sense::hover());
    let Some(origin) = r.origin() else {
        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_gray(60));
        return;
    };
    let beat = Duration::from_micros(u64::from(r.settings().tempo));
    let now = s.clock.elapsed();
    let from_count_in = (now + beat * 4).saturating_sub(origin);
    let on_beat = from_count_in.as_nanos() % beat.as_nanos() < 80_000_000;
    let lap = beat.as_nanos() * 4 * u128::from(measures);
    let position = now
        .checked_sub(origin)
        .map_or(0.0, |d| (d.as_nanos() % lap) as f32 / lap as f32);
    let p = ui.painter();
    p.rect_filled(
        rect,
        2.0,
        if on_beat {
            egui::Color32::from_gray(140)
        } else {
            egui::Color32::from_gray(60)
        },
    );
    p.rect_filled(
        egui::Rect::from_min_size(rect.min, egui::vec2(width * position, rect.height())),
        2.0,
        RECORDING,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Button {
    Rec,
    Stop,
    /// The container at this index of the session's sounds.
    Sound(usize),
}

/// Rec lights while counting in or recording; Stop is dim until then.
fn transport(ui: &mut egui::Ui, s: &mut Session) {
    let armed = play::armed(s);
    for button in [Button::Rec, Button::Stop] {
        let rec = button == Button::Rec;
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(BUTTON, BUTTON), egui::Sense::click());
        let enabled = rec != armed;
        let response = if enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        };
        let p = ui.painter();
        let face = rect.shrink(1.0);
        let center = rect.center();
        if rec {
            p.rect_filled(face, 3.0, egui::Color32::from_gray(210));
            if armed {
                p.circle_filled(center, BUTTON * 0.42, RECORDING.gamma_multiply(0.4));
            }
            let fill = if armed { RECORDING } else { REC_IDLE };
            p.circle_filled(center, BUTTON * 0.3, fill);
        } else {
            let (back, fill) = if armed {
                (egui::Color32::from_gray(210), egui::Color32::BLACK)
            } else {
                (egui::Color32::from_gray(75), egui::Color32::from_gray(105))
            };
            p.rect_filled(face, 3.0, back);
            let square = egui::Rect::from_center_size(center, egui::Vec2::splat(BUTTON * 0.46));
            p.rect_filled(square, 1.0, fill);
        }
        if enabled && response.clicked() {
            s.clicked = Some(button);
        }
    }
}

/// The companion windows, with the keys that also toggle them.
fn menu(ui: &mut egui::Ui, s: &mut Session) {
    ui.menu_button("☰", |ui| {
        // Wide enough for the keys; the popup shifts left inside the window.
        ui.set_min_width(150.0);
        for (label, key, open) in [
            ("設定", "Ctrl+,", &mut s.settings_open),
            ("Piano", "Ctrl+.", &mut s.piano_open),
        ] {
            // egui paints shortcuts in the weak color, unreadable on the hovered row.
            let key = egui::RichText::new(key).color(ui.visuals().text_color());
            let button = egui::Button::new(label).shortcut_text(key).selected(*open);
            if ui.add(button).clicked() {
                *open = !*open;
                ui.close_menu();
            }
        }
    });
}

/// The playing container; until a take is armed, a click lists the others.
fn container(ui: &mut egui::Ui, s: &mut Session) {
    let name = &s.sounds[s.selected].name;
    if play::armed(s) {
        ui.label(name);
        return;
    }
    let mut chosen = None;
    ui.menu_button(name, |ui| {
        for (i, sound) in s.sounds.iter().enumerate() {
            if ui.selectable_label(i == s.selected, &sound.name).clicked() {
                chosen = Some(i);
                ui.close_menu();
            }
        }
    });
    if let Some(i) = chosen.filter(|&i| i != s.selected) {
        s.clicked = Some(Button::Sound(i));
    }
}

fn playing(ui: &mut egui::Ui, s: &mut Session) {
    ui.horizontal(|ui| {
        transport(ui, s);
        container(ui, s);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            menu(ui, s)
        });
    });
    lap(ui, s);
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let mut keys = vec![];
    match s.settings().layout {
        Layout::Rows => {
            for row in (0..4u8).rev() {
                ui.horizontal(|ui| {
                    ui.add_space(PAD * STAGGER[usize::from(row)]);
                    for column in 0..ROW_KEYS[usize::from(row)] {
                        let label = pads::ROW_LABELS[usize::from(row)][usize::from(column)];
                        let at = Pad::Row { row, column };
                        keys.push((at, pad(ui, s, at, label)));
                    }
                });
            }
        }
        Layout::Letters => {
            ui.horizontal(|ui| {
                for (i, label) in pads::LETTER_LABELS.iter().enumerate() {
                    let at = Pad::Letter(i as u8);
                    keys.push((at, pad(ui, s, at, label)));
                }
            });
        }
    }
    let target = pointed(ui, &keys);
    play::point(s, Pointer::Pads, target);
}

pub(super) const TITLE: &str = "Virtual MIDI keyboard";

pub(crate) fn viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("virtual-midi-keyboard")
}

/// Size of a new window: four rows of pads with their stagger, plus the lap bar.
pub(super) const SIZE: egui::Vec2 = egui::vec2(PAD * 15.5, PAD * 4.0 + 100.0);
/// Title bar and borders around `SIZE`, so the outer window is what avoids others.
pub(super) const FRAME: egui::Vec2 = egui::vec2(16.0, 40.0);

/// Visible pattern windows, in screen coordinates.
pub(super) fn covers(a: &App, ctx: &egui::Context) -> Vec<egui::Rect> {
    [Pane::Scene, Pane::Song]
        .into_iter()
        .filter(|&p| {
            super::super::patterns::mode::workspace_open(a)
                && (p == Pane::Song || a.patterns.workspace.scene_window)
        })
        .filter_map(|p| ctx.input_for(p.viewport(), |i| i.viewport().outer_rect))
        .collect()
}

/// The place of a `size` window that overlaps `covers` least, nearest the center.
pub(super) fn place(screen: egui::Rect, covers: &[egui::Rect], size: egui::Vec2) -> egui::Pos2 {
    const GAP: f32 = 8.0;
    let inside = |p: egui::Pos2| p.clamp(screen.min, (screen.max - size).max(screen.min));
    let mut candidates = vec![screen.center() - size / 2.0];
    for c in covers {
        candidates.extend([
            egui::pos2(c.min.x - GAP - size.x, c.min.y),
            egui::pos2(c.max.x + GAP, c.min.y),
            egui::pos2(c.min.x, c.min.y - GAP - size.y),
            egui::pos2(c.min.x, c.max.y + GAP),
        ]);
    }
    let overlap = |p: egui::Pos2| -> f32 {
        let rect = egui::Rect::from_min_size(p, size);
        covers
            .iter()
            .map(|c| c.intersect(rect).area().max(0.0))
            .sum()
    };
    let far = |p: egui::Pos2| (p + size / 2.0).distance(screen.center());
    candidates
        .into_iter()
        .map(inside)
        .min_by(|&p, &q| {
            overlap(p)
                .total_cmp(&overlap(q))
                .then(far(p).total_cmp(&far(q)))
        })
        .unwrap()
}

pub(super) fn show(ctx: &egui::Context, class: egui::ViewportClass, s: &mut Session) {
    let color = if s.recorder.as_ref().is_some_and(Recorder::started) {
        RECORDING
    } else {
        egui::Color32::from_gray(90)
    };
    let body = |ui: &mut egui::Ui| {
        playing(ui, s);
        if let Some(error) = &s.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    };
    let frame = egui::Frame::central_panel(&ctx.style()).stroke(egui::Stroke::new(2.0_f32, color));
    if class == egui::ViewportClass::Embedded {
        egui::Window::new(TITLE).frame(frame).show(ctx, body);
    } else {
        egui::CentralPanel::default().frame(frame).show(ctx, body);
    }
}
