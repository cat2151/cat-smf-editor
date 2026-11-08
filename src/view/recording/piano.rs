//! An 88-key piano of the notes sounding now and the notes each pad sounds now.
use super::*;
use cat_smf_editor::score::{
    note_view::note_name,
    pattern::recording::{pitches, Mode, Pad, ROW_KEYS},
};

/// A0 and C8.
const LOW: u8 = 21;
const HIGH: u8 = 108;
const WHITE: egui::Vec2 = egui::vec2(18.0, 100.0);
const BLACK: egui::Vec2 = egui::vec2(11.0, 62.0);
/// Width of the box beyond each end, lit by notes outside the 88 keys.
const EDGE: f32 = 22.0;
const WHITE_KEYS: f32 = 52.0;
/// The keys at their own size.
const KEYS: egui::Vec2 = egui::vec2(EDGE * 2.0 + WHITE.x * WHITE_KEYS, WHITE.y);
pub(super) const SIZE: egui::Vec2 = egui::vec2(KEYS.x + 16.0, KEYS.y + 16.0);

const SOUNDING: egui::Color32 = egui::Color32::from_rgb(0xf9, 0x26, 0x72);
const PLAYABLE_WHITE: egui::Color32 = egui::Color32::from_rgb(0xb4, 0xec, 0xf7);
const PLAYABLE_BLACK: egui::Color32 = egui::Color32::from_rgb(0x2f, 0x8c, 0xa0);

fn black(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

fn pads(mode: Mode) -> Vec<(Pad, &'static str)> {
    match mode {
        Mode::Letters => super::pads::LETTER_LABELS
            .iter()
            .enumerate()
            .map(|(i, &label)| (Pad::Letter(i as u8), label))
            .collect(),
        _ => (0..4u8)
            .flat_map(|row| {
                (0..ROW_KEYS[usize::from(row)]).map(move |column| {
                    (
                        Pad::Row { row, column },
                        super::pads::ROW_LABELS[usize::from(row)][usize::from(column)],
                    )
                })
            })
            .collect(),
    }
}

/// Pitches some pad sounds, and the key cap of the first pad whose lowest note each one is.
pub(super) fn playable(t: &Settings) -> ([bool; 128], [Option<&'static str>; 128]) {
    let mut playable = [false; 128];
    let mut labels = [None; 128];
    for (pad, label) in pads(t.mode) {
        let notes = pitches(t, pad);
        for &p in &notes {
            playable[usize::from(p)] = true;
        }
        if let Some(&low) = notes.first() {
            labels[usize::from(low)].get_or_insert(label);
        }
    }
    (playable, labels)
}

fn fill(sounding: bool, playable: bool, black: bool) -> egui::Color32 {
    match (sounding, playable, black) {
        (true, _, _) => SOUNDING,
        (false, true, false) => PLAYABLE_WHITE,
        (false, true, true) => PLAYABLE_BLACK,
        (false, false, false) => egui::Color32::from_gray(235),
        (false, false, true) => egui::Color32::from_gray(25),
    }
}

/// The box beyond one end, pointing outward.
fn edge(
    p: &egui::Painter,
    rect: egui::Rect,
    outside: std::ops::Range<usize>,
    sounding: &[bool],
    playable: &[bool],
    left: bool,
) {
    let any = |set: &[bool]| outside.clone().any(|i| set[i]);
    let color = if any(sounding) {
        SOUNDING
    } else if any(playable) {
        PLAYABLE_BLACK
    } else {
        egui::Color32::from_gray(60)
    };
    p.rect_filled(rect.shrink(1.0), 2.0, color);
    let c = rect.center();
    let k = rect.height() / WHITE.y;
    let (tip, back) = if left { (-5.0, 4.0) } else { (5.0, -4.0) };
    p.add(egui::Shape::convex_polygon(
        vec![
            c + egui::vec2(tip, 0.0) * k,
            c + egui::vec2(back, -6.0) * k,
            c + egui::vec2(back, 6.0) * k,
        ],
        egui::Color32::from_gray(220),
        egui::Stroke::NONE,
    ));
}

/// The keys at their own size, as the piano's window shows them.
pub(super) fn show(ui: &mut egui::Ui, s: &mut Session) {
    draw(ui, s, 1.0);
}

/// The keys as large as the pane holds them.
pub(super) fn pane(ui: &mut egui::Ui, s: &mut Session) {
    let room = ui.available_size();
    draw(ui, s, (room.x / KEYS.x).min(room.y / KEYS.y).max(0.0));
}

/// A pressed or dragged key sounds and records like a pad. `k` scales the keys.
fn draw(ui: &mut egui::Ui, s: &mut Session, k: f32) {
    let (white, black_key, edge_width) = (WHITE * k, BLACK * k, EDGE * k);
    let t = s.settings();
    let (mut playable, mut labels) = playable(t);
    if s.sound().patch.kit.is_some() {
        playable.fill(false);
        labels.fill(None);
        for (pad, label) in pads(t.mode) {
            for pitch in super::kit::pitches(s, pad) {
                playable[usize::from(pitch)] = true;
                labels[usize::from(pitch)].get_or_insert(label);
            }
        }
    }
    let mut sounding = [false; 128];
    if let Some(r) = &s.recorder {
        for (p, on) in sounding.iter_mut().enumerate() {
            *on = r.sounding(p as u8);
        }
    }
    let (rect, _) = ui.allocate_exact_size(KEYS * k, egui::Sense::hover());
    let p = ui.painter();
    let side = |x: f32| {
        egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(edge_width, white.y))
    };
    edge(
        p,
        side(rect.left()),
        0..usize::from(LOW),
        &sounding,
        &playable,
        true,
    );
    edge(
        p,
        side(rect.right() - edge_width),
        usize::from(HIGH) + 1..128,
        &sounding,
        &playable,
        false,
    );
    let caption = |p: &egui::Painter, at: egui::Pos2, text: &str, color: egui::Color32| {
        p.text(
            at,
            egui::Align2::CENTER_BOTTOM,
            text,
            egui::FontId::monospace(11.0 * k),
            color,
        );
    };
    let mut x = rect.left() + edge_width;
    let mut blacks = vec![];
    let mut whites = vec![];
    let interact = |pitch: u8, key: egui::Rect| {
        let id = egui::Id::new(("virtual-midi-keyboard-piano", pitch));
        (Pad::Pitch(pitch), ui.interact(key, id, egui::Sense::drag()))
    };
    for pitch in LOW..=HIGH {
        let i = usize::from(pitch);
        if black(pitch) {
            blacks.push((pitch, x - black_key.x / 2.0));
            continue;
        }
        let key = egui::Rect::from_min_size(egui::pos2(x, rect.top()), white);
        whites.push(interact(pitch, key));
        p.rect_filled(key.shrink(0.5), 1.0, fill(sounding[i], playable[i], false));
        if let Some(label) = labels[i] {
            caption(
                p,
                key.center_bottom() - egui::vec2(0.0, 16.0 * k),
                label,
                egui::Color32::BLACK,
            );
        }
        if pitch % 12 == 0 {
            caption(
                p,
                key.center_bottom() - egui::vec2(0.0, 2.0 * k),
                &note_name(pitch),
                egui::Color32::from_gray(110),
            );
        }
        x += white.x;
    }
    let mut keys = vec![];
    for (pitch, left) in blacks {
        let i = usize::from(pitch);
        let key = egui::Rect::from_min_size(egui::pos2(left, rect.top()), black_key);
        keys.push(interact(pitch, key));
        p.rect_filled(key, 1.0, fill(sounding[i], playable[i], true));
        if let Some(label) = labels[i] {
            caption(
                p,
                key.center_bottom() - egui::vec2(0.0, 3.0 * k),
                label,
                egui::Color32::WHITE,
            );
        }
    }
    // Black keys lie over the white ones, so they are hit first.
    keys.extend(whites);
    let held = s.mouse[window::Pointer::Piano as usize].is_some();
    let target = window::pointed(ui, &keys);
    play::point(s, window::Pointer::Piano, target);
    // Pads read their keys in the keyboard window, so releasing a key hands it the focus back.
    if held && target.is_none() && !ui.input(|i| i.pointer.primary_down()) {
        crate::view::presentation::send_to(ui.ctx(), s.home, egui::ViewportCommand::Focus);
    }
}
