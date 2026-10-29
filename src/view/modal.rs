//! Shared suspension, focus and frame ownership for independent modal UIs.
use crate::app::App;
use eframe::egui;
mod input;
mod origin;
pub(crate) use input::{clear, frame_token, suspended};
pub(crate) use origin::Origin;

pub(crate) fn suspend(ctx: &egui::Context, origin: &Origin) -> egui::InputState {
    mark_owned(ctx);
    let focus_id = egui::Id::new(("modal-input-focus", ctx.viewport_id()));
    let focus = ctx.memory(|m| m.focused());
    ctx.data_mut(|d| d.insert_temp(focus_id, focus));
    let saved = ctx.input(Clone::clone);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("modal-suspended"), true));
    if origin.viewport == ctx.viewport_id() {
        origin.restore_focus(ctx);
    }
    clear(ctx);
    saved
}
pub(crate) fn resume(ctx: &egui::Context, input: egui::InputState) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("modal-suspended"), false));
    ctx.input_mut(|i| *i = input);
    let focus_id = egui::Id::new(("modal-input-focus", ctx.viewport_id()));
    if let Some(id) = ctx
        .data(|d| d.get_temp::<Option<egui::Id>>(focus_id))
        .flatten()
    {
        ctx.memory_mut(|m| m.request_focus(id));
    }
}
pub(crate) fn is_open(a: &App) -> bool {
    a.palette.is_open() || a.command_input.is_open() || a.patterns.recording.is_open()
}
pub(crate) fn mark_owned(ctx: &egui::Context) {
    let token = frame_token(ctx);
    let id = egui::Id::new(("modal-owned-frame", ctx.viewport_id()));
    ctx.data_mut(|d| d.insert_temp(id, token));
}
pub(crate) fn owns_frame(ctx: &egui::Context) -> bool {
    let token = frame_token(ctx);
    let id = egui::Id::new(("modal-owned-frame", ctx.viewport_id()));
    ctx.data(|d| d.get_temp::<(egui::ViewportId, u64)>(id)) == Some(token)
}
pub(crate) fn begin(a: &mut App, ctx: &egui::Context) -> Option<egui::InputState> {
    super::help::input(a, ctx);
    super::help::observe(a, ctx);
    if !is_open(a) && owns_frame(ctx) {
        clear(ctx);
        return None;
    }
    if a.patterns.recording.is_open() {
        super::recording::begin(a, ctx)
    } else if a.command_input.is_open() {
        super::command_input::begin(a, ctx)
    } else {
        super::palette::begin(a, ctx)
    }
}
pub(crate) fn end(a: &mut App, ctx: &egui::Context, input: Option<egui::InputState>) {
    if a.patterns.recording.is_open() {
        super::recording::end(a, ctx, input);
    } else if a.command_input.is_open() {
        super::command_input::end(a, ctx, input);
    } else {
        super::palette::end(a, ctx, input);
    }
}
