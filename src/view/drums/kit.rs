//! Multiple actual-note rows over one instrument; projection and transport stay shared.
use super::*;
use cat_smf_editor::score::drum_kit;
pub(crate) mod commands;
#[cfg(test)]
mod control_tests;
pub(super) mod controls;
mod picker;
mod row_edit;
mod widgets;
#[derive(Default)]
pub(crate) struct State {
    note: Option<u8>,
    slot: usize,
    picker: Option<picker::Picker>,
    query: String,
    search: crate::view::commands::SearchQuery,
    query_before: Option<(String, usize)>,
    input: Option<Velocity>,
    menu: bool,
    focus: bool,
    reveal: bool,
    composing: bool,
    /// The placed or picked note sounding briefly; the melody grid shares it.
    pub(super) preview: Option<(u8, std::time::Instant)>,
}
struct Velocity {
    hit: bool,
    swing: bool,
    note: u8,
    step: usize,
    number: u64,
    text: String,
}
impl State {
    pub(super) fn take_preview(&mut self) -> bool {
        self.preview.take().is_some()
    }
    pub(crate) fn has_input(&self) -> bool {
        self.query_before.is_some() || self.input.is_some() || self.menu || self.picker.is_some()
    }
}
pub(crate) fn active(a: &App) -> bool {
    a.loaded
        .as_ref()
        .is_some_and(|l| l.patches[a.track].kit.is_some())
}
struct Row {
    slot: usize,
    note: Option<u8>,
    name: String,
    velocity: u8,
}
fn rows(a: &App) -> Vec<Row> {
    let Some(l) = &a.loaded else {
        return vec![];
    };
    let Some(kit) = &l.patches[a.track].kit else {
        return vec![];
    };
    drum_kit::layout::rows(&l.music.tracks[a.track], kit)
        .into_iter()
        .enumerate()
        .map(|(slot, note)| Row {
            slot,
            note,
            name: note
                .and_then(|note| kit.notes.iter().find(|n| n.note == note))
                .map_or(String::new(), |n| n.name.clone()),
            velocity: note.map_or(100, |n| kit.velocity(n)),
        })
        .filter(|n| {
            a.drum_editor.kit.search.matches_text(&format!(
                "{} {}",
                n.note.map_or(String::new(), |n| n.to_string()),
                n.name
            ))
        })
        .collect()
}
fn sync(a: &mut App) {
    a.drum_editor.kit.search.update(&a.drum_editor.kit.query);
    let rows = rows(a);
    let k = &mut a.drum_editor.kit;
    if let Some(row) = rows.iter().find(|r| r.slot == k.slot).or(rows.first()) {
        k.slot = row.slot;
        k.note = row.note;
    } else {
        k.note = None;
    }
    a.drum_editor.row = k.slot;
    let enabled = a.drum_editor.kit.note.is_some_and(|note| {
        a.loaded.as_ref().unwrap().patches[a.track]
            .kit
            .as_ref()
            .unwrap()
            .swing
            .get(&note)
            .is_some_and(|s| s.enabled)
    });
    let last = playback::measure(a).map_or(16, |m| m.steps.len()) + if enabled { 4 } else { 3 };
    a.drum_editor.field = a.drum_editor.field.min(last);
}
fn activate(a: &mut App) -> Result<(), String> {
    let field = a.drum_editor.field;
    let steps = playback::measure(a)?.steps.len();
    match field {
        0 => picker::open(a),
        1 if a.drum_editor.kit.note.is_none() => Ok(()),
        1 => begin_velocity(a, false),
        f if f == steps + 3 => controls::swing(a, None),
        f if f == steps + 4 && a.drum_editor.kit.note.is_some() => {
            let note = a.drum_editor.kit.note.ok_or("No note")?;
            let amount = a.loaded.as_ref().unwrap().patches[a.track]
                .kit
                .as_ref()
                .unwrap()
                .swing
                .get(&note)
                .map_or(50, |s| s.amount);
            a.drum_editor.kit.input = Some(Velocity {
                hit: false,
                swing: true,
                note,
                step: 0,
                number: 0,
                text: amount.to_string(),
            });
            a.drum_editor.kit.focus = true;
            Ok(())
        }
        f if f == steps + 2 => {
            a.audition.stop_key();
            crate::player::kit_selection::open(a, false, false);
            Ok(())
        }
        f => {
            let Some(note) = a.drum_editor.kit.note else {
                return Ok(());
            };
            let (track, number) = (a.track, a.drum_editor.page + 1);
            let velocity = a.loaded.as_ref().unwrap().patches[track]
                .kit
                .as_ref()
                .unwrap()
                .velocity(note);
            a.edit_music(|m| drum_kit::toggle_step(m, track, note, number, f - 2, velocity))
        }
    }
}
fn begin_velocity(a: &mut App, hit: bool) -> Result<(), String> {
    let (note, step, number) = (
        a.drum_editor.kit.note.ok_or("No note")?,
        a.drum_editor.field.saturating_sub(2),
        a.drum_editor.page + 1,
    );
    let l = a.loaded.as_ref().ok_or("No song")?;
    let value = if hit {
        drum_kit::hits(
            &l.visible().tracks[a.track],
            &playback::measure(a)?,
            note,
            step,
        )
        .first()
        .ok_or("Empty step")?
        .1
        .data[1]
    } else {
        l.patches[a.track].kit.as_ref().unwrap().velocity(note)
    };
    a.drum_editor.kit.input = Some(Velocity {
        hit,
        swing: false,
        note,
        step,
        number,
        text: value.to_string(),
    });
    a.drum_editor.kit.menu = false;
    a.drum_editor.kit.focus = true;
    Ok(())
}
fn confirm(a: &mut App) -> Result<(), String> {
    if a.drum_editor.kit.query_before.take().is_some() {
        return Ok(());
    }
    if a.drum_editor.kit.menu {
        return begin_velocity(a, true);
    }
    let input = a.drum_editor.kit.input.as_ref().ok_or("No input")?;
    let swing = input.swing;
    let value = input
        .text
        .parse::<u8>()
        .ok()
        .filter(|v| (1..=if swing { 99 } else { 127 }).contains(v))
        .ok_or(if swing {
            "Swing must be 1..99"
        } else {
            "Velocity must be 1..127"
        })?;
    let (note, number, step, hit) = (input.note, input.number, input.step, input.hit);
    let track = a.track;
    if swing {
        controls::swing(a, Some(value))?;
    } else if hit {
        a.edit_music(|m| drum_kit::velocity(m, track, note, number, step, value))?;
    } else {
        controls::velocity(a, note, value)?;
    }
    a.drum_editor.kit.input = None;
    Ok(())
}
fn cancel(a: &mut App) {
    let k = &mut a.drum_editor.kit;
    if let Some((query, slot)) = k.query_before.take() {
        k.query = query;
        k.slot = slot;
        sync(a);
    } else if k.input.take().is_some() {
    } else if k.menu {
        k.menu = false;
    } else {
        a.audition.stop_key();
        super::dismiss(a);
    }
}
/// A placed-note preview sounds for 350 ms; the melody grid shares it.
pub(super) fn expire_preview(a: &mut App) {
    if a.drum_editor
        .kit
        .preview
        .is_some_and(|(_, t)| t.elapsed().as_millis() >= 350)
    {
        a.audition.stop_key();
        a.drum_editor.kit.preview = None;
    }
}
/// A pane leaves the keys to the window holding it, which routed them before its panes.
pub(super) fn show(a: &mut App, ctx: &egui::Context, pane: Option<&mut egui::Ui>) {
    sync(a);
    expire_preview(a);
    for event in ctx.input(|i| i.events.clone()) {
        if let egui::Event::Ime(event) = event {
            a.drum_editor.kit.composing =
                matches!(event, egui::ImeEvent::Enabled | egui::ImeEvent::Preedit(_));
        }
    }
    if pane.is_none() {
        crate::view::commands::route(a, ctx);
    }
    if !super::is_open(a) {
        return;
    }
    if a.drum_editor.kit.picker.is_some() {
        picker::show(a, ctx);
    } else {
        widgets::show(a, ctx, pane);
    }
}

#[cfg(test)]
mod row_tests;
#[cfg(test)]
mod tests;
