//! Current-kit note selection; audition never changes the document.
use super::*;
use crate::view::commands::{Command, Screen};
#[derive(Default)]
pub(super) struct Picker {
    query: String,
    search: crate::view::commands::SearchQuery,
    selected: Option<u8>,
    editing: bool,
    focus: bool,
    reveal: bool,
}
fn candidates(a: &App) -> Vec<drum_kit::Note> {
    let l = a.loaded.as_ref().unwrap();
    let kit = l.patches[a.track].kit.as_ref().unwrap();
    let assigned = drum_kit::layout::rows(&l.music.tracks[a.track], kit);
    let p = a.drum_editor.kit.picker.as_ref().unwrap();
    drum_kit::rows(&l.music.tracks[a.track], kit)
        .into_iter()
        .rev()
        .filter(|n| {
            !assigned
                .iter()
                .enumerate()
                .any(|(i, note)| i != a.drum_editor.kit.slot && *note == Some(n.note))
                && p.search.matches_text(&format!("{} {}", n.note, n.name))
        })
        .collect()
}
pub(super) fn open(a: &mut App) -> Result<(), String> {
    a.drum_editor.kit.picker = Some(Picker {
        selected: a.drum_editor.kit.note,
        reveal: true,
        ..Default::default()
    });
    let notes = candidates(a);
    let p = a.drum_editor.kit.picker.as_mut().unwrap();
    if !notes.iter().any(|n| Some(n.note) == p.selected) {
        p.selected = notes
            .iter()
            .min_by_key(|n| n.note.abs_diff(36))
            .map(|n| n.note);
    }
    Ok(())
}
fn audition(a: &mut App) -> Result<(), String> {
    let note = a
        .drum_editor
        .kit
        .picker
        .as_ref()
        .and_then(|p| p.selected)
        .ok_or("No note")?;
    let patch = &a.loaded.as_ref().unwrap().patches[a.track];
    let velocity = patch.kit.as_ref().unwrap().velocity(note);
    a.audition.stop_key();
    if !a
        .audition
        .preview_key(a.track, patch, note, velocity, Some(0.35), None)
    {
        return Err(a.audition.error().unwrap_or("Kit preview failed").into());
    }
    a.drum_editor.kit.preview = Some((note, std::time::Instant::now()));
    Ok(())
}
fn close(a: &mut App) {
    if a.drum_editor.kit.take_preview() {
        a.audition.stop_key();
    }
    a.drum_editor.kit.picker = None;
}
fn accept(a: &mut App) -> Result<(), String> {
    let p = a.drum_editor.kit.picker.as_mut().unwrap();
    if p.editing {
        p.editing = false;
        return Ok(());
    }
    let note = p.selected.ok_or("No note")?;
    // Validate against the current result set, including duplicate-row exclusion.
    if !candidates(a).iter().any(|n| n.note == note) {
        return Err("No note".into());
    }
    row_edit::assign(a, note)?;
    close(a);
    Ok(())
}
fn move_note(a: &mut App, delta: isize) -> Result<(), String> {
    let notes = candidates(a);
    let p = a.drum_editor.kit.picker.as_mut().unwrap();
    let index = notes
        .iter()
        .position(|n| Some(n.note) == p.selected)
        .unwrap_or(0);
    p.selected = notes
        .get(
            index
                .saturating_add_signed(delta)
                .min(notes.len().saturating_sub(1)),
        )
        .map(|n| n.note);
    p.reveal = true;
    Ok(())
}
pub(super) fn collect(a: &App, screen: Screen, out: &mut Vec<Command>) {
    let editing = a.drum_editor.kit.picker.as_ref().unwrap().editing;
    let mut add = |id, label, keys: &[Key], action: fn(&mut App) -> Result<(), String>| {
        out.push(Command::plain(screen, id, label, keys, move |a, ctx| {
            if a.drum_editor.kit.composing
                || ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_))))
            {
                return;
            }
            let result = action(a);
            super::super::result(a, result);
        }));
    };
    add("kit-picker-confirm", "Note確定", &[Key::Enter], accept);
    add("kit-picker-cancel", "取消", &[Key::Escape], |a| {
        let p = a.drum_editor.kit.picker.as_mut().unwrap();
        if p.editing {
            p.query.clear();
            p.editing = false;
        } else {
            close(a);
        }
        Ok(())
    });
    if editing {
        return;
    }
    add(
        "kit-picker-down",
        "次のnote",
        &[Key::ArrowDown, Key::J],
        |a| move_note(a, 1),
    );
    add(
        "kit-picker-up",
        "前のnote",
        &[Key::ArrowUp, Key::K],
        |a| move_note(a, -1),
    );
    add("kit-picker-preview", "試聴", &[Key::Space], audition);
    add("kit-picker-query", "Note検索", &[Key::Slash], |a| {
        let p = a.drum_editor.kit.picker.as_mut().unwrap();
        p.editing = true;
        p.focus = true;
        Ok(())
    });
}
pub(super) fn show(a: &mut App, ctx: &egui::Context) {
    let p = a.drum_editor.kit.picker.as_mut().unwrap();
    p.search.update(&p.query);
    let notes = candidates(a);
    let p = a.drum_editor.kit.picker.as_mut().unwrap();
    if !notes.iter().any(|n| Some(n.note) == p.selected) {
        p.selected = notes.first().map(|n| n.note);
        p.reveal = true;
    }
    let error = a.drum_editor.error.clone();
    let mut open = true;
    let mut preview = false;
    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Note")
        .id(egui::Id::new("kit-note-picker"))
        .open(&mut open)
        .collapsible(false)
        .default_size([420.0, 420.0])
        .show(ctx, |ui| {
            if p.editing {
                let response = ui.text_edit_singleline(&mut p.query);
                if std::mem::take(&mut p.focus) {
                    response.request_focus();
                }
            } else if ui
                .button(format!("/  {}", p.query))
                .on_hover_text("Note検索 · /")
                .clicked()
            {
                p.editing = true;
                p.focus = true;
            }
            let reveal = std::mem::take(&mut p.reveal);
            let area = egui::ScrollArea::vertical()
                .animated(false)
                .max_height(320.0);
            let mut selected_rect = None;
            let scroll = area.show(ui, |ui| {
                for note in &notes {
                    let selected = p.selected == Some(note.note);
                    let r = ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new(format!("{}  {}", note.note, note.name))
                                .truncate()
                                .selected(selected),
                        )
                        .on_hover_text(&note.name);
                    if selected {
                        selected_rect = Some(r.rect);
                    }
                    #[cfg(test)]
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::new(("kit-picker-note", note.note)), r.rect)
                    });
                    if r.clicked() {
                        p.selected = Some(note.note);
                        preview = true;
                    }
                    if r.double_clicked() {
                        confirm = true;
                    }
                }
            });
            if reveal && let Some(r) = selected_rect {
                // Use the actual viewport rather than the padded clipping rect.
                let desired = (scroll.state.offset.y + r.center().y - scroll.inner_rect.center().y)
                    .clamp(
                        0.0,
                        (scroll.content_size.y - scroll.inner_rect.height()).max(0.0),
                    );
                if (desired - scroll.state.offset.y).abs() > 1.0 {
                    let mut state = scroll.state;
                    state.offset.y = desired;
                    state.store(ctx, scroll.id);
                    p.reveal = true;
                    ctx.request_repaint();
                }
            }
            #[cfg(test)]
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("kit-picker-clip"), scroll.inner_rect));
            ui.horizontal(|ui| {
                preview |= ui
                    .add_enabled(p.selected.is_some(), egui::Button::new("Space · 試聴"))
                    .clicked();
                confirm |= ui
                    .add_enabled(p.selected.is_some(), egui::Button::new("Enter · OK"))
                    .clicked();
                cancel |= ui.button("Esc · Cancel").clicked();
            });
            if let Some(error) = &error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        });
    let result = if !open || cancel {
        close(a);
        Ok(())
    } else if confirm {
        accept(a)
    } else if preview {
        audition(a)
    } else {
        Ok(())
    };
    super::super::result(a, result);
}

#[cfg(test)]
mod tests;
