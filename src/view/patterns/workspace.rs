//! The arrangement windows share a lifetime, but never a selected cell.
use super::*;
use cat_smf_editor::score::{
    pattern::Catalog,
    scene::{
        self,
        arrangement::{self as model, PatternOrScene},
    },
};
use eframe::egui::{self, Key, Modifiers};
mod commands;
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
    pub fn title(self) -> &'static str {
        match self {
            Self::Scene => "Scene",
            Self::Song => "Pattern / Song",
        }
    }
}
pub fn pane(ctx: &egui::Context) -> Option<Pane> {
    [Pane::Scene, Pane::Song]
        .into_iter()
        .find(|p| p.viewport() == ctx.viewport_id())
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
pub fn input_owner(a: &App, ctx: &egui::Context) -> bool {
    a.patterns
        .workspace
        .dialog
        .as_ref()
        .is_some_and(|d| d.viewport == ctx.viewport_id())
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
pub fn suspend_background(a: &App, ctx: &egui::Context) {
    if a.patterns
        .workspace
        .dialog
        .as_ref()
        .is_some_and(|d| d.viewport != ctx.viewport_id())
    {
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
pub fn choices(a: &App, p: Pane) -> Vec<(PatternOrScene, String)> {
    let Some(l) = &a.loaded else { return vec![] };
    let Ok(c) = Catalog::read(&l.music) else {
        return vec![];
    };
    let mut patterns = c.patterns.values().collect::<Vec<_>>();
    patterns.sort_by_key(|p| (p.track, p.tick));
    let mut out = patterns
        .into_iter()
        .map(|p| (PatternOrScene::Pattern(p.id), p.name.clone()))
        .collect::<Vec<_>>();
    if p == Pane::Song
        && let Ok(c) = scene::read(&l.music)
    {
        out.extend(c.scenes.iter().map(|s| {
            (
                PatternOrScene::Scene(s.id),
                format!("{} · {} measures", s.name, s.meters.len()),
            )
        }));
    }
    out
}
pub fn enable(a: &mut App) {
    if !a.note_mode {
        mode::toggle(a);
    }
    a.patterns.workspace_open = true;
}
pub fn focus(a: &mut App, ctx: &egui::Context, p: Pane) {
    ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
    enable(a);
    a.patterns.workspace.selection_mut(p).focus = true;
    ctx.send_viewport_cmd_to(p.viewport(), egui::ViewportCommand::Focus);
    ctx.request_repaint();
}
pub fn reset_windows(a: &mut App) {
    a.patterns.workspace.timeline.suspend();
    a.patterns.workspace.scene_timeline.suspend();
    for p in [Pane::Scene, Pane::Song] {
        let s = a.patterns.workspace.selection_mut(p);
        s.owned = false;
        s.focus = false;
        s.composing = false;
    }
}
pub fn open_scene(a: &mut App, id: u64) -> Result<(), String> {
    let c = scene::read(&a.loaded.as_ref().ok_or("No SMF is loaded")?.music)?;
    if !c.scenes.iter().any(|s| s.id == id) {
        return Err("Missing scene".into());
    }
    a.patterns.workspace.scene = Some(id);
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
        return Err("Create a scene first".into());
    }
    let s = a.patterns.workspace.selection(p);
    let (row, measure) = (s.row, s.measure);
    if part.is_none() {
        let Some(lane) = lane(a, p) else {
            return Ok(());
        };
        let collection = scene::read(&a.loaded.as_ref().ok_or("No SMF is loaded")?.music)?;
        let measure =
            model::covering(&collection, id, lane, measure).map_or(measure, |(_, start)| start);
        return mutate(a, |m| model::place(m, id, lane, measure, None));
    }
    mutate(a, |m| {
        let lane = materialize(m, id, row)?;
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
    Ok(())
}
pub fn place_selected(a: &mut App, p: Pane) {
    let part = a.patterns.workspace.selection(p).choice;
    let r = part
        .ok_or("Select a pattern or scene".into())
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
pub fn edit(a: &mut App, ctx: &egui::Context, p: Pane) {
    match current(a, p) {
        Some(PatternOrScene::Pattern(id)) => {
            let r = show_body(a, p, id);
            if r.is_ok() {
                a.patterns.workspace.return_to = Some(p);
                presentation::focus_root(a, ctx);
            }
            result(a, r);
        }
        Some(PatternOrScene::Scene(id)) => {
            let r = open_scene(a, id);
            result(a, r);
            focus(a, ctx, Pane::Scene);
        }
        None => {}
    }
}
/// The chosen part opens independently of any placement; Escape returns to this pane.
/// A drum pattern opens in the drum editor and an empty one in MML input.
pub fn edit_choice(a: &mut App, ctx: &egui::Context, p: Pane) {
    match a.patterns.workspace.selection(p).choice {
        Some(PatternOrScene::Pattern(id)) => {
            let r = library::edit(a, id);
            if r.is_ok() {
                a.patterns.workspace.return_to = Some(p);
                presentation::focus_root(a, ctx);
                let l = a.loaded.as_ref().unwrap();
                if !crate::view::drums::is_open(a) && super::beacon::empty(&l.music, id) {
                    crate::view::drums::input(a, ctx);
                }
            }
            result(a, r);
        }
        Some(PatternOrScene::Scene(id)) => {
            let r = open_scene(a, id);
            result(a, r);
            focus(a, ctx, Pane::Scene);
        }
        None => result(a, Err("Select a pattern or scene".into())),
    }
}
/// Open the body editor, bound to the placement under the selected cell.
fn show_body(a: &mut App, p: Pane, id: u64) -> Result<(), String> {
    let lane = lane(a, p);
    let placement = a
        .loaded
        .as_ref()
        .and_then(|l| scene::read(&l.music).ok())
        .and_then(|c| {
            let selection = a.patterns.workspace.selection(p);
            let measure = model::covering(&c, scene_id(a, p), lane?, selection.measure)?.1;
            if p == Pane::Scene {
                c.scenes
                    .iter()
                    .find(|s| Some(s.id) == a.patterns.workspace.scene)?
                    .cells
                    .iter()
                    .find(|cell| Some(cell.lane) == lane && cell.measure + 1 == measure)
                    .map(|cell| cell.id)
            } else {
                c.song
                    .iter()
                    .find(|cell| Some(cell.lane) == lane && cell.measure == measure)
                    .map(|cell| cell.id)
            }
        });
    library::edit(a, id)?;
    if let Some(c) = &mut a.patterns.editing {
        c.placement = placement;
    }
    Ok(())
}
/// A click on a placed pattern opens its body editor and focus stays here;
/// a click on a placed scene opens it in the scene window, which takes focus.
pub fn click_body(a: &mut App, p: Pane) {
    let r = match current(a, p) {
        Some(PatternOrScene::Pattern(id)) => show_body(a, p, id),
        Some(PatternOrScene::Scene(id)) => open_scene(a, id),
        None => return,
    };
    result(a, r);
}
pub fn open_dialog(a: &mut App, ctx: &egui::Context, purpose: dialog::Purpose, p: Pane) {
    enable(a);
    a.patterns.workspace.dialog = Some(dialog::Dialog::new(a, ctx, purpose, p));
}
pub fn root_dialog(a: &mut App, ctx: &egui::Context) {
    if input_owner(a, ctx) {
        dialog::show(a, ctx);
    }
}
pub fn show(a: &mut App, ctx: &egui::Context) {
    if !mode::workspace_open(a) {
        reset_windows(a);
        return;
    }
    for p in [Pane::Scene, Pane::Song] {
        if !mode::workspace_open(a) {
            break;
        }
        let visible = if let Some(d) = &a.patterns.workspace.dialog {
            d.viewport == p.viewport()
        } else {
            (p == Pane::Song || a.patterns.workspace.scene_window)
                && !(a.palette.owns_root() || a.command_input.owns_root())
                && !a.patterns.confirming_creation()
                && crate::view::analysis::companion_visible(a, ctx)
        };
        if !visible {
            match p {
                Pane::Song => a.patterns.workspace.timeline.suspend(),
                Pane::Scene => a.patterns.workspace.scene_timeline.suspend(),
            }
        }
        ctx.show_viewport_immediate(
            p.viewport(),
            super::windows::builder(a, p.title())
                .with_title(p.title())
                .with_active(false)
                .with_min_inner_size([500.0, 360.0])
                .with_visible(visible),
            |ctx, class| {
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
                if input_owner(a, ctx) {
                    dialog::show(a, ctx);
                }
                crate::play::playback::show(a, ctx);
                crate::view::modal::end(a, ctx, input);
                if ctx.input(|i| i.viewport().close_requested()) {
                    crate::play::transport::stop(a);
                    mode::close_workspace(a);
                    presentation::focus_root(a, ctx);
                }
            },
        );
        #[cfg(windows)]
        if !a.patterns.workspace.selection(p).owned
            && ctx.input(|i| i.viewport().focused.unwrap_or(false))
        {
            a.patterns.workspace.selection_mut(p).owned =
                crate::view::analysis::owner::adopt(p.title());
        }
    }
}
