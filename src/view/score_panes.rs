//! The same score layout for the main view and Views previews.
use super::{staff, ACTIVE_PANE};
use crate::app::App;
use eframe::egui;

pub(super) fn show(
    app: &mut App,
    ui: &mut egui::Ui,
    browse_patch: &mut bool,
    replace_effect: &mut Option<usize>,
) -> Option<(usize, usize, u64)> {
    let mut map_click = None;
    // Raw rows are too wide for two columns.
    if let Some(tracks) = app.track_view.split.filter(|_| app.note_mode) {
        ui.columns(2, |panes| {
            let active = app.track_view.active;
            for slot in [active, 1 - active] {
                panes[slot].push_id(slot, |ui| {
                    if let Some(tick) = super::pane(
                        app,
                        ui,
                        tracks[slot],
                        slot == active,
                        browse_patch,
                        replace_effect,
                    ) {
                        map_click = Some((slot, tracks[slot], tick));
                    }
                });
                if slot == active {
                    let rect = panes[slot].min_rect().expand(2.0);
                    panes[slot].painter().with_clip_rect(rect).rect_stroke(
                        rect,
                        2.0,
                        egui::Stroke::new(2.0_f32, ACTIVE_PANE),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        });
    } else if app.note_mode {
        // The single column leaves the second column to the score.
        ui.columns(2, |panes| {
            if let Some(tick) = super::pane(
                app,
                &mut panes[0],
                app.track,
                true,
                browse_patch,
                replace_effect,
            ) {
                map_click = Some((app.track_view.active, app.track, tick));
            }
            // The tick overlays show the same time axis and own the wheel while open.
            if !app.piano_roll.open && !app.tempo_curve.open {
                staff::show(app, &mut panes[1]);
            }
        });
    } else if let Some(tick) = super::pane(app, ui, app.track, true, browse_patch, replace_effect) {
        map_click = Some((app.track_view.active, app.track, tick));
    }
    map_click
}
