//! A pulsing frame on the next step while the only pattern is unfinished.
use crate::app::App;
use cat_smf_editor::score::{
    note_view::note_on,
    pattern::{body, Catalog},
    scene,
};
use eframe::egui::{self, Stroke};

/// The next step shown on the only pattern candidate of Song.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Edit(u64),
    Place(u64),
}

pub fn step(a: &App) -> Option<Step> {
    let m = &a.loaded.as_ref()?.music;
    let catalog = Catalog::read(m).ok()?;
    let arrangement = scene::read(m).ok()?;
    let [p] = catalog.patterns.values().collect::<Vec<_>>()[..] else {
        return None;
    };
    if !arrangement.scenes.is_empty() {
        return None;
    }
    if empty(m, p.id) {
        Some(Step::Edit(p.id))
    } else {
        arrangement.song.is_empty().then_some(Step::Place(p.id))
    }
}

pub fn empty(m: &cat_smf_editor::model::Music, id: u64) -> bool {
    Catalog::read(m)
        .ok()
        .and_then(|c| {
            let p = c.patterns.get(&id)?;
            let events = body::extract(&m.tracks[p.track].events, p.range().ok()?).ok()?;
            Some(!events.iter().any(note_on))
        })
        .unwrap_or(false)
}

pub fn focused(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().focused.unwrap_or(false))
}

pub fn stroke(ctx: &egui::Context) -> Stroke {
    ctx.request_repaint_after(std::time::Duration::from_millis(33));
    let t = ctx.input(|i| i.time) as f32;
    let alpha = 0.55 + 0.45 * (t * std::f32::consts::TAU).sin();
    Stroke::new(3.0_f32, super::presentation::ACCENT.gamma_multiply(alpha))
}

pub fn frame(painter: &egui::Painter, rect: egui::Rect, stroke: Stroke) {
    painter.rect_stroke(rect.expand(2.0), 4.0, stroke, egui::StrokeKind::Outside);
}
