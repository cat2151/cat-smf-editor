//! Row-local numeric controls use the same commit/undo path as keyboard input.
use super::{commands::Op, *};

#[derive(Clone, Copy, Debug)]
pub(super) enum Field {
    Note,
    Velocity,
    Swing,
}
impl Field {
    fn stepped(self, value: u8, up: bool) -> u8 {
        let next = if matches!(self, Self::Velocity) {
            if up {
                (value / 10 + 1).saturating_mul(10)
            } else {
                value.saturating_sub(1) / 10 * 10
            }
        } else {
            value.saturating_add_signed(if up { 1 } else { -1 })
        };
        next.clamp(*self.range().start(), *self.range().end())
    }
    fn range(self) -> std::ops::RangeInclusive<u8> {
        match self {
            Self::Note => 0..=127,
            Self::Velocity => 1..=127,
            Self::Swing => 1..=99,
        }
    }
    pub(super) fn error(self) -> &'static str {
        match self {
            Self::Note => message::MIDI_NOTE_MUST_BE_0_127,
            Self::Velocity => "Velocityは1～127で指定します",
            Self::Swing => "Swingは1～99%で指定します",
        }
    }
}
pub(super) fn level(patch: &Patch, row: usize) -> Result<f32, String> {
    let id = patch.drums.as_ref().ok_or(message::NO_DRUM_SETTINGS)?.rows[row].layer;
    patch
        .layers
        .iter()
        .find(|l| l.settings.id == id)
        .map(|l| l.settings.volume_db)
        .ok_or(message::MISSING_INSTRUMENT_LAYER.into())
}
pub(super) fn set_level(app: &mut App, row: usize, db: f32) -> Result<(), String> {
    let mut patch = app.loaded.as_ref().ok_or(message::NO_SONG)?.patches[app.track].clone();
    let id = patch.drums.as_ref().ok_or(message::NO_DRUM_SETTINGS)?.rows[row].layer;
    patch
        .layers
        .iter_mut()
        .find(|l| l.settings.id == id)
        .ok_or(message::MISSING_INSTRUMENT_LAYER)?
        .settings
        .volume_db = db;
    patch.validate()?;
    crate::player::selection::store_track(app, app.track, &patch)
}
pub(super) fn set(app: &mut App, row: usize, field: Field, value: u8) -> Result<(), String> {
    if !field.range().contains(&value) {
        return Err(field.error().into());
    }
    let mut patch = app.loaded.as_ref().ok_or(message::NO_SONG)?.patches[app.track].clone();
    let settings = patch.drums.as_mut().ok_or(message::NO_DRUM_SETTINGS)?;
    match field {
        Field::Note => settings.rows[row].output = value,
        Field::Velocity => settings.rows[row].velocity = value,
        Field::Swing => settings.rows[row].swing = value,
    }
    crate::player::selection::store_track(app, app.track, &patch)
}
pub(super) fn toggle_swing(app: &mut App, row: usize) -> Result<(), String> {
    let mut patch = app.loaded.as_ref().ok_or(message::NO_SONG)?.patches[app.track].clone();
    let r = &mut patch.drums.as_mut().ok_or(message::NO_DRUM_SETTINGS)?.rows[row];
    r.swing_enabled = !r.swing_enabled;
    crate::player::selection::store_track(app, app.track, &patch)
}
pub(super) fn toggle_mute(app: &mut App, row: usize) -> Result<(), String> {
    super::ensure(app)?;
    if app.drum_editor.confirm_create {
        return Ok(());
    }
    let mut patch = app.loaded.as_ref().ok_or(message::NO_SONG)?.patches[app.track].clone();
    let id = patch.drums.as_ref().ok_or(message::NO_DRUM_SETTINGS)?.rows[row].layer;
    let layer = patch
        .layers
        .iter_mut()
        .find(|l| l.settings.id == id)
        .ok_or(message::MISSING_INSTRUMENT_LAYER)?;
    layer.settings.mute = !layer.settings.mute;
    crate::player::selection::store_track(app, app.track, &patch)
}
pub(super) fn wheel(
    ui: &mut egui::Ui,
    response: &egui::Response,
    row: usize,
    field: Field,
    value: u8,
    action: &mut Option<Op>,
) {
    if response.hovered() {
        let delta = ui.input(|i| i.raw_scroll_delta.y);
        if delta != 0.0 {
            ui.input_mut(|i| {
                i.raw_scroll_delta.y = 0.0;
                i.smooth_scroll_delta.y = 0.0;
            });
            let value = field.stepped(value, delta > 0.0);
            *action = Some(Op::Value(row, field, value));
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn controls(
    ui: &mut egui::Ui,
    bounds: egui::Rect,
    patch: &Patch,
    state: &mut State,
    row: usize,
    first: usize,
    cursor: egui::Stroke,
    action: &mut Option<Op>,
) {
    let Some(settings) = &patch.drums else { return };
    let r = &settings.rows[row];
    if !r.swing_enabled && state.row == row && state.field == first + 3 {
        state.field = first + 2;
    }
    ui.scope_builder(egui::UiBuilder::new().max_rect(bounds), |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.horizontal(|ui| {
            let button = |label: String, selected: bool| {
                let b = egui::Button::new(label).truncate();
                if selected {
                    b.stroke(cursor)
                } else {
                    b
                }
            };
            let db = level(patch, row).unwrap_or(0.0);
            let response = ui
                .add_sized(
                    [64.0, 24.0],
                    button(
                        format!("{db:.1} dB"),
                        state.row == row && state.field == first + 1,
                    ),
                )
                .on_hover_text("Level · dB");
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new(("drum-level", row)), response.rect));
            if response.hovered() {
                let delta = ui.input(|i| i.raw_scroll_delta.y);
                if delta != 0.0 {
                    ui.input_mut(|i| {
                        i.raw_scroll_delta.y = 0.0;
                        i.smooth_scroll_delta.y = 0.0;
                    });
                    *action = Some(Op::Level(row, db + if delta > 0.0 { 3.0 } else { -3.0 }));
                }
            }
            if response.clicked() {
                state.row = row;
                state.field = first + 1;
                *action = Some(Op::Activate);
            }
            let mut enabled = r.swing_enabled;
            let response = ui.checkbox(&mut enabled, "Swing");
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new(("drum-swing-toggle", row)), response.rect);
            });
            if state.row == row && state.field == first + 2 {
                ui.painter()
                    .rect_stroke(response.rect, 2.0, cursor, egui::StrokeKind::Inside);
            }
            if response.clicked() {
                state.row = row;
                state.field = first + 2;
                *action = Some(Op::Swing(row));
            }
            if enabled {
                let response = ui.add_sized(
                    [40.0, 24.0],
                    button(
                        format!("{}%", r.swing),
                        state.row == row && state.field == first + 3,
                    ),
                );
                #[cfg(test)]
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new(("drum-swing", row)), response.rect));
                wheel(ui, &response, row, Field::Swing, r.swing, action);
                if response.clicked() {
                    state.row = row;
                    state.field = first + 3;
                    *action = Some(Op::Activate);
                }
            } else {
                // Keep later columns in place when the swing value is hidden.
                ui.allocate_exact_size(egui::vec2(40.0, 24.0), egui::Sense::hover());
                #[cfg(test)]
                ui.ctx().data_mut(|d| {
                    d.remove::<egui::Rect>(egui::Id::new(("drum-swing", row)));
                });
            }
            ui.label("Note");
            let response = ui.add_sized(
                [36.0, 24.0],
                button(r.output.to_string(), state.row == row && state.field == 1),
            );
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new(("drum-note", row)), response.rect));
            wheel(ui, &response, row, Field::Note, r.output, action);
            if response.clicked() {
                state.row = row;
                state.field = 1;
                *action = Some(Op::Activate);
            }
            ui.label("Vel");
            let response = ui.add_sized(
                [36.0, 24.0],
                button(
                    r.velocity.to_string(),
                    state.row == row && state.field == first,
                ),
            );
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new(("drum-velocity", row)), response.rect));
            wheel(ui, &response, row, Field::Velocity, r.velocity, action);
            if response.clicked() {
                state.row = row;
                state.field = first;
                *action = Some(Op::Activate);
            }
        });
    });
}
