use cat_smf_editor::score::key_signature::KeySignature;
use eframe::egui::{self, Color32};

/// Lit keys are dimmed so the cell text above them stays readable; the playing row keeps full color.
pub const LIT: f32 = 0.35;

/// Selection outline; Monokai background so it shows on every key color.
const SELECTED: Color32 = Color32::from_rgb(0x27, 0x28, 0x22);

/// Dark keyboard painted behind the row's cells; it takes no space or input.
/// All note rows use the same measured track range.
pub fn backdrop(
    ui: &egui::Ui,
    rect: egui::Rect,
    pitches: &[(u8, bool)],
    range: (u8, u8),
    key: Option<KeySignature>,
    playing: bool,
) {
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    paint(&painter, rect, pitches, range, key, playing);
}

fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    pitches: &[(u8, bool)],
    range: (u8, u8),
    key: Option<KeySignature>,
    playing: bool,
) {
    let mut unique = std::collections::BTreeMap::new();
    for &(pitch, selected) in pitches {
        *unique.entry(pitch).or_insert(false) |= selected;
    }
    for is_black in [false, true] {
        for note in range.0..=range.1 {
            if black(note) != is_black {
                continue;
            }
            let mut key_box = key_rect(rect, range, note);
            if !is_black {
                key_box.max.x -= 0.5;
            }
            let fill = if unique.contains_key(&note) {
                let color = note_color(note, key);
                if playing {
                    color
                } else {
                    color.gamma_multiply(LIT)
                }
            } else if is_black {
                Color32::BLACK
            } else {
                Color32::from_white_alpha(5)
            };
            painter.rect_filled(key_box, 0.0, fill);
        }
    }
    for (&pitch, &selected) in &unique {
        if selected {
            painter.rect_stroke(
                key_rect(rect, range, pitch).shrink(1.0),
                0.0,
                egui::Stroke::new(2.0_f32, SELECTED),
                egui::StrokeKind::Inside,
            );
        }
    }
}

pub(crate) fn key_rect(rect: egui::Rect, range: (u8, u8), note: u8) -> egui::Rect {
    let bounds = |note| {
        let whites = (0..note).filter(|&n| !black(n)).count() as f32;
        if black(note) {
            (whites - 0.325, whites + 0.325)
        } else {
            (whites, whites + 1.0)
        }
    };
    let left = bounds(range.0).0;
    let right = bounds(range.1).1;
    let scale = rect.width() / (right - left);
    let (start, end) = bounds(note);
    egui::Rect::from_min_max(
        egui::pos2(rect.left() + (start - left) * scale, rect.top()),
        egui::pos2(
            rect.left() + (end - left) * scale,
            rect.top() + rect.height() * if black(note) { 0.6 } else { 1.0 },
        ),
    )
}

fn black(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

/// One hue turn over the fifth circle, tonic cyan and tritone (its complement) pink.
/// The six Monokai accents keep their exact values; the rest are OKLCH-interpolated between them.
const FIFTH_COLORS: [Color32; 12] = [
    Color32::from_rgb(0x66, 0xd9, 0xef),
    Color32::from_rgb(0x6c, 0xc3, 0xfe),
    Color32::from_rgb(0x8b, 0xa7, 0xfd),
    Color32::from_rgb(0xae, 0x81, 0xff),
    Color32::from_rgb(0xcf, 0x66, 0xe2),
    Color32::from_rgb(0xe9, 0x49, 0xb2),
    Color32::from_rgb(0xf9, 0x26, 0x72),
    Color32::from_rgb(0xfd, 0x97, 0x1f),
    Color32::from_rgb(0xe6, 0xdb, 0x74),
    Color32::from_rgb(0xa6, 0xe2, 0x2e),
    Color32::from_rgb(0x54, 0xea, 0x98),
    Color32::from_rgb(0x2e, 0xe5, 0xd2),
];

/// Minor turns the sharp side toward purple, major toward green.
pub fn note_color(pitch: u8, key: Option<KeySignature>) -> Color32 {
    key.map_or(Color32::from_gray(150), |key| {
        let up = key.fifths_above_tonic(pitch);
        FIFTH_COLORS[usize::from(if key.minor() { up } else { (12 - up) % 12 })]
    })
}

pub(crate) fn roll_color(pitch: u8, key: Option<KeySignature>) -> Color32 {
    key.map_or(
        FIFTH_COLORS[usize::from((12 - (7 * (pitch % 12)) % 12) % 12)],
        |key| note_color(pitch, Some(key)),
    )
}

#[cfg(test)]
mod tests;
