//! Shared app operations. Enumeration is read-only; bindings and execution live together.
use crate::app::App;
use eframe::egui::{self, Key, Modifiers};

mod basic;
mod menus;
mod navigation;
mod ownership;
mod search;
pub(crate) use search::SearchQuery;
#[cfg(test)]
mod keybind_tests;
#[cfg(test)]
mod tests;
pub(crate) use ownership::{owner, Screen};

type Execute = dyn Fn(&mut App, &egui::Context);

pub struct Command {
    pub id: String,
    pub screen: Screen,
    pub name: String,
    pub keys: Vec<(Key, Modifiers)>,
    pub repeat: bool,
    target: Option<Target>,
    run: Box<Execute>,
}

impl Command {
    pub fn new(
        screen: Screen,
        id: impl Into<String>,
        name: impl Into<String>,
        keys: &[(Key, Modifiers)],
        run: impl Fn(&mut App, &egui::Context) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            screen,
            name: name.into(),
            keys: keys.into(),
            repeat: false,
            target: None,
            run: Box::new(run),
        }
    }
    pub fn plain(
        screen: Screen,
        id: impl Into<String>,
        name: impl Into<String>,
        keys: &[Key],
        run: impl Fn(&mut App, &egui::Context) + 'static,
    ) -> Self {
        Self::new(
            screen,
            id,
            name,
            &keys
                .iter()
                .map(|k| (*k, Modifiers::NONE))
                .collect::<Vec<_>>(),
            run,
        )
    }
    pub fn repeating(mut self) -> Self {
        self.repeat = true;
        self
    }
    pub fn key_label(&self) -> String {
        self.keys
            .iter()
            .map(|(k, m)| {
                format!(
                    "{}{}{}{}",
                    if m.ctrl { "Ctrl+" } else { "" },
                    if m.alt { "Alt+" } else { "" },
                    if m.shift { "Shift+" } else { "" },
                    k.name()
                )
            })
            .collect::<Vec<_>>()
            .join(" / ")
    }
    pub fn label(&self) -> String {
        format!("{}: {}", self.screen.label(), self.name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    song: u64,
    generation: u64,
    track: usize,
    event: Option<usize>,
    layer: (Option<u64>, usize),
    effect: (Option<super::effects::Target>, usize),
    mixer: (bool, usize, bool),
    pane: usize,
}
impl Target {
    pub(crate) fn of(a: &App) -> Self {
        Self {
            song: a.song_id,
            generation: a.generation,
            track: a.track,
            event: a.selected_event,
            layer: a.layer_editor.command_target(),
            effect: a.effect_editor.command_target(),
            mixer: (
                a.playback.mixer_focused(),
                a.playback.mixer_track,
                a.playback.mixer_master,
            ),
            pane: a.track_view.active,
        }
    }
}

/// Compare physical modifiers: Windows also sets command when Ctrl is held.
pub fn matches(actual: Modifiers, expected: Modifiers) -> bool {
    actual.ctrl == expected.ctrl
        && actual.alt == expected.alt
        && actual.shift == expected.shift
        && actual.mac_cmd == expected.mac_cmd
}

pub struct Spec<T> {
    pub id: &'static str,
    pub name: &'static str,
    pub keys: Vec<(Key, Modifiers)>,
    pub action: T,
}
impl<T> Spec<T> {
    pub fn plain(id: &'static str, name: &'static str, keys: &[Key], action: T) -> Self {
        Self {
            id,
            name,
            keys: keys.iter().map(|k| (*k, Modifiers::NONE)).collect(),
            action,
        }
    }
}

pub fn definitions(app: &App, screen: Screen) -> Vec<Command> {
    let mut result = Vec::new();
    basic::collect(app, screen, &mut result);
    menus::collect(app, screen, &mut result);
    navigation::collect(app, screen, &mut result);
    super::layers::commands::collect(app, screen, &mut result);
    super::effects::commands::collect(app, screen, &mut result);
    super::recent::commands::collect(app, screen, &mut result);
    super::event_selection::commands::collect(app, screen, &mut result);
    super::mml_input::commands::collect(app, screen, &mut result);
    super::voicevox::collect(app, screen, &mut result);
    for c in &mut result {
        c.target = Some(Target::of(app));
    }
    result
}

pub fn candidates(app: &App, ctx: &egui::Context) -> Vec<Command> {
    let current = owner(app, ctx);
    let mut result = contextual_definitions(app, ctx, current);
    let screens: &[Screen] = match current {
        Screen::Main => &[
            Screen::Settings,
            Screen::Program,
            Screen::Edit,
            Screen::Go,
            Screen::Tracks,
            Screen::Split,
            Screen::Play,
            Screen::Mixer,
            Screen::Layers,
            Screen::Effects,
            Screen::Recent,
            Screen::Timeline,
        ],
        Screen::Timeline => &[Screen::Settings, Screen::Program, Screen::Play],
        Screen::Mixer => &[Screen::Effects],
        Screen::Settings => &[Screen::Program],
        Screen::Go => &[Screen::Tracks],
        _ => &[],
    };
    for &screen in screens {
        if ownership::reachable(app, screen) {
            result.extend(contextual_definitions(app, ctx, screen));
        }
    }
    result
}

pub fn execute(app: &mut App, ctx: &egui::Context, command: &Command) -> bool {
    if command.target.as_ref() != Some(&Target::of(app)) {
        return false;
    }
    // Rebuild from current domain conditions. Never use the saved closure after invalidation.
    let Some(fresh) = candidates(app, ctx)
        .into_iter()
        .find(|c| c.id == command.id && c.screen == command.screen)
    else {
        return false;
    };
    if owner(app, ctx) != fresh.screen {
        ownership::enter(app, ctx, fresh.screen);
    }
    if let Some(valid) = definitions(app, fresh.screen)
        .into_iter()
        .find(|c| c.id == fresh.id)
    {
        (valid.run)(app, ctx);
        true
    } else {
        false
    }
}

/// One owner per input batch, including its closing frame. No synthetic key events.
pub fn route(app: &mut App, ctx: &egui::Context) {
    if app.palette.is_open() {
        return;
    }
    let token = super::palette::frame_token(ctx);
    let guard = egui::Id::new(("command-route", ctx.viewport_id()));
    if ctx.data(|d| d.get_temp::<(egui::ViewportId, u64)>(guard)) == Some(token) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(guard, token));
    let mut screen = owner(app, ctx);
    if screen == Screen::Lyrics {
        crate::view::voicevox::lyrics::keep_ime_enter(app, ctx);
    }
    if screen == Screen::Singer {
        crate::view::voicevox::singer::keep_ime(app, ctx);
    }
    let events = ctx.input(|i| i.events.clone());
    let question_repeat = events
        .iter()
        .any(|e| matches!(e, egui::Event::Key { repeat: true, .. }));
    for event in events {
        let defs = contextual_definitions(app, ctx, screen);
        let command = match event {
            egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                repeat,
                ..
            } => defs.into_iter().find(|c| {
                (!repeat || c.repeat)
                    && c.keys
                        .iter()
                        .any(|(k, m)| *k == key && matches(modifiers, *m))
            }),
            egui::Event::Text(ref t) if t == "?" && !question_repeat => {
                defs.into_iter().find(|c| c.id == "help")
            }
            _ => None,
        };
        if let Some(command) = command {
            (command.run)(app, ctx);
            ctx.request_repaint();
            ctx.input_mut(|i| i.events.retain(|e| e != &event));
            let next = owner(app, ctx);
            if next != screen {
                if matches!(
                    next,
                    Screen::Go | Screen::Tracks | Screen::Split | Screen::Play
                ) && matches!(
                    screen,
                    Screen::Main | Screen::Go | Screen::Tracks | Screen::Timeline
                ) {
                    screen = next;
                } else {
                    super::comma_menu::swallow(ctx);
                    break;
                }
            }
        }
    }
    if screen.owns_keys() {
        super::comma_menu::swallow(ctx);
    }
}

#[cfg(test)]
pub(crate) fn run_direct(a: &mut App, c: &egui::Context, command: &Command) {
    (command.run)(a, c);
}

/// Stop runs before transport advance; other commands run after it.
pub(crate) fn route_stop(app: &mut App, ctx: &egui::Context) {
    if app.palette.is_open() {
        return;
    }
    let screen = owner(app, ctx);
    let Some(c) = definitions(app, screen)
        .into_iter()
        .find(|c| c.id == "stop")
    else {
        return;
    };
    let pressed=ctx.input(|i|i.events.iter().any(|e|matches!(e,egui::Event::Key{key:Key::Space,pressed:true,modifiers,..} if matches(*modifiers,Modifiers::SHIFT))));
    if pressed {
        (c.run)(app, ctx);
        ctx.input_mut(|i|i.events.retain(|e|!matches!(e,egui::Event::Key{key:Key::Space,modifiers,..} if matches(*modifiers,Modifiers::SHIFT))));
    }
}

fn contextual_definitions(app: &App, ctx: &egui::Context, screen: Screen) -> Vec<Command> {
    let mut definitions = definitions(app, screen);
    if ctx.input(|i| i.pointer.any_down() || i.pointer.any_released()) {
        definitions.retain(|c| c.id != "undo" && c.id != "edit-undo");
    }
    definitions
}
