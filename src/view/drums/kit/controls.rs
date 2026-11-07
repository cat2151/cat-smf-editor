//! Row controls persist through the same patch/undo/live-reload path as the drum grid.
use super::*;

pub(super) fn swing(a: &mut App, value: Option<u8>) -> Result<(), String> {
    let Some(note) = a.drum_editor.kit.note else {
        return Ok(());
    };
    if value.is_some_and(|v| !(1..=99).contains(&v)) {
        return Err("Swing must be 1..99".into());
    }
    let mut patch = a.loaded.as_ref().ok_or("No song")?.patches[a.track].clone();
    let swing = patch
        .kit
        .as_mut()
        .ok_or("No kit")?
        .swing
        .entry(note)
        .or_insert(cat_smf_editor::drum_kit::Swing {
            enabled: false,
            amount: 50,
        });
    if let Some(value) = value {
        swing.amount = value;
    } else {
        swing.enabled = !swing.enabled;
    }
    crate::player::selection::store_track(a, a.track, &patch)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn swing_controls(
    ui: &mut egui::Ui,
    state: &mut super::super::State,
    row: &Row,
    kit: &cat_smf_editor::drum_kit::Kit,
    steps: usize,
    reveal: bool,
    cursor: egui::Stroke,
    action: &mut Option<commands::Action>,
) {
    let swing = row.note.and_then(|n| kit.swing.get(&n));
    let enabled = swing.is_some_and(|s| s.enabled);
    let selected = state.kit.slot == row.slot;
    let response = super::super::surface::swing_toggle(
        ui,
        enabled,
        selected && state.field == steps + 3,
        cursor,
        row.note.is_some(),
    )
    .on_hover_text("Swing · Enter");
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new(("kit-swing-toggle", row.slot)), response.rect));
    if reveal && selected && state.field == steps + 3 {
        response.scroll_to_me(None);
    }
    if response.clicked() {
        state.kit.slot = row.slot;
        state.kit.note = row.note;
        state.field = steps + 3;
        *action = Some(commands::Action::Swing(None));
    }
    let amount = swing.map_or(50, |s| s.amount);
    let Some(response) = super::super::surface::swing_amount(
        ui,
        enabled,
        amount,
        selected && state.field == steps + 4,
        cursor,
        egui::vec2(44.0, 28.0),
    ) else {
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.remove::<egui::Rect>(egui::Id::new(("kit-swing", row.slot))));
        return;
    };
    let response = response.on_hover_text("Swing % · Enter / Wheel");
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new(("kit-swing", row.slot)), response.rect));
    if let Some(up) = wheel(ui, &response) {
        state.kit.slot = row.slot;
        state.kit.note = row.note;
        *action = Some(commands::Action::Swing(Some(
            super::super::values::Field::Swing.stepped(amount, up),
        )));
    }
    if reveal && selected && state.field == steps + 4 {
        response.scroll_to_me(None);
    }
    if response.clicked() {
        state.kit.slot = row.slot;
        state.kit.note = row.note;
        state.field = steps + 4;
        *action = Some(commands::Action::Activate);
    }
}

pub(super) fn velocity(a: &mut App, note: u8, value: u8) -> Result<(), String> {
    let mut patch = a.loaded.as_ref().ok_or("No song")?.patches[a.track].clone();
    let kit = patch.kit.as_mut().ok_or("No kit")?;
    if !(1..=127).contains(&value) {
        return Err("Velocity must be 1..127".into());
    }
    if let Some(n) = kit.notes.iter_mut().find(|n| n.note == note) {
        n.velocity = value;
    } else {
        kit.notes.push(drum_kit::Note {
            note,
            name: String::new(),
            velocity: value,
        });
        kit.notes.sort_by_key(|n| n.note);
    }
    crate::player::selection::store_track(a, a.track, &patch)
}

pub(super) fn mute(a: &mut App) -> Result<(), String> {
    let Some(note) = a.drum_editor.kit.note else {
        return Ok(());
    };
    let mut patch = a.loaded.as_ref().ok_or("No song")?.patches[a.track].clone();
    let kit = patch.kit.as_mut().ok_or("No kit")?;
    if let Some(index) = kit.muted.iter().position(|&n| n == note) {
        kit.muted.remove(index);
    } else {
        kit.muted.push(note);
        kit.muted.sort_unstable();
    }
    a.audition.stop_key();
    a.drum_editor.kit.preview = None;
    crate::player::selection::store_track(a, a.track, &patch)
}

pub(super) fn wheel(ui: &mut egui::Ui, response: &egui::Response) -> Option<bool> {
    if !response.hovered() {
        return None;
    }
    let delta = ui.input(|i| i.raw_scroll_delta.y);
    if delta == 0.0 {
        return None;
    }
    ui.input_mut(|i| {
        i.raw_scroll_delta.y = 0.0;
        i.smooth_scroll_delta.y = 0.0;
    });
    Some(delta > 0.0)
}

/// The A and B measure buttons; the melody grid shows the same ones.
pub(in crate::view::drums) fn boundaries(
    ui: &mut egui::Ui,
    state: &super::super::State,
    mut set: impl FnMut(super::super::commands::Op),
) {
    for (end, label, number, hint) in [
        (false, "A", state.first, "[ / ]"),
        (true, "B", state.last, "Shift+[ / ]"),
    ] {
        ui.label(label);
        let response = ui.button(format!("meas. {number}")).on_hover_text(hint);
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(("kit-boundary", end)), response.rect));
        if let Some(up) = wheel(ui, &response) {
            set(super::super::commands::Op::Boundary(
                end,
                if up { 1 } else { -1 },
            ));
        }
    }
}
