//! Shared visual contract for both drum grids; layouts only supply cell rectangles.
use super::*;
use egui::{Color32, Rect, Stroke};

pub(super) fn lane_text(label: &str, muted: bool) -> egui::RichText {
    let text = egui::RichText::new(label);
    if muted {
        text.color(Color32::from_gray(100))
    } else {
        text
    }
}

pub(super) fn step_fill(ui: &egui::Ui, active: bool, muted: bool) -> Color32 {
    match (active, muted) {
        (true, true) => Color32::from_rgb(101, 105, 94),
        (true, false) => Color32::from_rgb(166, 226, 46),
        (false, true) => ui
            .visuals()
            .window_fill()
            .lerp_to_gamma(Color32::WHITE, 0.04),
        (false, false) => ui.visuals().widgets.inactive.bg_fill,
    }
}

pub(super) fn swing_toggle(
    ui: &mut egui::Ui,
    mut enabled: bool,
    selected: bool,
    cursor: Stroke,
    available: bool,
) -> egui::Response {
    let response = ui.add_enabled(available, egui::Checkbox::new(&mut enabled, "Swing"));
    if selected {
        ui.painter()
            .rect_stroke(response.rect, 2.0, cursor, egui::StrokeKind::Inside);
    }
    response
}

pub(super) fn swing_amount(
    ui: &mut egui::Ui,
    enabled: bool,
    amount: u8,
    selected: bool,
    cursor: Stroke,
    size: egui::Vec2,
) -> Option<egui::Response> {
    if !enabled {
        ui.allocate_exact_size(size, egui::Sense::hover());
        return None;
    }
    let mut button = egui::Button::new(format!("{amount}%")).truncate();
    if selected {
        button = button.stroke(cursor);
    }
    Some(ui.add_sized(size, button))
}

/// The same beat boundaries and appearance apply to both drum grids.
pub(super) fn beat_line(painter: &egui::Painter, step: usize, x: f32, top: f32, bottom: f32) {
    if step.is_multiple_of(4) {
        painter.line_segment(
            [egui::pos2(x, top), egui::pos2(x, bottom)],
            Stroke::new(2.0_f32, Color32::from_gray(150)),
        );
    }
}

pub(super) fn cursor_stroke(ctx: &egui::Context, focused: bool) -> Stroke {
    let orange = super::super::ACTIVE_PANE;
    let color = if focused {
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
        let phase = (ctx.input(|i| i.time) / 0.7).fract();
        let blend = (0.5 - 0.5 * (phase * std::f64::consts::TAU).cos()) as f32;
        orange.lerp_to_gamma(Color32::WHITE, blend)
    } else {
        orange
    };
    Stroke::new(if focused { 3.0_f32 } else { 2.0_f32 }, color)
}

pub(super) fn column(
    ui: &mut egui::Ui,
    step: usize,
    header: Rect,
    pitch: f32,
    line_x: f32,
    extent: Rect,
) {
    if pitch >= 16.0 {
        ui.put(header, egui::Label::new((step + 1).to_string()).truncate());
    }
    beat_line(ui.painter(), step, line_x, extent.top(), extent.bottom());
}

pub(super) struct Surface {
    pub cursor: Stroke,
    focused: bool,
    beacon: bool,
    sounding: Option<usize>,
    beat: Rect,
    playing: Rect,
}

impl Surface {
    pub fn new(app: &App, ctx: &egui::Context, sounding: Option<usize>) -> Self {
        let focused = super::title::active(app, ctx);
        let beacon = crate::view::patterns::beacon::focused(ctx)
            && crate::view::patterns::mode::editing(app, app.track).is_some_and(|c| {
                crate::view::patterns::beacon::empty(app.loaded.as_ref().unwrap(), c.pattern)
            });
        Self {
            cursor: cursor_stroke(ctx, focused),
            focused,
            beacon,
            sounding,
            beat: Rect::NOTHING,
            playing: Rect::NOTHING,
        }
    }

    pub fn selection(&self, ui: &egui::Ui, rect: Rect, selected: bool) {
        if self.focused && selected {
            ui.painter().rect_stroke(
                rect.shrink(self.cursor.width),
                2.0,
                Stroke::new(1.0_f32, Color32::from_gray(24)),
                egui::StrokeKind::Inside,
            );
        }
    }

    pub fn cell(&mut self, rect: Rect, step: usize) {
        if step < 4 {
            self.beat = self.beat.union(rect);
        }
        if self.sounding == Some(step) {
            self.playing = self.playing.union(rect);
        }
    }

    pub fn finish(&self, ui: &egui::Ui) {
        if self.playing.is_positive() {
            ui.painter().rect_stroke(
                self.playing.expand(2.0),
                2.0,
                Stroke::new(2.0_f32, crate::view::timeline_repeat::PLAYHEAD),
                egui::StrokeKind::Outside,
            );
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("drum-playing"), self.playing));
        }
        if self.beacon && self.beat.is_positive() {
            let stroke = crate::view::patterns::beacon::stroke(ui.ctx());
            crate::view::patterns::beacon::frame(ui.painter(), self.beat, stroke);
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("drum-beacon"), self.beat));
        }
    }
}

#[cfg(test)]
mod tests;
