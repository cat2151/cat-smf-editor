//! Presentation follows command ownership across egui layers and native windows.
//! Every native viewport goes through `Native::show`; root floating editors use
//! `window`/`area` so the input owner is also the top interactive layer.
use super::commands::{self, Screen};
use crate::app::App;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Native {
    Analysis,
    Workspace,
    Keyboard,
    Help,
    /// A window of panes from `layout.toml`.
    Dock,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    Workspace,
    FocusedWorkspace,
    Operation,
}

impl Screen {
    /// Exhaustive on purpose: adding an input owner requires a presentation decision.
    fn presentation(self) -> Policy {
        match self {
            Self::Main
            | Self::Score
            | Self::Timeline
            | Self::Mixer
            | Self::Drums
            | Self::Scene
            | Self::Song
            | Self::Recording
            | Self::Analysis
            | Self::Help
            | Self::Selection => Policy::Workspace,
            Self::Overview => Policy::FocusedWorkspace,
            Self::Global
            | Self::EditStyle
            | Self::TrackRename
            | Self::Settings
            | Self::Program
            | Self::Edit
            | Self::Go
            | Self::Tracks
            | Self::Split
            | Self::Play
            | Self::Layers
            | Self::DrumInput
            | Self::DrumCreate
            | Self::LayerInput
            | Self::Effects
            | Self::SelectionMenu
            | Self::ChordTiming
            | Self::Mml
            | Self::Note
            | Self::Lyrics
            | Self::Trill
            | Self::Recent
            | Self::Confirm
            | Self::Backup
            | Self::Invalid
            | Self::Missing
            | Self::Waiting
            | Self::Browser
            | Self::Voicevox
            | Self::Singer
            | Self::MacroEditor
            | Self::ParameterPicker
            | Self::ParameterInput
            | Self::Patterns
            | Self::ArrangementInput => Policy::Operation,
        }
    }
    pub(crate) fn allows_companions(self) -> bool {
        self.presentation() == Policy::Workspace
    }
    fn floating_order(self) -> egui::Order {
        match self.presentation() {
            Policy::Workspace | Policy::FocusedWorkspace => egui::Order::Middle,
            Policy::Operation => egui::Order::Foreground,
        }
    }
}

fn priority(a: &App) -> Option<egui::ViewportId> {
    if let Some(id) = a.palette.viewport() {
        return Some(id);
    }
    if a.command_input.is_open() {
        return Some(egui::ViewportId::ROOT);
    }
    match commands::root_owner(a) {
        Screen::Play => Some(crate::view::dock::locate::viewport_of(a, a.playback.view)),
        Screen::ArrangementInput => Some(
            a.patterns
                .workspace
                .dialog
                .as_ref()
                .map_or(egui::ViewportId::ROOT, |d| {
                    crate::view::dock::locate::viewport_of(a, d.view)
                }),
        ),
        // The grid's own input and its track-creation dialog stay in the window holding the grid.
        Screen::DrumInput | Screen::DrumCreate => Some(crate::view::dock::locate::viewport_of(
            a,
            crate::view::dock::ViewKind::StepSequencer,
        )),
        // The curve's point input and its variation chooser stay in the window holding the curve.
        Screen::ParameterInput => Some(crate::view::dock::locate::viewport_of(
            a,
            crate::view::dock::ViewKind::ParameterCurve,
        )),
        // The chord timing buttons stay in the window holding the roll.
        Screen::ChordTiming => Some(crate::view::dock::locate::viewport_of(
            a,
            crate::view::dock::ViewKind::PianoRoll,
        )),
        // The overview keeps the window holding it, as the main window holds its floating one.
        Screen::Overview => Some(crate::view::dock::locate::viewport_of(
            a,
            crate::view::dock::ViewKind::TrackOverview,
        )),
        owner => (!owner.allows_companions()).then_some(egui::ViewportId::ROOT),
    }
}

/// Kept independently of native window lifetime: hiding never closes a session.
#[cfg(test)]
pub(crate) fn companions_visible(a: &App, ctx: &egui::Context) -> bool {
    priority(a).is_none() && ctx.memory(|m| m.top_modal_layer()).is_none()
}

#[derive(Clone, Default)]
struct NativeState {
    pass: u64,
    modal: Option<(egui::ViewportId, u64)>,
    popup: Option<(egui::ViewportId, u64)>,
    seen: Vec<(egui::ViewportId, Native, bool)>,
    previous: Vec<egui::ViewportId>,
    pending: Vec<(egui::ViewportId, egui::ViewportCommand)>,
    /// The last `Visible` each window got, and the pass it was sent in.
    sent: Vec<(egui::ViewportId, bool, u64)>,
}
fn native_key() -> egui::Id {
    egui::Id::new("presentation-native")
}
fn effective_priority(a: &App, ctx: &egui::Context) -> Option<egui::ViewportId> {
    priority(a).or_else(|| {
        let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
        ctx.data(|d| d.get_temp::<NativeState>(native_key()))
            .and_then(|s| s.modal)
            .filter(|(_, seen)| *seen + 1 >= pass)
            .map(|(id, _)| id)
    })
}

#[derive(Clone, Default)]
struct HelpPermission {
    identity: Option<(egui::ViewportId, Screen, bool)>,
    allowed: bool,
}

impl Native {
    pub(crate) fn visible(self, a: &App, ctx: &egui::Context, id: egui::ViewportId) -> bool {
        let Some(target) = effective_priority(a, ctx) else {
            ctx.data_mut(|d| d.remove::<HelpPermission>(egui::Id::new("presentation-help")));
            return true;
        };
        let identity = (target, commands::root_owner(a), a.palette.is_open());
        let help_allowed = ctx.data_mut(|d| {
            let state =
                d.get_temp_mut_or_default::<HelpPermission>(egui::Id::new("presentation-help"));
            if state.identity != Some(identity) {
                state.identity = Some(identity);
                state.allowed = false;
            }
            state.allowed |= a.help.raise;
            state.allowed
        });
        target == id || (self == Self::Help && help_allowed)
    }

    /// The callback of a hidden viewport is never run, even if the OS/backend
    /// still delivers queued pointer/key events. Visible is sent on every change,
    /// including restoration; a builder alone does not guarantee that transition.
    pub(crate) fn show<R>(
        self,
        a: &mut App,
        ctx: &egui::Context,
        id: egui::ViewportId,
        builder: egui::ViewportBuilder,
        mut body: impl FnMut(&mut App, &egui::Context, egui::ViewportClass) -> R,
    ) -> Option<R> {
        let requested = builder.visible.unwrap_or(true);
        let visible = requested && self.visible(a, ctx, id);
        let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<NativeState>(native_key())
                .begin(pass)
        });
        ctx.show_viewport_immediate(id, builder.with_visible(visible), |ctx, class| {
            crate::frame_trace::viewport(ctx);
            if class != egui::ViewportClass::Embedded {
                windows::send_visible(ctx, id, visible);
                ctx.data_mut(|d| {
                    d.get_temp_mut_or_default::<NativeState>(native_key())
                        .seen
                        .push((id, self, requested))
                });
            }
            if visible {
                ctx.data_mut(|d| {
                    let state = d.get_temp_mut_or_default::<NativeState>(native_key());
                    if state.modal.is_some_and(|(host, _)| host == id) {
                        state.modal = None;
                    }
                });
            }
            let result = visible.then(|| body(a, ctx, class));
            ctx.data_mut(|d| {
                let state = d.get_temp_mut_or_default::<NativeState>(native_key());
                if state
                    .popup
                    .is_some_and(|(host, seen)| host == id && seen < pass)
                {
                    state.popup = None;
                }
            });
            result
        })
    }
}

/// Register the actual modal host in its opening frame; egui's top_modal_layer
/// describes the preceding frame and cannot be used for visibility transitions.
pub(crate) fn popup_owns_input(ctx: &egui::Context) -> bool {
    let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
    let host = ctx.viewport_id();
    ctx.memory(|m| m.any_popup_open())
        || ctx
            .data(|d| d.get_temp::<NativeState>(native_key()))
            .and_then(|s| s.popup)
            .is_some_and(|(id, seen)| id == host && seen + 1 >= pass)
}

fn claim(ctx: &egui::Context, popup: bool) {
    if !ctx.embed_viewports() {
        let host = ctx.viewport_id();
        let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
        ctx.data_mut(|d| {
            let state = d.get_temp_mut_or_default::<NativeState>(native_key());
            state.modal = Some((host, pass));
            if popup {
                state.popup = Some((host, pass));
            }
        });
    }
}

pub(crate) fn modal(ctx: &egui::Context, id: egui::Id) -> egui::Modal {
    claim(ctx, false);
    egui::Modal::new(id)
}

mod popups;
pub(crate) mod windows;
pub(crate) use popups::{context_menu, menu_button, popup_below_widget};
pub(crate) use windows::{focus_after_restoration, input_for, send_to};

#[derive(Clone, Default)]
struct Layers {
    owner: Option<Screen>,
    registered: Vec<(Screen, egui::LayerId)>,
}
fn key(ctx: &egui::Context) -> egui::Id {
    egui::Id::new(("presentation-layers", ctx.viewport_id()))
}

pub(crate) fn prepare(a: &App, ctx: &egui::Context) {
    ctx.data_mut(|d| {
        let state = d.get_temp_mut_or_default::<NativeState>(native_key());
        if state
            .modal
            .is_some_and(|(host, _)| host == egui::ViewportId::ROOT)
        {
            state.modal = None;
        }
    });
    layers(a, ctx);
}

/// Starts the frame's floating windows of `ctx`'s window, owned by that window's input owner.
pub(crate) fn layers(a: &App, ctx: &egui::Context) {
    let key = key(ctx);
    let owner = commands::owner(a, ctx);
    ctx.data_mut(|d| {
        d.insert_temp(
            key,
            Layers {
                owner: Some(owner),
                ..Default::default()
            },
        )
    });
}

fn accepts(owner: Screen, screen: Screen) -> bool {
    owner.allows_companions()
        || owner == screen
        || matches!(
            (screen, owner),
            (Screen::Drums, Screen::DrumInput) | (Screen::Timeline, Screen::ParameterInput)
        )
}

pub(crate) fn refresh(a: &App, ctx: &egui::Context) {
    let key = key(ctx);
    let owner = commands::owner(a, ctx);
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Layers>(key).owner = Some(owner));
}

pub(crate) fn background_enabled(a: &App) -> bool {
    commands::root_owner(a).allows_companions()
}

/// The placed view whose own input, or the overview itself, is open: its pane stays usable
/// while the rest of the windows are disabled, as its floating window does.
pub(crate) fn input_view(a: &App, ctx: &egui::Context) -> Option<crate::view::dock::ViewKind> {
    use crate::view::dock::{focus, ViewKind};
    let view = match commands::root_owner(a) {
        Screen::DrumInput => ViewKind::StepSequencer,
        Screen::ParameterInput => ViewKind::ParameterCurve,
        Screen::Overview => ViewKind::TrackOverview,
        _ => return None,
    };
    (focus::placed(a, view) && !crate::view::command_input::preview_active(ctx)).then_some(view)
}

/// The panes of `host` are disabled as a whole, unless one of them holds the open input; then
/// each of the others disables itself.
pub(crate) fn panes_enabled(a: &App, ctx: &egui::Context, host: crate::view::dock::Host) -> bool {
    background_enabled(a)
        || input_view(a, ctx)
            .is_some_and(|view| crate::view::dock::focus::root(a).host_of(view) == Some(host))
}

fn register(ctx: &egui::Context, screen: Screen, id: egui::Id) -> (egui::Order, bool) {
    let key = key(ctx);
    ctx.data_mut(|d| {
        let layers = d.get_temp_mut_or_default::<Layers>(key);
        let inline_owner = layers
            .owner
            .filter(|&owner| !owner.allows_companions() && accepts(owner, screen));
        let effective = inline_owner.unwrap_or(screen);
        let order = effective.floating_order();
        layers
            .registered
            .push((effective, egui::LayerId::new(order, id)));
        (
            order,
            layers.owner.is_none_or(|owner| accepts(owner, screen)),
        )
    })
}

pub(crate) fn area(ctx: &egui::Context, screen: Screen, id: egui::Id) -> egui::Area {
    let (order, enabled) = register(ctx, screen, id);
    egui::Area::new(id).order(order).enabled(enabled)
}

pub(crate) fn window<'a>(
    ctx: &egui::Context,
    screen: Screen,
    id: egui::Id,
    title: impl Into<egui::WidgetText>,
) -> egui::Window<'a> {
    let (order, enabled) = register(ctx, screen, id);
    egui::Window::new(title)
        .id(id)
        .order(order)
        .enabled(enabled)
}

pub(crate) fn finish(a: &App, ctx: &egui::Context) {
    // A later child may open a dialog after an earlier companion was drawn.
    // Apply the final decision to every registered native window before presenting.
    let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
    let seen = ctx.data_mut(|d| {
        let state = d.get_temp_mut_or_default::<NativeState>(native_key());
        state.begin(pass);
        if state.modal.is_some_and(|(id, _)| {
            id != egui::ViewportId::ROOT && !state.seen.iter().any(|(shown, _, _)| *shown == id)
        }) {
            state.modal = None;
        }
        if state.popup.is_some_and(|(host, seen)| {
            (host == egui::ViewportId::ROOT && seen < pass) || seen + 1 < pass
        }) {
            state.popup = None;
        }
        state.seen.clone()
    });
    for (id, native, requested) in seen {
        windows::send_visible(ctx, id, requested && native.visible(a, ctx, id));
    }
    let target = effective_priority(a, ctx);
    let focus = windows::restored_focus(ctx, target);
    if let Some(target) = focus {
        send_to(ctx, target, egui::ViewportCommand::Focus);
    }
    windows::flush(ctx);
    // Real modal layers and their backdrop retain priority over floating editors.
    if super::modal::is_open(a)
        || ctx.memory(|m| m.top_modal_layer().is_some() || m.any_popup_open())
    {
        return;
    }
    let owner = commands::owner(a, ctx);
    if owner.allows_companions() {
        return;
    }
    let key = key(ctx);
    if let Some(layers) = ctx.data(|d| d.get_temp::<Layers>(key)) {
        for (screen, layer) in layers.registered {
            if screen == owner {
                ctx.move_to_top(layer);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
