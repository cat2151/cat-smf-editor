//! The keyboard windows sit above the editor beside the windows that stay usable.
use super::{
    commands::{self, Command, Screen},
    dock::{focus, locate, ViewKind},
    modal,
    patterns::workspace::Pane,
    presentation::{input_for, send_to},
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
mod choices;
mod companions;
mod following;
mod pads;
mod panel;
mod phrase;
mod piano;
mod play;
mod resume;
mod setup;
mod sweep;
mod take;
mod ticking;
pub(crate) use following::pattern as follow_pattern;
#[cfg(test)]
mod tests;
pub(crate) mod window;
pub(crate) use capture::advance_app as advance_capture;
#[cfg(test)]
pub(crate) use capture::viewport as capture_viewport;
pub(crate) use companions::reset as reset_windows;
pub(crate) use companions::{piano_viewport, settings_viewport};
pub(crate) use panel::LOOPING;
pub(crate) use resume::begin as resume;
pub(crate) use resume::{close_for_exit, restore};
use take::{finish, reveal};

/// Ticks into pattern `id` that a Phrase loop repeating it plays now.
pub(crate) fn phrase_tick(a: &App, id: u64) -> Option<u64> {
    let s = a.patterns.recording.session.as_ref()?;
    phrase::tick(s, id, a.loaded.as_ref()?.music.division)
}

#[derive(Default)]
pub struct State {
    session: Option<Session>,
    follow: Option<u64>,
    /// The keyboard reopens at startup once the Song window is up.
    resuming: bool,
    /// The keyboard opening at startup leaves the focus where the app closed with it.
    starting: bool,
    /// The song whose keyboard could not open by itself; it is not retried every frame.
    failed: Option<u64>,
}
impl State {
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }
    /// Minimized by its own button; the Song window offers it back.
    pub fn minimized(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.minimized)
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
    /// Full-keyboard kit page, counted upward from the bottom of the editor.
    kit_bank: usize,
    /// Runtime chromatic range, independent of the full-keyboard page.
    kit_base: u8,
}
struct Session {
    /// The window the keyboard opened from; it gets the focus back.
    origin: egui::ViewportId,
    /// The view the keyboard opened from; it gets the takes and, in a pane, the focus back.
    from: ViewKind,
    /// The window drawing the keyboard: its own, or the one holding its pane.
    home: egui::ViewportId,
    /// Set while the keyboard's pane draws this frame: whether it closes the keyboard.
    drawn: Option<bool>,
    /// The editor window that owns the keyboard windows, keeping them above it.
    owner: Option<isize>,
    /// Titles of the open windows `owner` already owns.
    owned: Vec<&'static str>,
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
    /// The piano sits in a pane of `layout.toml`, so its own window stays shut.
    piano_placed: bool,
    /// The keyboard came to the front, so its companions follow it there.
    raise: bool,
    /// Of the keyboard and `origin`, the keyboard had the focus last.
    focused: bool,
    minimized: bool,
    /// Where each open companion window opened, fixed while it is open so it can be moved.
    companions: companions::Places,
    /// The pad the mouse holds in each `window::Pointer` window.
    mouse: [Option<recording::Pad>; 2],
    /// Present while playing.
    recorder: Option<Recorder>,
    /// Session time the CC1 sweep started, while the keys are recorded.
    sweep: Option<std::time::Duration>,
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
    new_sound: Option<usize>,
    kit_creation: Option<choices::KitCreation>,
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

#[path = "recording/entry_commands.rs"]
mod entry_commands;
pub(crate) use entry_commands::collect;

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

mod opening;
use opening::{open, open_from};
mod kit;

/// The keyboard or one of its companion windows. While they draw, the session is out of `App`.
pub(crate) fn owns(viewport: egui::ViewportId) -> bool {
    [
        window::viewport(),
        settings_viewport(),
        piano_viewport(),
        capture::viewport(),
    ]
    .contains(&viewport)
}

/// Draws the keyboard as a pane, empty while no session is open. It reads its keys only while
/// it holds the focus; `show` finishes its frame.
pub(crate) fn pane(a: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let active = locate::view_at(a, &ctx) == Some(ViewKind::Keyboard);
    let mut state = std::mem::take(&mut a.patterns.recording);
    if let Some(s) = state.session.as_mut() {
        if active {
            super::help::observe_recording(a, &ctx);
        }
        let done = (s.first_read(&ctx) && play::process(a, &ctx, s, active)) || {
            window::pane(ui, s);
            clicked(a, &ctx, s)
        };
        s.drawn = Some(s.drawn.unwrap_or(false) || done);
    }
    a.patterns.recording = state;
}

/// Draws the piano as a pane, empty while no session is open.
pub(crate) fn piano_pane(a: &mut App, ui: &mut egui::Ui) {
    if let Some(s) = a.patterns.recording.session.as_mut() {
        piano::pane(ui, s);
    }
}

/// Handles the button clicked while drawing; `true` closes the keyboard.
fn clicked(a: &mut App, ctx: &egui::Context, s: &mut Session) -> bool {
    match s.clicked.take() {
        Some(window::Button::Browse) => {
            phrase::browse(a, ctx, s);
            false
        }
        Some(window::Button::Effects) => {
            if phrase::looping(s) && s.sound().container.is_some() {
                super::effects::entrances::bound(a, s.sound().track, ViewKind::Keyboard, s.home);
            }
            false
        }
        b => b.is_some_and(|b| play::button(a, s, b)),
    }
}

/// Gives the keys to the keyboard: its pane takes the focus, or its window comes back.
fn front(a: &mut App, ctx: &egui::Context) {
    if focus::placed(a, ViewKind::Keyboard) {
        focus::set(a, ViewKind::Keyboard);
        send_to(
            ctx,
            locate::viewport_of(a, ViewKind::Keyboard),
            egui::ViewportCommand::Focus,
        );
    } else {
        restore(ctx);
    }
}

/// Gives the keys back to `from`, the view the keyboard opened from, in window `origin`.
fn hand_back(a: &mut App, ctx: &egui::Context, from: ViewKind, origin: egui::ViewportId) {
    if focus::placed(a, ViewKind::Keyboard) {
        focus::set(a, from);
    }
    send_to(ctx, origin, egui::ViewportCommand::Focus);
}

/// Shows the keyboard windows; the other windows keep their own input.
pub(crate) fn show(a: &mut App, ctx: &egui::Context) {
    resume::follow(a, ctx);
    following::apply(a);
    let placed = focus::placed(a, ViewKind::Keyboard);
    let pane_focused = focus::focused(a) == ViewKind::Keyboard;
    let home = locate::viewport_of(a, ViewKind::Keyboard);
    let mut state = std::mem::take(&mut a.patterns.recording);
    let Some(s) = state.session.as_mut() else {
        a.patterns.recording = state;
        return;
    };
    s.home = home;
    s.piano_placed = focus::placed(a, ViewKind::Piano);
    let mut moved = None;
    let song = a.song_id;
    let done = if placed {
        s.minimized = false;
        // A frame whose pane was not drawn, such as under a menu, still plays the phrases.
        s.drawn.take().unwrap_or_else(|| {
            play::suspend(s);
            play::advance(a, s)
        })
    } else {
        own_window(a, ctx, s, &mut moved)
    };
    if let Some(at) = moved {
        companions::remember(a, companions::KEYBOARD, at);
    }
    if input_for(ctx, home, |i| i.focused) == Some(true) && (!placed || pane_focused) {
        s.focused = true;
    } else if input_for(ctx, s.origin, |i| i.focused) == Some(true) {
        s.focused = false;
    }
    if !done {
        if let Err(error) = capture::flush(a, s, false) {
            s.error = Some(error);
        }
        let keyboard = moved.unwrap_or(s.position);
        companions::show(a, ctx, s, keyboard);
        capture::show(a, ctx, s, keyboard);
        companions::adopt(ctx, s, !placed);
    }
    if done {
        // A closing that commits a take has put the keys in the song already.
        if a.song_id == song {
            if let Err(error) = capture::flush(a, s, true) {
                a.load_error = Some(error);
            }
            // A take reveals its pattern instead.
            capture::close(a, s);
        }
        hand_back(a, ctx, s.from, s.origin);
        // Only closing the app leaves the switch on for the next start.
        a.config.select_phrase_loop(false);
        resume::shut(a, &mut state);
    }
    let ticking = state.session.as_ref().is_some_and(Session::ticking);
    a.patterns.recording = state;
    if ticking {
        ctx.request_repaint_after(std::time::Duration::from_millis(8));
    }
}

/// The keyboard's own window; `true` closes the keyboard.
fn own_window(
    a: &mut App,
    ctx: &egui::Context,
    s: &mut Session,
    moved: &mut Option<egui::Pos2>,
) -> bool {
    let visible = super::presentation::Native::Keyboard.visible(a, ctx, window::viewport());
    let suspended_done = if !visible {
        play::suspend(s);
        play::advance(a, s)
    } else {
        false
    };
    let builder = egui::ViewportBuilder::default()
        .with_title(window::TITLE)
        .with_position(s.position)
        .with_inner_size(window::SIZE);
    let _window_span = crate::frame_trace::span("keyboard");
    let done = super::presentation::Native::Keyboard.show(
        a,
        ctx,
        window::viewport(),
        builder,
        |a, ctx, class| {
            let _span = crate::frame_trace::span("keyboard/ui");
            let native = class != egui::ViewportClass::Embedded;
            if native {
                *moved = ctx.input(|i| i.viewport().outer_rect).map(|r| r.min);
                s.minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
                if crate::view::alt_f4::closes(ctx) {
                    if play::kept(a) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    }
                    return play::end(a, s, s.clock.elapsed());
                }
            }
            super::help::observe_recording(a, ctx);
            // Embedded, this shares the root input whose modal already ran; a second one eats the palette's keys.
            let input = native.then(|| modal::begin(a, ctx)).flatten();
            let done = (s.first_read(ctx) && play::process(a, ctx, s, true)) || {
                window::show(ctx, class, s);
                clicked(a, ctx, s)
            };
            if native {
                modal::end(a, ctx, input);
            }
            done
        },
    );
    suspended_done || done.unwrap_or(false)
}
