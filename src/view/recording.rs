//! The keyboard windows sit above the editor beside the windows that stay usable.
use super::{
    commands::{self, Command, Screen},
    modal,
    patterns::workspace::Pane,
};
use crate::{app::App, play::audition::live};
use cat_smf_editor::score::{
    instrument::Patch,
    key_signature::KeySignature,
    pattern::{
        recording::{self, Recorder, Settings},
        Catalog,
    },
    scene::arrangement::PatternOrScene,
};
use eframe::egui::{self, Key, Modifiers};
use std::time::Instant;
mod companions;
mod pads;
mod piano;
mod play;
mod setup;
#[cfg(test)]
mod tests;
pub(crate) mod window;
pub(crate) use companions::reset as reset_windows;
pub(crate) use companions::{piano_viewport, settings_viewport};

#[derive(Default)]
pub struct State {
    session: Option<Session>,
    /// Settings and the loop length survive closing; the key is reread at every open.
    kept: Option<(Settings, u16)>,
}
impl State {
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }
    /// The keyboard or one of its companion windows.
    pub(crate) fn owns(&self, viewport: egui::ViewportId) -> bool {
        self.is_open()
            && [window::viewport(), settings_viewport(), piano_viewport()].contains(&viewport)
    }
    #[cfg(test)]
    pub(crate) fn preparing(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.recorder.is_none())
    }
}
struct Sound {
    container: Option<u64>,
    track: usize,
    patch: Patch,
    name: String,
}
struct Session {
    /// The window the keyboard opened from; it gets the takes and the focus back.
    origin: egui::ViewportId,
    /// The editor window that owns the keyboard windows, keeping them above it.
    owner: Option<isize>,
    /// Titles of the open windows `owner` already owns.
    owned: Vec<&'static str>,
    song: u64,
    division: u16,
    clock: Instant,
    sounds: Vec<Sound>,
    selected: usize,
    /// Used while no instrument plays; the recorder owns a copy while playing.
    settings: Settings,
    /// Loop length, also remembered while the range is free.
    measures: u16,
    /// Selected row of the settings window.
    item: setup::Item,
    settings_open: bool,
    piano_open: bool,
    /// The keyboard came to the front, so its companions follow it there.
    raise: bool,
    /// Where each open companion window opened, fixed while it is open so it can be moved.
    companions: companions::Places,
    /// The pad the mouse holds in each `window::Pointer` window.
    mouse: [Option<recording::Pad>; 2],
    /// Present while playing.
    recorder: Option<Recorder>,
    queue: Option<live::Queue>,
    /// The recording voice's metronome; its first click fixes the loop origin.
    click: Option<std::sync::Arc<crate::play::audition::click::Click>>,
    error: Option<String>,
    /// A transport button clicked while drawing, handled after it.
    clicked: Option<window::Button>,
    /// The last frame of each window whose keys were read.
    handled: Vec<(egui::ViewportId, u64)>,
    /// Where the keyboard window opens on the screen.
    position: egui::Pos2,
    /// Reads a Win32 virtual key; replaced by tests.
    down: fn(u8) -> bool,
}
impl Session {
    fn settings(&self) -> &Settings {
        self.recorder
            .as_ref()
            .map_or(&self.settings, Recorder::settings)
    }
    fn settings_mut(&mut self) -> &mut Settings {
        self.recorder
            .as_mut()
            .map_or(&mut self.settings, Recorder::settings_mut)
    }
    /// `false` when this frame of the window already read its keys.
    fn first_read(&mut self, ctx: &egui::Context) -> bool {
        let token = modal::frame_token(ctx);
        let fresh = !self.handled.contains(&token);
        if fresh {
            self.handled.retain(|(id, _)| *id != token.0);
            self.handled.push(token);
        }
        fresh
    }
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

/// Moves the session clock as if `by` had passed.
#[cfg(test)]
pub(crate) fn rewind(a: &mut App, by: std::time::Duration) {
    a.patterns.recording.session.as_mut().unwrap().clock -= by;
}

#[cfg(test)]
pub(crate) fn sounding(a: &App, pitch: u8) -> bool {
    a.patterns
        .recording
        .session
        .as_ref()
        .is_some_and(|s| s.recorder.as_ref().is_some_and(|r| r.sounding(pitch)))
}

/// Plays one audio block of the recording voice's metronome, as the output stream would.
#[cfg(test)]
pub(crate) fn render_click(a: &mut App) {
    let s = a.patterns.recording.session.as_mut().unwrap();
    s.click
        .as_ref()
        .unwrap()
        .render(&mut [0.0; 2048], 2, 48000, Instant::now());
    play::follow_click(s);
}

/// The conductor's key at tick 0.
fn song_key(music: &cat_smf_editor::model::Music) -> recording::Key {
    recording::Key::from_signature(music.tracks.first().and_then(|t| {
        t.events
            .iter()
            .take_while(|e| e.tick == 0)
            .find_map(KeySignature::from_event)
    }))
}

fn open(a: &mut App, ctx: &egui::Context) -> Result<(), String> {
    if a.patterns.recording.is_open() {
        ctx.send_viewport_cmd_to(window::viewport(), egui::ViewportCommand::Focus);
        return Ok(());
    }
    if modal::is_open(a) {
        return Ok(());
    }
    let loaded = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    let c = Catalog::read(&loaded.music)?;
    let key = song_key(&loaded.music);
    let (mut settings, measures) = a.patterns.recording.kept.unwrap_or_else(|| {
        (
            Settings::new(key, setup::tempo(a.patterns.tempo.0.round() as i32)),
            4,
        )
    });
    if settings.key() != key {
        settings.set_key(key);
    }
    Recorder::new(loaded.music.division, settings)?;
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
    // No container is implied, so the settings open on the container row.
    let ambiguous = selected.is_none() && sounds.len() > 1;
    let division = loaded.music.division;
    crate::play::transport::stop(a);
    super::piano_roll::cancel_keyboard(a, ctx);
    let mut session = Session {
        origin: ctx.viewport_id(),
        #[cfg(windows)]
        owner: super::analysis::owner::editor(),
        #[cfg(not(windows))]
        owner: None,
        owned: vec![],
        song: a.song_id,
        division,
        clock: Instant::now(),
        sounds,
        selected: selected.unwrap_or(0),
        settings,
        measures,
        item: setup::Item::Sound,
        settings_open: false,
        piano_open: true,
        raise: false,
        companions: companions::Places::default(),
        mouse: [None; 2],
        recorder: None,
        queue: None,
        click: None,
        error: None,
        clicked: None,
        handled: vec![],
        position: companions::saved(a, companions::KEYBOARD).unwrap_or_else(|| {
            let screen = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                ctx.input(|i| i.viewport().monitor_size)
                    .unwrap_or(egui::vec2(1920.0, 1080.0)),
            );
            window::place(
                screen,
                &window::covers(a, ctx),
                window::SIZE + window::FRAME,
            )
        }),
        down: pads::key_down,
    };
    if !ambiguous {
        session.item = setup::first(&session);
    }
    // Pads come first; a sound that cannot play opens the settings beside the error.
    session.settings_open = !play::start(a, &mut session) || ambiguous;
    a.patterns.recording.session = Some(session);
    modal::mark_owned(ctx);
    modal::clear(ctx);
    ctx.request_repaint();
    Ok(())
}

/// Shows the keyboard windows; the other windows keep their own input.
pub(crate) fn show(a: &mut App, ctx: &egui::Context) {
    let mut state = std::mem::take(&mut a.patterns.recording);
    let Some(s) = state.session.as_mut() else {
        a.patterns.recording = state;
        return;
    };
    let mut done = a.song_id != s.song;
    if !done {
        let builder = egui::ViewportBuilder::default()
            .with_title(window::TITLE)
            .with_position(s.position)
            .with_inner_size(window::SIZE);
        let mut moved = None;
        done = ctx.show_viewport_immediate(window::viewport(), builder, |ctx, class| {
            let native = class != egui::ViewportClass::Embedded;
            if native {
                moved = ctx.input(|i| i.viewport().outer_rect).map(|r| r.min);
                if ctx.input(|i| i.viewport().close_requested()) {
                    return play::close(a, s, s.clock.elapsed());
                }
            }
            super::help::observe_recording(a, ctx);
            if s.first_read(ctx) && play::process(a, ctx, s) {
                return true;
            }
            window::show(ctx, class, s);
            s.clicked.take().is_some_and(|b| play::button(a, s, b))
        });
        if let Some(at) = moved {
            companions::remember(a, companions::KEYBOARD, at);
        }
        if !done {
            let keyboard = moved.unwrap_or(s.position);
            companions::show(a, ctx, s, keyboard);
            adopt(s);
        }
    }
    if done {
        ctx.send_viewport_cmd_to(s.origin, egui::ViewportCommand::Focus);
        shut(a, &mut state);
    }
    a.patterns.recording = state;
    ctx.request_repaint_after(std::time::Duration::from_millis(8));
}

/// Gives each open keyboard window to the editor once its native window exists.
fn adopt(s: &mut Session) {
    let open = [
        (window::TITLE, true),
        (companions::SETTINGS_TITLE, s.settings_open),
        (companions::PIANO_TITLE, s.piano_open),
    ];
    // A closed companion gets a new native window when it reopens.
    s.owned
        .retain(|t| open.iter().any(|(title, shown)| title == t && *shown));
    #[cfg(windows)]
    if let Some(owner) = s.owner {
        for (title, shown) in open {
            if shown && !s.owned.contains(&title) && super::analysis::owner::adopt_by(owner, title)
            {
                s.owned.push(title);
            }
        }
    }
}

fn shut(a: &mut App, state: &mut State) {
    if let Some(s) = state.session.take() {
        a.audition.stop_live();
        state.kept = Some((*s.settings(), s.measures));
    }
}

/// Closes the keyboard window as its own close button does, so the exit confirmation gets input.
/// `false` keeps it open with the commit error.
pub(crate) fn close_for_exit(a: &mut App) -> bool {
    let mut state = std::mem::take(&mut a.patterns.recording);
    let done = state
        .session
        .as_mut()
        .is_none_or(|s| play::close(a, s, s.clock.elapsed()));
    if done {
        shut(a, &mut state);
    }
    a.patterns.recording = state;
    done
}

fn finish(a: &mut App, s: &Session) -> Result<(), String> {
    let recorder = s.recorder.as_ref().ok_or("Recording has not started")?;
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
    let mut created = vec![];
    a.edit_tracks(|music| {
        created = recorder.commit(music, source)?;
        Ok(())
    })?;
    let id = *created.last().ok_or("Missing recorded pattern")?;
    // Candidates audition at the pattern preview tempo, so takes replay as recorded.
    a.patterns.tempo.0 = 60_000_000.0 / f64::from(recorder.settings().tempo);
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music)?;
    a.patterns.selected = Some(id);
    a.patterns.container = Some(catalog.patterns[&id].container);
    if let Some(p) = [Pane::Scene, Pane::Song]
        .into_iter()
        .find(|p| p.viewport() == s.origin)
    {
        a.patterns.workspace.selection_mut(p).choice = Some(PatternOrScene::Pattern(id));
        a.patterns.workspace.selection_mut(p).query.clear();
    } else {
        super::patterns::library::edit(a, id)?;
    }
    Ok(())
}
