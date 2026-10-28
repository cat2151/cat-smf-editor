//! Container selection precedes creation; cancellation does not mutate the song.
use super::*;
use eframe::egui::{self, Key};
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Choice {
    /// Both initial containers are created together; `true` starts in Drums.
    Initial(bool),
    Container(u64),
    /// Another container of the kind; `true` is Drums.
    New(bool),
}
pub(super) struct Pending {
    /// Only the first creation hides the existing state; later choices keep Song as it was.
    previous: Option<Box<State>>,
    pub(super) choices: Vec<(Choice, String)>,
    selected: usize,
    initial: bool,
}
/// A new container is offered only once every container of that kind holds notes,
/// so an unused one is reused instead of piling up tracks and instances.
pub(super) fn choices(a: &App) -> Result<Vec<(Choice, String)>, String> {
    let l = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    let m = &l.music;
    let c = cat_smf_editor::score::pattern::Catalog::read(m)?;
    if c.sources.is_empty() {
        return Ok(vec![
            (Choice::Initial(false), "Surge XT / Init Saw".into()),
            (Choice::Initial(true), "Drums".into()),
        ]);
    }
    let mut v: Vec<_> = c
        .tracks
        .iter()
        .filter(|(_, t)| c.sources.contains(t))
        .map(|(&id, &t)| (t, id))
        .collect();
    v.sort();
    let drums = |t: usize| l.patches.get(t).is_some_and(|p| p.drums.is_some());
    let empty = |kind: bool| {
        v.iter().any(|&(t, _)| {
            drums(t) == kind
                && !m.tracks[t]
                    .events
                    .iter()
                    .any(cat_smf_editor::score::note_view::note_on)
        })
    };
    let mut out: Vec<_> = v
        .iter()
        .map(|&(t, id)| {
            (
                Choice::Container(id),
                m.tracks[t]
                    .name()
                    .unwrap_or_else(|| format!("Container {id}")),
            )
        })
        .collect();
    for (kind, name) in [(false, "新規: Surge XT / Init Saw"), (true, "新規: Drums")] {
        if !empty(kind) {
            out.push((Choice::New(kind), name.into()));
        }
    }
    Ok(out)
}
pub(super) fn open(a: &mut App) -> Result<(), String> {
    let choices = choices(a)?;
    let initial = matches!(choices[0].0, Choice::Initial(_));
    let selected = choices
        .iter()
        .position(|(c, _)| {
            a.patterns
                .container
                .is_some_and(|id| *c == Choice::Container(id))
        })
        .unwrap_or(0);
    let previous = initial.then(|| Box::new(std::mem::take(&mut a.patterns)));
    a.patterns.first = Some(Pending {
        previous,
        choices,
        selected,
        initial,
    });
    Ok(())
}
fn edit(a: &mut App, ctx: &egui::Context) {
    let Some(p) = a.patterns.first.take() else {
        return;
    };
    let result = match p.choices[p.selected].0 {
        Choice::Initial(drums) => library::initialize(a, drums),
        Choice::Container(id) => {
            a.patterns.container = Some(id);
            library::create(a, None)
        }
        Choice::New(drums) => library::add_container(a, drums),
    };
    match result {
        Ok(()) => library::input(a, ctx),
        Err(error) => {
            a.patterns.first = Some(p);
            a.patterns.error = Some(error);
        }
    }
}
fn back(a: &mut App) {
    let Some(p) = a.patterns.first.take() else {
        return;
    };
    if let Some(previous) = p.previous {
        a.patterns = *previous;
        a.patterns.edit_song = a.song_id;
    }
    refresh(a);
}
fn movement(a: &mut App, delta: isize) {
    if let Some(p) = &mut a.patterns.first {
        p.selected = p
            .selected
            .saturating_add_signed(delta)
            .min(p.choices.len().saturating_sub(1));
    }
}
pub(super) fn collect(s: Screen, out: &mut Vec<Command>) {
    out.push(Command::plain(
        s,
        "first-pattern-edit",
        "Create",
        &[Key::Enter],
        edit,
    ));
    out.push(Command::plain(
        s,
        "first-pattern-back",
        "Cancel",
        &[Key::Escape],
        |a, _| back(a),
    ));
    for (key, delta) in [(Key::ArrowUp, -1), (Key::ArrowDown, 1)] {
        out.push(Command::plain(
            s,
            if delta < 0 {
                "first-container-prev"
            } else {
                "first-container-next"
            },
            "Container",
            &[key],
            move |a, _| movement(a, delta),
        ));
    }
}
pub(super) fn show(a: &mut App, ctx: &egui::Context) {
    let mut action = None;
    egui::Modal::new(egui::Id::new("first-pattern")).show(ctx, |ui| {
        if let Some(p) = &mut a.patterns.first {
            if p.initial {
                ui.heading("楽器を選んでください");
            }
            for (i, (_, name)) in p.choices.iter().enumerate() {
                if ui.selectable_label(p.selected == i, name).clicked() {
                    p.selected = i;
                }
            }
        }
        if let Some(error) = &a.patterns.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.horizontal(|ui| {
            for (label, confirm) in [("Enter: Create", true), ("Esc: Cancel", false)] {
                let response = ui.button(label);
                #[cfg(test)]
                ctx.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new(("first-pattern-button", confirm)),
                        response.rect,
                    )
                });
                if response.clicked() {
                    action = Some(confirm);
                }
            }
        });
    });
    if let Some(confirm) = action {
        if confirm {
            edit(a, ctx);
        } else {
            back(a);
        }
        super::super::comma_menu::swallow(ctx);
    }
}
#[cfg(test)]
mod tests;
