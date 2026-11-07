use super::*;
use crate::view::piano_roll::Layout;
use eframe::egui::Color32;
use eframe::emath::GuiRounding;

pub(super) const NAMES_WIDTH: f32 = 120.0;
pub(super) const EMPTY_FILL: Color32 = Color32::from_gray(40);
/// Space between two tracks' panes.
pub(super) const GAP: f32 = 4.0;
/// Width of the track color strip at the header's left.
pub(super) const STRIP: f32 = 4.0;
pub(super) const LANE_TINT: f32 = 0.12;
pub(super) const NAME_ACTIVE_TINT: f32 = 0.35;
pub(super) const MEASURE_TINT: f32 = 0.3;
pub(super) const FRAME_TINT: f32 = 0.7;
/// Match the attack mark width in Pattern Song.
pub(super) const ONSET_WIDTH: f32 = 2.0;
pub(super) const TAIL_TINT: f32 = 0.25;

pub(super) fn patterns(
    painter: &egui::Painter,
    rect: egui::Rect,
    scale: f32,
    row: &super::song::Row,
) {
    let color = crate::view::track_color(row.track);
    for placed in &row.curves {
        let clip = egui::Rect::from_x_y_ranges(
            rect.left() + placed.start as f32 * scale..=rect.left() + placed.end as f32 * scale,
            rect.y_range(),
        )
        .intersect(painter.clip_rect());
        if clip.is_positive() {
            super::curves::draw(
                &painter.with_clip_rect(clip),
                rect,
                scale,
                std::slice::from_ref(&placed.curve),
            );
        }
    }
    for o in &row.occurrences {
        let block = egui::Rect::from_x_y_ranges(
            rect.left() + o.start as f32 * scale..=rect.left() + o.end as f32 * scale,
            lane(rect).y_range(),
        );
        if !block.intersects(painter.clip_rect()) {
            continue;
        }
        let tint = if o.disabled { 0.25 } else { 0.8 };
        painter.rect_stroke(
            block,
            1.0,
            egui::Stroke::new(1.0_f32, color.gamma_multiply(tint)),
            egui::StrokeKind::Inside,
        );
        let label_clip = block.intersect(painter.clip_rect());
        painter.with_clip_rect(label_clip).text(
            egui::pos2(block.left().max(label_clip.left()) + 3.0, block.top() + 1.0),
            egui::Align2::LEFT_TOP,
            &o.name,
            egui::FontId::proportional(11.0),
            color.gamma_multiply(tint),
        );
    }
}

/// The smallest scroll change that brings `cell` inside the `shown` size.
pub(super) fn reveal_offset(
    mut offset: egui::Vec2,
    shown: egui::Vec2,
    cell: egui::Rect,
) -> egui::Vec2 {
    for axis in 0..2 {
        if cell.max[axis] > offset[axis] + shown[axis] {
            offset[axis] = cell.max[axis] - shown[axis];
        }
        if cell.min[axis] < offset[axis] {
            offset[axis] = cell.min[axis];
        }
        offset[axis] = offset[axis].max(0.0);
    }
    offset
}

/// The track's own color on fill, frame, measure lines and notes; an empty track stays gray.
pub(super) fn draw_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    layout: &Layout,
    kind: Kind,
    drums: bool,
    color: Color32,
) {
    let lane = lane(rect);
    // The lane spans the whole song; shapes that long lose precision and flicker while scrolling.
    let shown = lane
        .intersect(painter.clip_rect())
        .round_to_pixels(painter.pixels_per_point());
    if !shown.is_positive() {
        return;
    }
    if kind == Kind::Empty {
        painter.rect_filled(shown, 0.0, EMPTY_FILL);
        return;
    }
    painter.rect_filled(shown, 0.0, color.gamma_multiply(LANE_TINT));
    for &(x, beat) in &layout.beats {
        let x = rect.left() + x;
        if beat == 1 && shown.x_range().contains(x) {
            painter.vline(
                x,
                shown.y_range(),
                egui::Stroke::new(1.0_f32, color.gamma_multiply(MEASURE_TINT)),
            );
        }
    }
    let origin = rect.min.to_vec2();
    for &(_, note) in &layout.notes {
        let note = note.translate(origin);
        let split = if drums {
            note.right().min(note.left() + ONSET_WIDTH)
        } else {
            note.right()
        };
        for (part, fill) in [
            (note.with_max_x(split), color),
            (note.with_min_x(split), color.gamma_multiply(TAIL_TINT)),
        ] {
            let part = part.intersect(shown);
            if part.is_positive() {
                painter.rect_filled(part, 0.0, fill);
            }
        }
    }
    let frame = egui::Stroke::new(1.0_f32, color.gamma_multiply(FRAME_TINT));
    painter.hline(shown.x_range(), shown.top() + 0.5, frame);
    painter.hline(shown.x_range(), shown.bottom() - 0.5, frame);
    for x in [lane.left() + 0.5, lane.right() - 0.5] {
        if shown.x_range().contains(x) {
            painter.vline(x, shown.y_range(), frame);
        }
    }
}

/// The pane less the gap that separates it from its neighbors.
pub(super) fn lane(rect: egui::Rect) -> egui::Rect {
    rect.shrink2(egui::vec2(0.0, GAP / 2.0))
}

/// DAW-like track header: a color strip and the name; the cell's track is filled brighter.
pub(super) fn draw_name(
    painter: &egui::Painter,
    rect: egui::Rect,
    name: &str,
    color: Color32,
    active: bool,
) {
    let lane = lane(rect);
    let fill = if active { NAME_ACTIVE_TINT } else { LANE_TINT };
    painter.rect_filled(lane, 0.0, color.gamma_multiply(fill));
    painter.rect_filled(lane.with_max_x(lane.left() + STRIP), 0.0, color);
    painter.text(
        egui::pos2(lane.left() + STRIP + 4.0, lane.top() + 2.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(12.0),
        if active { Color32::WHITE } else { color },
    );
}

pub(super) fn placement(ctx: &egui::Context, maximized: bool) -> egui::Window<'static> {
    if crate::view::command_input::preview_active(ctx) {
        return crate::view::command_input::preview_window(ctx, "Track Overview");
    }
    let window = egui::Window::new("Track Overview").collapsible(false);
    if maximized {
        window
            .id(egui::Id::new("track-overview-maximized"))
            .title_bar(false)
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
            .fixed_rect(ctx.screen_rect())
    } else {
        window.default_size([900.0, 480.0]).min_height(180.0)
    }
}
