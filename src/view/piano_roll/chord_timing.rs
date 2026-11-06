//! `c` mode: moves, stretches and quantizes the chord under the cursor while its measure loops.
use super::*;
use crate::view::commands::{Command, Screen};
use cat_smf_editor::edit::{chord_timing, quantize};
use eframe::egui::{Key, Modifiers};

pub(crate) struct Session {
    song: u64,
    track: usize,
    /// Onset of the selected chord, in the ticks the event list shows.
    tick: u64,
    /// The mode's first edit pushed the undo step that all later edits fold into.
    merged: bool,
    /// The A-B range this mode started.
    repeat: Option<(u64, u64)>,
}

#[derive(Clone, Copy)]
enum Action {
    Step(bool),
    Move(i64),
    Length(i64),
    Quantize,
    Confirm,
    Cancel,
}

/// h j k l and the arrows are one set: j k pick a chord, h l move it, Shift+h l move its end.
const PLAIN: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;

/// `(id, name, keys, button, action)` of each key, in the overlay's order.
type Binding = (
    &'static str,
    &'static str,
    &'static [(Key, Modifiers)],
    &'static str,
    Action,
);
const ACTIONS: [Binding; 9] = [
    (
        "chord-next",
        "次のchord",
        &[(Key::J, PLAIN), (Key::ArrowDown, PLAIN)],
        "j: Next",
        Action::Step(true),
    ),
    (
        "chord-prev",
        "前のchord",
        &[(Key::K, PLAIN), (Key::ArrowUp, PLAIN)],
        "k: Previous",
        Action::Step(false),
    ),
    (
        "chord-earlier",
        "16分前へ",
        &[(Key::H, PLAIN), (Key::ArrowLeft, PLAIN)],
        "h: Earlier 1/16",
        Action::Move(-1),
    ),
    (
        "chord-later",
        "16分後ろへ",
        &[(Key::L, PLAIN), (Key::ArrowRight, PLAIN)],
        "l: Later 1/16",
        Action::Move(1),
    ),
    (
        "chord-shorter",
        "16分短く",
        &[(Key::H, SHIFT), (Key::ArrowLeft, SHIFT)],
        "H: Shorter 1/16",
        Action::Length(-1),
    ),
    (
        "chord-longer",
        "16分長く",
        &[(Key::L, SHIFT), (Key::ArrowRight, SHIFT)],
        "L: Longer 1/16",
        Action::Length(1),
    ),
    (
        "chord-quantize",
        "16分quantize",
        &[(Key::Q, PLAIN)],
        "q: Quantize 1/16",
        Action::Quantize,
    ),
    (
        "chord-confirm",
        "確定",
        &[(Key::Enter, PLAIN)],
        "Enter: OK",
        Action::Confirm,
    ),
    (
        "chord-cancel",
        "取消",
        &[(Key::Escape, PLAIN)],
        "Esc: Cancel",
        Action::Cancel,
    ),
];

fn onsets(app: &App) -> Vec<u64> {
    app.loaded.as_ref().map_or_else(Vec::new, |l| {
        chord_timing::onsets(&l.visible().tracks[app.track])
    })
}

fn can_begin(app: &App) -> bool {
    app.piano_roll.open
        && !app.tempo_curve.open
        && !app.parameter_editor.open
        && app.can_edit_notes(app.track)
        && !onsets(app).is_empty()
}

pub(crate) fn active(app: &App) -> bool {
    app.piano_roll.chord.is_some()
}

/// Selects the first chord from the cursor on, or the last one before it.
fn begin(app: &mut App) {
    let onsets = onsets(app);
    let from = crate::play::playback::location(app).0;
    let Some(&tick) = onsets.iter().find(|&&t| t >= from).or(onsets.last()) else {
        return;
    };
    app.piano_roll.error = None;
    app.piano_roll.chord = Some(Session {
        song: app.song_id,
        track: app.track,
        tick,
        merged: false,
        repeat: None,
    });
    select(app, tick);
}

/// The measure holding `tick`, start inclusive and end exclusive.
fn measure(app: &App, tick: u64) -> Option<(u64, u64)> {
    let map = app.loaded.as_ref()?.time.map(app.track)?;
    let mut position = map.position(tick);
    position.beat = 1;
    position.tick_numerator = 0;
    position.tick_denominator = 1;
    let start = map.tick_at(&position)?;
    position.measure += 1;
    Some((start, map.tick_at(&position)?))
}

/// Selects the chord at `tick` and loops its measure.
fn select(app: &mut App, tick: u64) {
    let range = measure(app, tick);
    if let Some(range) = range
        && app.playback.repeat != Some(range)
    {
        crate::play::transport::repeat::start(app, range);
    }
    crate::cursor::moves::at_tick(app, tick);
    let Some(session) = &mut app.piano_roll.chord else {
        return;
    };
    session.tick = tick;
    if session.repeat != range {
        session.repeat = range;
        app.piano_roll.reveal = true;
    }
}

fn run(app: &mut App, action: Action) {
    let Some(session) = &app.piano_roll.chord else {
        return;
    };
    let (track, tick, merged) = (session.track, session.tick, session.merged);
    let change = match action {
        Action::Step(forward) => {
            let onsets = onsets(app);
            let next = if forward {
                onsets.iter().find(|&&t| t > tick)
            } else {
                onsets.iter().rev().find(|&&t| t < tick)
            };
            if let Some(&next) = next {
                select(app, next);
            }
            return;
        }
        Action::Confirm => return finish(app),
        Action::Cancel => {
            finish(app);
            if merged && let Err(error) = app.undo_music() {
                app.piano_roll.error = Some(error);
            }
            return;
        }
        Action::Move(steps) | Action::Length(steps) => {
            let grid = match grid(app) {
                Ok(grid) => grid as i64 * steps,
                Err(error) => return app.piano_roll.error = Some(error),
            };
            if matches!(action, Action::Move(_)) {
                chord_timing::Change::Move(grid)
            } else {
                chord_timing::Change::Length(grid)
            }
        }
        Action::Quantize => match grid(app) {
            Ok(grid) => chord_timing::Change::Quantize(grid),
            Err(error) => return app.piano_roll.error = Some(error),
        },
    };
    let pushes = app.undo.pushes();
    let mut next = tick;
    let result = app.edit_music(|music| {
        for on in chord_timing::notes(&music.tracks[track], tick) {
            cat_smf_editor::edit::trill::ensure_note_editable(music, track, on)?;
        }
        next = chord_timing::apply(&mut music.tracks[track], tick, change)?;
        Ok(())
    });
    if let Err(error) = result {
        app.piano_roll.error = Some(error);
        return;
    }
    app.piano_roll.error = None;
    if app.undo.pushes() != pushes {
        if merged {
            app.undo.discard_newest();
        } else if let Some(session) = &mut app.piano_roll.chord {
            session.merged = true;
        }
    }
    select(app, next);
}

fn grid(app: &App) -> Result<u64, String> {
    quantize::sixteenth(app.loaded.as_ref().unwrap().visible().division)
}

/// Leaves the mode and stops the loop it started.
fn finish(app: &mut App) {
    let Some(session) = app.piano_roll.chord.take() else {
        return;
    };
    if session.repeat.is_some() && app.playback.repeat == session.repeat {
        crate::play::transport::stop(app);
    }
}

/// Ends the mode once its roll, song or track is gone.
pub(super) fn validate(app: &mut App) {
    if app.piano_roll.chord.as_ref().is_some_and(|s| {
        !app.piano_roll.open
            || s.song != app.song_id
            || s.track != app.track
            || app.loaded.is_none()
    }) {
        finish(app);
    }
}

/// Note On indices of the selected chord in `track`.
pub(super) fn highlight(app: &App, track: usize) -> Vec<usize> {
    match (&app.piano_roll.chord, &app.loaded) {
        (Some(s), Some(l)) if s.track == track => {
            chord_timing::notes(&l.visible().tracks[track], s.tick)
        }
        _ => vec![],
    }
}

pub(super) fn show(app: &mut App, ctx: &egui::Context) {
    if !active(app) {
        return;
    }
    let mut clicked = None;
    egui::Window::new("Chord timing")
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
        .show(ctx, |ui| {
            egui::Grid::new("chord-timing").show(ui, |ui| {
                for (i, &(.., label, action)) in ACTIONS.iter().enumerate() {
                    if ui.button(label).clicked() {
                        clicked = Some(action);
                    }
                    // j k, h l, H L, q, Enter Esc.
                    if matches!(i, 1 | 3 | 5 | 6) {
                        ui.end_row();
                    }
                }
            });
        });
    if let Some(action) = clicked {
        run(app, action);
    }
}

pub(crate) fn collect(app: &App, s: Screen, out: &mut Vec<Command>) {
    if s == Screen::Timeline && can_begin(app) {
        out.push(Command::plain(
            s,
            "chord-timing",
            "Chord timing",
            &[Key::C],
            |a, _| begin(a),
        ));
    }
    if s != Screen::ChordTiming {
        return;
    }
    out.push(Command::plain(
        s,
        "help",
        "ヘルプ",
        &[Key::F1],
        |a, _| crate::view::help::summon(a),
    ));
    for (id, name, keys, _, action) in ACTIONS {
        let command = Command::new(s, id, name, keys, move |a, _| run(a, action));
        out.push(match action {
            Action::Move(_) | Action::Length(_) | Action::Step(_) => command.repeating(),
            _ => command,
        });
    }
}

#[cfg(test)]
pub(crate) mod tests;
