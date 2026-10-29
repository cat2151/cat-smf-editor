//! Recording owns its initiating viewport and suspends every background editor.
use super::{
    commands::{self, Command, Screen},
    modal::{self, Origin},
    patterns::workspace::Pane,
};
use crate::{app::App, play::audition::live};
use cat_smf_editor::score::{
    instrument::Patch,
    pattern::{recording::Take, Catalog},
    scene::arrangement::PatternOrScene,
};
use eframe::egui::{self, Key, Modifiers};
use std::time::Instant;
#[cfg(test)]
mod tests;
mod window;

pub(super) const KEYS: [(Key, u8); 7] = [
    (Key::C, 60),
    (Key::D, 62),
    (Key::E, 64),
    (Key::F, 65),
    (Key::G, 67),
    (Key::A, 69),
    (Key::B, 71),
];

#[derive(Default)]
pub struct State {
    session: Option<Session>,
}
impl State {
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }
    pub(crate) fn choosing(&self) -> bool {
        self.session.as_ref().is_some_and(|s| !s.ready)
    }
}
struct Sound {
    container: Option<u64>,
    track: usize,
    patch: Patch,
    name: String,
}
struct Session {
    origin: Origin,
    song: u64,
    take: Take,
    clock: Instant,
    sounds: Vec<Sound>,
    selected: usize,
    ready: bool,
    queue: Option<live::Queue>,
    error: Option<String>,
    handled: Option<(egui::ViewportId, u64)>,
}

pub(crate) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if a.loaded.is_some()
        && matches!(s, Screen::Main | Screen::Scene | Screen::Song)
        && !a.patterns.confirming_creation()
    {
        out.push(Command::new(
            s,
            "record-pattern",
            "Virtual MIDI keyboard",
            &[(Key::R, Modifiers::SHIFT)],
            |a, ctx| {
                if let Err(error) = open(a, ctx) {
                    a.load_error = Some(error);
                }
            },
        ));
    }
}

fn open(a: &mut App, ctx: &egui::Context) -> Result<(), String> {
    if modal::is_open(a) {
        return Ok(());
    }
    let loaded = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    let c = Catalog::read(&loaded.music)?;
    let take = Take::new(loaded.music.division)?;
    let mut sounds = c
        .tracks
        .iter()
        .filter(|(_, t)| c.sources.contains(t))
        .map(|(&id, &track)| Sound {
            container: Some(id),
            track,
            patch: loaded.patches[track].clone(),
            name: loaded.music.tracks[track]
                .name()
                .unwrap_or_else(|| format!("Container {id}")),
        })
        .collect::<Vec<_>>();
    sounds.sort_by_key(|s| s.track);
    let target = if let Some(pane) = super::patterns::workspace::pane(ctx) {
        match a.patterns.workspace.selection(pane).choice {
            Some(PatternOrScene::Pattern(id)) => c.patterns.get(&id).map(|p| p.container),
            _ => a.patterns.container,
        }
    } else {
        c.tracks
            .iter()
            .find_map(|(&id, &track)| (track == a.track).then_some(id))
    };
    if sounds.is_empty() {
        sounds.push(Sound {
            container: None,
            track: loaded.music.tracks.len(),
            patch: Patch::default(),
            name: "Init Saw".into(),
        });
    }
    let selected = sounds.iter().position(|s| s.container == target);
    let ready = selected.is_some() || sounds.len() == 1;
    crate::play::transport::stop(a);
    super::piano_roll::cancel_keyboard(a, ctx);
    let mut session = Session {
        origin: Origin::capture(a, ctx),
        song: a.song_id,
        take,
        clock: Instant::now(),
        sounds,
        selected: selected.unwrap_or(0),
        ready,
        queue: None,
        error: None,
        handled: None,
    };
    if ready {
        prepare(a, &mut session);
    }
    a.patterns.recording.session = Some(session);
    modal::mark_owned(ctx);
    modal::clear(ctx);
    ctx.request_repaint();
    Ok(())
}

fn prepare(a: &mut App, s: &mut Session) {
    let sound = &s.sounds[s.selected];
    if sound.patch.drums.is_some() {
        s.error = Some("C–B recording requires a melodic container".into());
        s.ready = false;
        return;
    }
    s.queue = a.audition.start_live(sound.track, &sound.patch);
    s.error = s.queue.is_none().then(|| {
        a.audition
            .error()
            .unwrap_or("Cannot open recording instrument")
            .to_owned()
    });
    s.ready = s.queue.is_some();
}

pub(crate) fn begin(a: &mut App, ctx: &egui::Context) -> Option<egui::InputState> {
    Some(modal::suspend(
        ctx,
        &a.patterns.recording.session.as_ref()?.origin,
    ))
}

pub(crate) fn end(a: &mut App, ctx: &egui::Context, input: Option<egui::InputState>) {
    let Some(input) = input else { return };
    let mut state = std::mem::take(&mut a.patterns.recording);
    let Some(s) = state.session.as_mut() else {
        return;
    };
    if s.origin.viewport != ctx.viewport_id() {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("modal-suspended"), false));
        a.patterns.recording = state;
        modal::clear(ctx);
        return;
    }
    modal::resume(ctx, input);
    let token = modal::frame_token(ctx);
    let mut done = a.song_id != s.song || !s.origin.target_current(a);
    if done {
        a.audition.stop_key();
    }
    if !done && s.handled != Some(token) {
        s.handled = Some(token);
        done = process(a, ctx, s);
    }
    if !done {
        window::show(a, ctx, s);
    }
    if done {
        s.origin.restore_focus(ctx);
        ctx.send_viewport_cmd_to(s.origin.viewport, egui::ViewportCommand::Focus);
        state.session = None;
    }
    a.patterns.recording = state;
    modal::mark_owned(ctx);
    modal::clear(ctx);
    ctx.request_repaint_after(std::time::Duration::from_millis(8));
}

fn process(a: &mut App, ctx: &egui::Context, s: &mut Session) -> bool {
    let at = s.clock.elapsed();
    let focused = ctx.input(|i| {
        i.focused
            && !i
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(false)))
    });
    if !focused {
        release_all(s, at);
        return false;
    }
    for event in ctx.input(|i| i.events.clone()) {
        let egui::Event::Key {
            key,
            pressed,
            repeat,
            modifiers,
            ..
        } = event
        else {
            continue;
        };
        if key == Key::Escape && pressed && !repeat {
            if commands::matches(modifiers, Modifiers::SHIFT) {
                a.audition.stop_key();
                return true;
            }
            release_all(s, at);
            if !s.take.started() {
                a.audition.stop_key();
                return true;
            }
            match finish(a, s) {
                Ok(()) => return true,
                Err(error) => {
                    s.error = Some(error);
                    return false;
                }
            }
        }
        if !s.ready {
            if pressed && !repeat {
                match key {
                    Key::ArrowUp => s.selected = s.selected.saturating_sub(1),
                    Key::ArrowDown => s.selected = (s.selected + 1).min(s.sounds.len() - 1),
                    Key::Enter => prepare(a, s),
                    _ => {}
                }
            }
            continue;
        }
        let Some((_, pitch)) = KEYS.iter().find(|(k, _)| *k == key) else {
            continue;
        };
        if pressed && (repeat || !commands::matches(modifiers, Modifiers::NONE)) {
            continue;
        }
        let result = if pressed {
            s.take.press(*pitch, at)
        } else {
            s.take.release(*pitch, at)
        };
        match result {
            Ok(true) => {
                if let Some(q) = &s.queue {
                    live::push(q, *pitch, pressed);
                }
            }
            Ok(false) => {}
            Err(error) => s.error = Some(error),
        }
    }
    false
}

fn release_all(s: &mut Session, at: std::time::Duration) {
    match s.take.release_all(at) {
        Ok(pitches) => {
            if let Some(q) = &s.queue {
                for pitch in pitches {
                    live::push(q, pitch, false);
                }
            }
        }
        Err(error) => s.error = Some(error),
    }
}

fn finish(a: &mut App, s: &Session) -> Result<(), String> {
    let sound = &s.sounds[s.selected];
    let source = if let Some(container) = sound.container {
        Some(
            *Catalog::read(&a.loaded.as_ref().ok_or("No SMF is loaded")?.music)?
                .tracks
                .get(&container)
                .ok_or("Recording container disappeared")?,
        )
    } else {
        None
    };
    let mut created = None;
    a.edit_tracks(|music| {
        created = Some(s.take.commit(music, source)?);
        Ok(())
    })?;
    let id = created.ok_or("Missing recorded pattern")?;
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music)?;
    a.patterns.selected = Some(id);
    a.patterns.container = Some(catalog.patterns[&id].container);
    if let Some(p) = [Pane::Scene, Pane::Song]
        .into_iter()
        .find(|p| p.viewport() == s.origin.viewport)
    {
        a.patterns.workspace.selection_mut(p).choice = Some(PatternOrScene::Pattern(id));
        a.patterns.workspace.selection_mut(p).query.clear();
    } else {
        super::patterns::library::edit(a, id)?;
    }
    Ok(())
}
