//! The playing position over the arrangement cells.
use super::*;
use crate::view::timeline_repeat::{TickAxis, PLAYHEAD};

/// Content x of the playing position, while this pane's own arrangement plays.
pub(super) fn x(a: &App, c: &scene::Collection, p: Pane, stride: f32, label: f32) -> Option<f32> {
    let id = scene_id(a, p);
    if p == Pane::Scene && id.is_none() {
        return None;
    }
    let active = a
        .playback
        .active
        .as_ref()
        .filter(|active| active.mode == crate::play::playback::Mode::Arrangement(id))?;
    let tick = crate::play::transport::tick_for(a, active.clock_track)?;
    match id {
        Some(id) => {
            let scene = c.scenes.iter().find(|s| s.id == id)?;
            let boundaries = super::super::timeline::scene_boundaries(scene);
            let axis = super::super::timeline::SceneCells {
                boundaries: &boundaries,
                stride,
                label,
            };
            Some(axis.x(tick))
        }
        None => {
            let time =
                cat_smf_editor::score::musical_time::MusicalTime::new(&a.loaded.as_ref()?.music);
            let map = time.map(0)?;
            let axis = super::super::timeline::Cells { map, stride, label };
            Some(axis.x(tick))
        }
    }
}

pub(super) fn paint(ui: &egui::Ui, x: f32, cells: egui::Rect) {
    ui.painter()
        .vline(x, cells.y_range(), egui::Stroke::new(2.0_f32, PLAYHEAD));
}
