//! The Capture list: one row per phrase with its time, length, notes and container; a click
//! auditions the row and its button takes it as a pattern.
use super::*;

const ROLL: egui::Vec2 = egui::vec2(260.0, 30.0);
const NOTE: egui::Color32 = egui::Color32::from_rgb(0x66, 0xd9, 0xef);
const SOUNDING: egui::Color32 = egui::Color32::from_rgb(0xa6, 0xe2, 0x2e);
const TAKE: &str = "pattern化";

/// `YYYY-MM-DD HH:MM:SS` in local time, the text of a phrase's marker.
pub(super) fn stamp(ms: u64) -> String {
    let t = local(ms);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        t[0], t[1], t[3], t[4], t[5], t[6]
    )
}

/// Seconds from the local time `stamp` (`YYYY-MM-DD HH:MM:SS`) to the wall-clock `now_ms`.
pub(super) fn elapsed(stamp: &str, now_ms: u64) -> Option<u64> {
    let field = |range: std::ops::Range<usize>| stamp.get(range)?.parse::<i64>().ok();
    let then = [
        field(0..4)?,
        field(5..7)?,
        field(8..10)?,
        field(11..13)?,
        field(14..16)?,
        field(17..19)?,
    ];
    let t = local(now_ms);
    let now = [t[0], t[1], t[3], t[4], t[5], t[6]].map(i64::from);
    u64::try_from(seconds(now) - seconds(then)).ok()
}

/// Seconds since 1970-01-01 00:00:00 of a civil `[year, month, day, hour, minute, second]`.
fn seconds([y, m, d, hh, mm, ss]: [i64; 6]) -> i64 {
    // Days from civil, with the year starting in March.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400 + hh * 3_600 + mm * 60 + ss
}

/// SYSTEMTIME words: year, month, weekday, day, hour, minute, second, milliseconds.
#[cfg(windows)]
fn local(ms: u64) -> [u16; 8] {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FileTimeToLocalFileTime(utc: *const u64, local: *mut u64) -> i32;
        fn FileTimeToSystemTime(time: *const u64, out: *mut u16) -> i32;
    }
    // FILETIME counts 100 ns from 1601.
    let utc = ms * 10_000 + 116_444_736_000_000_000;
    let mut local = 0u64;
    let mut out = [0u16; 8];
    // SAFETY: both FILETIMEs are aligned 8-byte values and `out` holds a SYSTEMTIME.
    unsafe {
        FileTimeToLocalFileTime(&utc, &mut local);
        FileTimeToSystemTime(&local, out.as_mut_ptr());
    }
    out
}

#[cfg(not(windows))]
fn local(ms: u64) -> [u16; 8] {
    let s = ms / 1000;
    let mut out = [0u16; 8];
    out[4] = (s / 3600 % 24) as u16;
    out[5] = (s / 60 % 60) as u16;
    out[6] = (s % 60) as u16;
    out
}

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
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
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
                match elapsed(&p.stamp, now) {
                    Some(seconds) => {
                        let (text, color) = crate::view::recent::age(seconds);
                        ui.label(
                            egui::RichText::new(format!("{text:>4}"))
                                .monospace()
                                .color(color),
                        );
                    }
                    None => {
                        ui.monospace(&p.stamp);
                    }
                }
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

#[cfg(test)]
#[test]
fn a_stamp_reads_back_as_the_seconds_since_it() {
    let now = 1_790_000_000_000;
    for seconds in [0, 59, 61, 3_600, 86_400 * 40, 86_400 * 400] {
        assert_eq!(elapsed(&stamp(now - seconds * 1000), now), Some(seconds));
    }
    assert_eq!(elapsed("-", now), None);
}
