//! One editor shared by timeline labels and event-list metadata.
use crate::app::App;
use cat_smf_editor::edit::trill;
use eframe::egui;

pub struct Editing {
    track: usize,
    id: usize,
    generation: u64,
    first: f64,
    interval: f64,
    upper: bool,
    error: Option<String>,
    original: cat_smf_editor::model::Music,
    previewed: bool,
    prepared: cat_smf_editor::model::Music,
    initial: (f64, f64, bool),
}

pub(crate) fn begin(app: &mut App, track: usize, event: usize) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let music = loaded.music.clone();
    begin_prepared(app, track, event, &music)
}

pub(crate) fn begin_prepared(
    app: &mut App,
    track: usize,
    event: usize,
    music: &cat_smf_editor::model::Music,
) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    if let Some(region) = trill::for_event(music, track, event)? {
        app.trill_edit = Some(Editing {
            track,
            id: region.metadata.id,
            generation: app.generation,
            first: region
                .metadata
                .first_note_seconds
                .unwrap_or(region.metadata.interval_seconds),
            interval: region.metadata.interval_seconds,
            upper: region
                .metadata
                .pattern
                .as_ref()
                .map(|p| p.first_note)
                .or_else(|| {
                    music.tracks[track]
                        .events
                        .iter()
                        .find(|e| {
                            e.tick >= region.start
                                && e.tick < region.end
                                && cat_smf_editor::score::note_view::note_on(e)
                                && e.status & 15 == region.metadata.channel - 1
                                && region.metadata.notes.contains(&e.data[0])
                        })
                        .map(|e| e.data[0])
                })
                == Some(region.metadata.notes[1]),
            error: None,
            original: loaded.music.clone(),
            previewed: false,
            prepared: music.clone(),
            initial: (0.0, 0.0, false),
        });
        let editing = app.trill_edit.as_mut().unwrap();
        editing.initial = (editing.first, editing.interval, editing.upper);
    }
    Ok(())
}

pub(crate) fn show(app: &mut App, ctx: &egui::Context) {
    let Some(mut editing) = app.trill_edit.take() else {
        return;
    };
    if editing.generation != app.generation {
        return;
    }
    let mut apply = false;
    let mut cancel = false;
    let mut changed = false;
    let response = egui::Modal::new(egui::Id::new("trill-edit")).show(ctx, |ui| {
        ui.heading("Trill");
        egui::Grid::new("trill-timing").show(ui, |ui| {
            ui.label("First pitch");
            ui.horizontal(|ui| {
                changed |= ui
                    .selectable_value(&mut editing.upper, true, "Upper")
                    .changed();
                changed |= ui
                    .selectable_value(&mut editing.upper, false, "Lower")
                    .changed();
            });
            ui.end_row();
            for (label, seconds) in [
                ("First note", &mut editing.first),
                ("Interval", &mut editing.interval),
            ] {
                ui.label(label);
                changed |= ui
                    .add(
                        egui::DragValue::new(seconds)
                            .speed(0.001)
                            .max_decimals(9)
                            .suffix(" s"),
                    )
                    .changed();
                ui.end_row();
            }
        });
        if let Some(error) = &editing.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.horizontal(|ui| {
            apply = ui.button("Apply").clicked();
            cancel = ui.button("Cancel").clicked();
        });
    });
    cancel |= response.should_close();
    if cancel {
        if !editing.previewed {
            return;
        }
        match app.apply_music(editing.original.clone()) {
            Ok(()) => return,
            Err(error) => editing.error = Some(error),
        }
    } else if changed || apply {
        let result = editing.preview(app);
        match result {
            Ok(()) => {
                editing.generation = app.generation;
                editing.error = None;
                if apply {
                    if (editing.first, editing.interval, editing.upper) != editing.initial {
                        app.undo.push(editing.original);
                    }
                    return;
                }
            }
            Err(error) => editing.error = Some(error),
        }
    }
    app.trill_edit = Some(editing);
}

impl Editing {
    fn preview(&mut self, app: &mut App) -> Result<(), String> {
        let mut music = self.original.clone();
        if (self.first, self.interval, self.upper) != self.initial {
            music = self.prepared.clone();
            trill::set_timing(&mut music, self.track, self.id, self.interval, self.first)?;
            trill::set_first_note(&mut music, self.track, self.id, self.upper)?;
            cat_smf_editor::edit::regenerate(&mut music)?;
        }
        let before = &app.loaded.as_ref().ok_or("No SMF is loaded")?.music;
        if cat_smf_editor::smf_io::encode(before)? != cat_smf_editor::smf_io::encode(&music)? {
            app.apply_music(music)?;
            self.previewed = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
