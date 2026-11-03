//! The keyboard windows sit above the editor beside the windows that stay usable.
use super::{
    commands::{self, Command, Screen},
    modal,
    patterns::workspace::Pane,
};
use crate::{app::App, play::audition::live};
use cat_smf_editor::message;
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
mod capture;
mod companions;
mod pads;
mod panel;
mod phrase;
mod piano;
mod play;
mod resume;
mod setup;
mod take;
#[cfg(test)]
mod tests;
pub(crate) mod window;
pub(crate) use companions::reset as reset_windows;
pub(crate) use companions::{piano_viewport, settings_viewport};
pub(crate) use panel::LOOPING;
pub(crate) use resume::begin as resume;
use take::{finish, reveal};

#[derive(Default)]
pub struct State {
    session: Option<Session>,
    /// The keyboard reopens at startup once the Song window is up.
    resuming: bool,
}
impl State {
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }
    /// The keyboard or one of its companion windows.
    pub(crate) fn owns(&self, viewport: egui::ViewportId) -> bool {
        self.is_open()
            && [
                window::viewport(),
                settings_viewport(),
                piano_viewport(),
                capture::viewport(),
            ]
            .contains(&viewport)
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
    /// Its keys and phrase, playing beside the other containers' phrases.
    live: live::Live,
    looper: phrase::Looper,
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
    /// Of the keyboard and `origin`, the keyboard had the focus last.
    focused: bool,
    /// Where each open companion window opened, fixed while it is open so it can be moved.
    companions: companions::Places,
    /// The pad the mouse holds in each `window::Pointer` window.
    mouse: [Option<recording::Pad>; 2],
    /// Present while playing.
    recorder: Option<Recorder>,
    /// The recording voice's metronome, present while the instruments play; its first click
    /// fixes the loop origin.
    click: Option<std::sync::Arc<crate::play::audition::click::Click>>,
    /// The app generation whose patches `sounds` hold.
    generation: u64,
    /// Keys sounded, recording or not, not yet in the song.
    pending: capture::Pending,
    /// Present while the Capture window is open.
    capture: Option<capture::Browser>,
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
    fn sound(&self) -> &Sound {
        &self.sounds[self.selected]
    }
    /// The keys of the playing container, while the instruments play.
    fn queue(&self) -> Option<&live::Queue> {
        self.click.as_ref().map(|_| &self.sound().live.queue)
    }
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

fn container_name(music: &cat_smf_editor::model::Music, id: u64, track: usize) -> String {
    music.tracks[track]
        .name()
        .unwrap_or_else(|| format!("Container {id}"))
}

fn open(a: &mut App, ctx: &egui::Context) -> Result<(), String> {
    let pane = super::patterns::workspace::pane(ctx);
    open_from(a, ctx, ctx.viewport_id(), pane)
}

/// Opens as Shift+R in `origin`, which is `pane` when that is a pattern window.
fn open_from(
    a: &mut App,
    ctx: &egui::Context,
    origin: egui::ViewportId,
    pane: Option<Pane>,
) -> Result<(), String> {
    if a.patterns.recording.is_open() {
        ctx.send_viewport_cmd_to(window::viewport(), egui::ViewportCommand::Focus);
        return Ok(());
    }
    if modal::is_open(a) {
        return Ok(());
    }
    let loaded = a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?;
    let c = Catalog::read(&loaded.music)?;
    let key = song_key(&loaded.music);
    let (mut settings, measures) = a.config.keyboard_settings.unwrap_or_else(|| {
        (
            Settings::new(key, setup::tempo(a.patterns.tempo.0.round() as i32)),
            4,
        )
    });
    if settings.key() != key {
        settings.set_key(key);
    }
    Recorder::new(loaded.music.division, settings)?;
    let looper = || {
        if a.config.phrase_loop {
            phrase::Looper::Waiting
        } else {
            phrase::Looper::Off
        }
    };
    let mut sounds = c
        .tracks
        .iter()
        .filter(|(_, t)| c.sources.contains(t))
        .map(|(&id, &track)| Sound {
            container: Some(id),
            track,
            patch: loaded.patches[track].clone(),
            name: container_name(&loaded.music, id, track),
            live: live::Live::default(),
            looper: looper(),
        })
        .collect::<Vec<_>>();
    sounds.sort_by_key(|s| s.track);
    let target = if let Some(pane) = pane {
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
            live: live::Live::default(),
            looper: looper(),
        });
    }
    let selected = sounds.iter().position(|s| s.container == target);
    let index = selected.unwrap_or(0);
    if let Some(v) = sounds[index]
        .container
        .and_then(|id| a.config.keyboard_voicings.get(&id))
    {
        settings.set_voicing(*v);
    }
    // No container is implied, so the settings open on the container row.
    let ambiguous = selected.is_none() && sounds.len() > 1;
    let division = loaded.music.division;
    crate::play::transport::stop(a);
    super::piano_roll::cancel_keyboard(a, ctx);
    let mut session = Session {
        origin,
        #[cfg(windows)]
        owner: super::analysis::owner::editor(),
        #[cfg(not(windows))]
        owner: None,
        owned: vec![],
        song: a.song_id,
        division,
        clock: Instant::now(),
        sounds,
        selected: index,
        settings,
        measures,
        item: setup::Item::Sound,
        settings_open: false,
        piano_open: true,
        raise: false,
        focused: true,
        companions: companions::Places::default(),
        mouse: [None; 2],
        recorder: None,
        click: None,
        generation: a.generation,
        pending: capture::Pending::new(),
        capture: None,
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
    resume::follow(a, ctx);
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
            match s.clicked.take() {
                Some(window::Button::Browse) => {
                    phrase::browse(a, ctx, s);
                    false
                }
                b => b.is_some_and(|b| play::button(a, s, b)),
            }
        });
        if let Some(at) = moved {
            companions::remember(a, companions::KEYBOARD, at);
        }
        if ctx.input_for(window::viewport(), |i| i.focused) {
            s.focused = true;
        } else if ctx.input_for(s.origin, |i| i.focused) {
            s.focused = false;
        }
        if !done {
            if let Err(error) = capture::flush(a, s, false) {
                s.error = Some(error);
            }
            let keyboard = moved.unwrap_or(s.position);
            companions::show(a, ctx, s, keyboard);
            capture::show(a, ctx, s, keyboard);
            companions::adopt(ctx, s);
        }
    }
    if done {
        // A closing that commits a take has put the keys in the song already.
        if a.song_id == s.song {
            if let Err(error) = capture::flush(a, s, true) {
                a.load_error = Some(error);
            }
            // A take reveals its pattern instead.
            capture::close(a, s);
        }
        ctx.send_viewport_cmd_to(s.origin, egui::ViewportCommand::Focus);
        // Only closing the app leaves the switch on for the next start.
        a.config.select_phrase_loop(false);
        shut(a, &mut state);
    }
    a.patterns.recording = state;
    ctx.request_repaint_after(std::time::Duration::from_millis(8));
}

fn shut(a: &mut App, state: &mut State) {
    if let Some(s) = state.session.take() {
        a.audition.stop_live();
        // Settings and the loop length survive closing and restarts; the key is reread at every open.
        a.config
            .select_keyboard_settings((*s.settings(), s.measures));
        setup::keep_voicing(a, &s);
    }
}

/// Closes the keyboard window as its own close button does, so the exit confirmation gets input.
/// `false` keeps it open with the commit error.
pub(crate) fn close_for_exit(a: &mut App) -> bool {
    let mut state = std::mem::take(&mut a.patterns.recording);
    let done = state.session.as_mut().is_none_or(|s| {
        // Closing from the editor moves the focus there first, so the last of the two counts.
        a.config.select_keyboard_focused(s.focused);
        play::close(a, s, s.clock.elapsed())
    });
    if done {
        shut(a, &mut state);
    }
    a.patterns.recording = state;
    done
}
