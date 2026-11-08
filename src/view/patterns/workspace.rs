//! The arrangement windows share a lifetime, but never a selected cell.
use super::*;
use crate::view::dock::{locate, ViewKind};
use cat_smf_editor::message;
use cat_smf_editor::score::{
    pattern::Catalog,
    scene::{
        self,
        arrangement::{self as model, PatternOrScene},
    },
};
use eframe::egui::{self, Key, Modifiers};
mod commands;
mod editing;
pub use editing::{click_body, edit, edit_choice};
mod deletion;
mod dialog;
mod minimap;
mod panel;
mod rows;
mod selection;
mod timeline;
pub use commands::collect;
pub use rows::{lane, lanes, materialize, rows};
pub use selection::Selection;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Scene,
    Song,
}
impl Pane {
    pub fn viewport(self) -> egui::ViewportId {
        egui::ViewportId::from_hash_of(match self {
            Self::Scene => "pattern-scene",
            Self::Song => "pattern-song",
        })
    }
    pub fn screen(self) -> Screen {
        match self {
            Self::Scene => Screen::Scene,
            Self::Song => Screen::Song,
        }
    }
    pub(crate) fn view(self) -> ViewKind {
        match self {
            Self::Scene => ViewKind::Scene,
            Self::Song => ViewKind::Song,
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Scene => "Scene",
            Self::Song => "Pattern / Song",
        }
    }
}
/// The pattern view `ctx` shows.
pub fn pane(a: &App, ctx: &egui::Context) -> Option<Pane> {
    match crate::view::dock::locate::view_at(a, ctx)? {
        ViewKind::Scene => Some(Pane::Scene),
        ViewKind::Song => Some(Pane::Song),
        _ => None,
    }
}
#[derive(Default)]
pub struct State {
    timeline: timeline::State,
    scene_timeline: timeline::State,
    pub scene: Option<u64>,
    pub scene_cell: Selection,
    pub song_cell: Selection,
    pub dialog: Option<dialog::Dialog>,
    pub return_to: Option<Pane>,
    pub error: Option<String>,
    /// The Scene window is shown only on request from the Song window.
    pub scene_window: bool,
}
impl State {
    pub fn input_open(&self) -> bool {
        self.dialog.is_some()
    }
    pub fn selection(&self, p: Pane) -> &Selection {
        match p {
            Pane::Scene => &self.scene_cell,
            Pane::Song => &self.song_cell,
        }
    }
    pub fn selection_mut(&mut self, p: Pane) -> &mut Selection {
        match p {
            Pane::Scene => &mut self.scene_cell,
            Pane::Song => &mut self.song_cell,
        }
    }
}
/// The dialog belongs to the view `ctx` shows.
pub fn input_owner(a: &App, ctx: &egui::Context) -> bool {
    a.patterns
        .workspace
        .dialog
        .as_ref()
        .is_some_and(|d| locate::view_at(a, ctx) == Some(d.view))
}
/// The dialog is drawn in `ctx`.
fn dialog_shown(a: &App, ctx: &egui::Context) -> bool {
    a.patterns
        .workspace
        .dialog
        .as_ref()
        .is_some_and(|d| locate::shows(a, ctx, d.view))
}
pub fn query_id(p: Pane) -> egui::Id {
    egui::Id::new((p.title(), "part-query"))
}
pub fn focused_query(ctx: &egui::Context) -> Option<Pane> {
    [Pane::Scene, Pane::Song].into_iter().find(|p| {
        let id = query_id(*p);
        // egui clears focus for Escape at begin_pass, before command routing.
        // The closing key still belongs to the query that had focus.
        ctx.memory(|m| m.focused() == Some(id))
            || ctx.input(|i| i.key_pressed(Key::Escape))
                && ctx.memory(|m| m.had_focus_last_frame(id))
    })
}
/// Only the view that opened the dialog keeps its input.
pub fn suspend_background(a: &App, ctx: &egui::Context) {
    if a.patterns.workspace.dialog.is_some() && !input_owner(a, ctx) {
        ctx.input_mut(|i| {
            i.events.clear();
            i.raw.events.clear();
            i.keys_down.clear();
            i.pointer = Default::default();
            i.modifiers = Default::default();
            i.raw_scroll_delta = egui::Vec2::ZERO;
            i.smooth_scroll_delta = egui::Vec2::ZERO;
        });
    }
}
pub fn scene_id(a: &App, p: Pane) -> Option<u64> {
    if p == Pane::Scene {
        a.patterns.workspace.scene
    } else {
        None
    }
}
/// The keyboard's Capture is not offered; it reaches a song only as a pattern made from it.
/// The keyboard's Loop leads its container.
pub fn choices(a: &App, p: Pane) -> Vec<(PatternOrScene, String)> {
    use cat_smf_editor::score::pattern::Kind;
    let Some((catalog, collection)) = a.loaded.as_ref().and_then(|l| l.arrangement.as_ref()) else {
        return vec![];
    };
    let mut patterns = catalog
        .patterns
        .values()
        .filter(|p| p.kind != Kind::Capture)
        .collect::<Vec<_>>();
    patterns.sort_by_key(|p| (p.track, p.kind != Kind::Loop, p.tick));
    let mut out = patterns
        .into_iter()
        .map(|p| (PatternOrScene::Pattern(p.id), p.name.clone()))
        .collect::<Vec<_>>();
    if p == Pane::Song {
        out.extend(collection.scenes.iter().map(|s| {
            (
                PatternOrScene::Scene(s.id),
                format!("{} · {} measures", s.name, s.meters.len()),
            )
        }));
    }
    out
}
/// Pattern work happens in Notes.
pub fn enable(a: &mut App) {
    if !a.note_mode {
        mode::toggle(a);
    }
}
pub fn focus(a: &mut App, ctx: &egui::Context, p: Pane) {
    ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
    if crate::view::dock::focus::placed(a, p.view()) {
        crate::view::dock::focus::set(a, p.view());
    } else {
        a.patterns.workspace.selection_mut(p).focus = true;
    }
    crate::view::presentation::send_to(
        ctx,
        locate::viewport_of(a, p.view()),
        egui::ViewportCommand::Focus,
    );
    ctx.request_repaint();
}
/// `p` has something to show: the song has containers, and the Scene was asked for.
pub(crate) fn shown(a: &App, p: Pane) -> bool {
    mode::song_window(a) && (p == Pane::Song || a.patterns.workspace.scene_window)
}
/// Draws `p` as a pane, empty while it has nothing to show.
pub(crate) fn leaf(a: &mut App, ui: &mut egui::Ui, p: Pane) {
    if shown(a, p) {
        panel::show(a, ui, p);
    } else {
        suspend(a, p);
    }
}
/// A frame in which `p` is not drawn ends its timeline gestures.
pub(crate) fn suspend(a: &mut App, p: Pane) {
    match p {
        Pane::Song => a.patterns.workspace.timeline.suspend(),
        Pane::Scene => a.patterns.workspace.scene_timeline.suspend(),
    }
}
pub fn reset_windows(a: &mut App) {
    a.patterns.workspace.timeline.suspend();
    a.patterns.workspace.scene_timeline.suspend();
    for p in [Pane::Scene, Pane::Song] {
        let s = a.patterns.workspace.selection_mut(p);
        s.owned = false;
        s.opened = None;
        s.focus = false;
        s.composing = false;
    }
}
pub fn open_scene(a: &mut App, id: u64) -> Result<(), String> {
    let c = scene::read(&a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?;
    if !c.scenes.iter().any(|s| s.id == id) {
        return Err(message::MISSING_SCENE.into());
    }
    a.patterns.workspace.scene = Some(id);
    a.patterns.timing.scene = None;
    a.patterns.workspace.scene_cell.row = 0;
    a.patterns.workspace.scene_cell.measure = 1;
    a.patterns.workspace.scene_cell.focus = true;
    a.patterns.workspace.scene_window = true;
    enable(a);
    Ok(())
}
pub fn place(a: &mut App, p: Pane, part: Option<PatternOrScene>) -> Result<(), String> {
    let id = scene_id(a, p);
    if p == Pane::Scene && id.is_none() {
        return Err("先にsceneを作ってください".into());
    }
    let s = a.patterns.workspace.selection(p);
    let (row, measure) = (s.row, s.measure);
    if part.is_none() {
        let Some(lane) = lane(a, p) else {
            return Ok(());
        };
        let collection = scene::read(&a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?;
        let measure =
            model::covering(&collection, id, lane, measure).map_or(measure, |(_, start)| start);
        return mutate(a, |m| model::place(m, id, lane, measure, None));
    }
    mutate(a, |m| {
        let lane = materialize(m, id, row)?;
        let part = match part {
            Some(PatternOrScene::Pattern(p)) => Some(PatternOrScene::Pattern(
                cat_smf_editor::score::pattern::operations::placeable(m, p)?,
            )),
            part => part,
        };
        model::place(m, id, lane, measure, part)
    })
}
pub fn mutate(
    a: &mut App,
    edit: impl FnOnce(&mut cat_smf_editor::model::Music) -> Result<(), String>,
) -> Result<(), String> {
    let mut context = a.patterns.editing.clone();
    let tick = crate::play::playback::location(a).0;
    a.edit_tracks(edit)?;
    if let Some(c) = &mut context {
        if c.rebind(&a.loaded.as_ref().unwrap().music).is_err() {
            context = None;
        } else if c.standalone {
            a.track = c.track;
        }
    }
    a.patterns.editing = context.filter(|c| {
        a.loaded
            .as_ref()
            .is_some_and(|l| c.project(&l.music).is_ok())
    });
    a.patterns.edit_song = a.song_id;
    refresh(a);
    crate::cursor::moves::at_tick(a, tick);
    super::shown::ensure(a);
    Ok(())
}
pub fn place_selected(a: &mut App, p: Pane) {
    let part = a.patterns.workspace.selection(p).choice;
    let r = part
        .ok_or(message::SELECT_A_PATTERN_OR_SCENE.into())
        .and_then(|part| place(a, p, Some(part)));
    result(a, r);
}
pub fn play(a: &mut App, p: Pane) {
    if p == Pane::Scene && a.patterns.workspace.scene.is_none() {
        return;
    }
    crate::play::arrangement::start(a, scene_id(a, p));
}
fn result(a: &mut App, r: Result<(), String>) {
    a.patterns.workspace.error = r.err();
}
pub fn current(a: &App, p: Pane) -> Option<PatternOrScene> {
    let c = scene::read(&a.loaded.as_ref()?.music).ok()?;
    let s = a.patterns.workspace.selection(p);
    model::covering(&c, scene_id(a, p), lane(a, p)?, s.measure).map(|(part, _)| part)
}
pub fn open_dialog(a: &mut App, ctx: &egui::Context, purpose: dialog::Purpose, p: Pane) {
    enable(a);
    a.patterns.workspace.dialog = Some(dialog::Dialog::new(a, ctx, purpose, p));
}
/// Draws the dialog if it belongs to a view of the pane window `ctx`.
pub fn host_dialog(a: &mut App, ctx: &egui::Context) {
    if dialog_shown(a, ctx) {
        dialog::show(a, ctx);
    }
}
pub fn show(a: &mut App, ctx: &egui::Context) {
    if !mode::song_window(a) {
        reset_windows(a);
        return;
    }
    for p in [Pane::Scene, Pane::Song] {
        if !mode::song_window(a) {
            break;
        }
        if crate::view::dock::focus::placed(a, p.view()) {
            // Taking the pane out of the layout opens a new OS window, which needs its owner again.
            a.patterns.workspace.selection_mut(p).owned = false;
            continue;
        }
        let requested = if let Some(d) = &a.patterns.workspace.dialog {
            locate::viewport_of(a, d.view) == p.viewport()
        } else {
            shown(a, p)
                && !(a.palette.owns_root() || a.command_input.owns_root())
                && !a.patterns.confirming_creation()
        };
        let visible =
            requested && crate::view::presentation::Native::Workspace.visible(a, ctx, p.viewport());
        if !visible {
            suspend(a, p);
        }
        let opened = super::windows::rect(a, p.title());
        let opened = *a
            .patterns
            .workspace
            .selection_mut(p)
            .opened
            .get_or_insert(opened);
        let (window, ui) = match p {
            Pane::Song => ("song", "song/ui"),
            Pane::Scene => ("scene", "scene/ui"),
        };
        let window_span = crate::frame_trace::span(window);
        crate::view::presentation::Native::Workspace.show(
            a,
            ctx,
            p.viewport(),
            super::windows::builder(opened)
                .with_title(p.title())
                .with_active(false)
                .with_min_inner_size([500.0, 360.0])
                .with_visible(requested),
            |a, ctx, class| {
                let _span = crate::frame_trace::span(ui);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(visible));
                if !visible {
                    return;
                }
                if a.patterns.workspace.selection(p).focus {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    a.patterns.workspace.selection_mut(p).focus = false;
                }
                if class != egui::ViewportClass::Embedded {
                    super::windows::remember(a, ctx, p.title());
                }
                let input = crate::view::modal::begin(a, ctx);
                crate::view::commands::route(a, ctx);
                if class == egui::ViewportClass::Embedded {
                    egui::Window::new(p.title())
                        .id(egui::Id::new((p.title(), "embedded")))
                        .default_pos([460.0, if p == Pane::Scene { 40.0 } else { 520.0 }])
                        .default_size([760.0, 400.0])
                        .show(ctx, |ui| panel::show(a, ui, p));
                } else {
                    egui::CentralPanel::default().show(ctx, |ui| panel::show(a, ui, p));
                }
                if dialog_shown(a, ctx) {
                    dialog::show(a, ctx);
                }
                crate::play::playback::show(a, ctx);
                crate::view::modal::end(a, ctx, input);
                // The song's patterns keep this window, so its × quits the app.
                if crate::view::alt_f4::closes(ctx) {
                    a.request_close(ctx);
                }
                if ctx.input(|i| i.viewport().close_requested()) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
            },
        );
        drop(window_span);
        // Restored at startup, this window is under the editor until it is owned.
        #[cfg(windows)]
        if !a.patterns.workspace.selection(p).owned {
            a.patterns.workspace.selection_mut(p).owned =
                crate::view::analysis::owner::adopt(p.title());
        }
    }
}
