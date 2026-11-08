//! Drives the app frame by frame, with native windows rendered as immediate viewports.
use crate::app::App;
use eframe::egui::{self, Key, Modifiers};

pub(crate) fn context() -> egui::Context {
    let c = egui::Context::default();
    c.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
        let id = viewport.ids.this;
        ctx.data_mut(|d| remember(d, "workspace-rendered", id));
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("workspace-visible", id)),
                viewport.builder.visible.unwrap_or(true),
            )
        });
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("window-builder", id)),
                viewport.builder.clone(),
            )
        });
        let geometry = ctx.data(|d| d.get_temp::<[f32; 4]>(egui::Id::new(("window-geometry", id))));
        let close = ctx
            .data_mut(|d| d.remove_temp::<bool>(egui::Id::new(("workspace-close", id))))
            .unwrap_or_default();
        let events = ctx
            .data_mut(|d| d.remove_temp::<Vec<egui::Event>>(egui::Id::new(("workspace-input", id))))
            .unwrap_or_default();
        let mut input = egui::RawInput {
            viewport_id: id,
            events,
            focused: focused(ctx, id),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                viewport
                    .builder
                    .inner_size
                    .unwrap_or(egui::vec2(760.0, 440.0)),
            )),
            ..Default::default()
        };
        input.viewports.insert(
            id,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                focused: Some(focused(ctx, id)),
                outer_rect: geometry.map(|[x, y, w, h]| {
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w + 16.0, h + 40.0))
                }),
                inner_rect: geometry.map(|[x, y, w, h]| {
                    egui::Rect::from_min_size(egui::pos2(x + 8.0, y + 32.0), egui::vec2(w, h))
                }),
                events: if close {
                    vec![egui::ViewportEvent::Close]
                } else {
                    vec![]
                },
                ..Default::default()
            },
        );
        let out = ctx.run(input, |ctx| (viewport.viewport_ui_cb)(ctx));
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new(("workspace-shapes", id)), out.shapes.clone())
        });
        record(ctx, &out);
    });
    c
}
/// Viewports that a frame's output handed to the backend without showing them since the last
/// call. The backend opens a native window for each, with egui's default title.
pub(crate) fn unshown(c: &egui::Context) -> Vec<egui::ViewportId> {
    c.data_mut(|d| {
        let rendered = d
            .remove_temp::<Vec<egui::ViewportId>>(egui::Id::new("workspace-rendered"))
            .unwrap_or_default();
        let mut out = d
            .remove_temp::<Vec<egui::ViewportId>>(egui::Id::new("workspace-outputs"))
            .unwrap_or_default();
        out.retain(|id| *id != egui::ViewportId::ROOT && !rendered.contains(id));
        out
    })
}
fn remember(d: &mut egui::util::IdTypeMap, list: &str, id: egui::ViewportId) {
    let ids = d.get_temp_mut_or_default::<Vec<egui::ViewportId>>(egui::Id::new(list));
    if !ids.contains(&id) {
        ids.push(id);
    }
}
/// Every window counts as focused unless `focus` picked one.
fn focused(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data(|d| d.get_temp::<egui::ViewportId>(egui::Id::new("workspace-focused-only")))
        .is_none_or(|only| only == id)
}
/// The next frame of `id` carries the operating system's close request.
pub(crate) fn request_close(c: &egui::Context, id: egui::ViewportId) {
    c.data_mut(|d| d.insert_temp(egui::Id::new(("workspace-close", id)), true));
}
/// From now on only `id` reports the operating system focus.
pub(crate) fn focus(c: &egui::Context, id: egui::ViewportId) {
    c.data_mut(|d| d.insert_temp(egui::Id::new("workspace-focused-only"), id));
}
pub(crate) fn frame(
    a: &mut App,
    c: &egui::Context,
    id: egui::ViewportId,
    events: Vec<egui::Event>,
) {
    let root = if id == egui::ViewportId::ROOT {
        events
    } else {
        c.data_mut(|d| d.insert_temp(egui::Id::new(("workspace-input", id)), events));
        vec![]
    };
    let mut input = egui::RawInput {
        events: root,
        focused: focused(c, egui::ViewportId::ROOT),
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1500.0, 900.0),
        )),
        ..Default::default()
    };
    // The main window reports where it is only when a test placed it.
    let geometry = c.data(|d| {
        d.get_temp::<[f32; 4]>(egui::Id::new(("window-geometry", egui::ViewportId::ROOT)))
    });
    if let Some([x, y, w, h]) = geometry {
        input.viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                focused: Some(focused(c, egui::ViewportId::ROOT)),
                outer_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(w + 16.0, h + 40.0),
                )),
                inner_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(x + 8.0, y + 32.0),
                    egui::vec2(w, h),
                )),
                ..Default::default()
            },
        );
    }
    let out = c.run(input, |ctx| crate::view::show(a, ctx));
    c.data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("workspace-shapes", egui::ViewportId::ROOT)),
            out.shapes.clone(),
        )
    });
    record(c, &out);
}
fn record(c: &egui::Context, out: &egui::FullOutput) {
    for (&id, out) in &out.viewport_output {
        c.data_mut(|d| remember(d, "workspace-outputs", id));
        c.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("workspace-commands", id)),
                out.commands.clone(),
            )
        });
        if out.commands.contains(&egui::ViewportCommand::Focus) {
            c.data_mut(|d| d.insert_temp(egui::Id::new(("workspace-focus", id)), true));
        }
    }
}
pub(crate) fn key(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key, m: Modifiers) {
    // Native Windows input sets both ctrl and command. egui's text selection
    // uses command, while the application's physical bindings use ctrl.
    let m = Modifiers {
        command: m.command || m.ctrl,
        ..m
    };
    frame(
        a,
        c,
        id,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key: k,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: m,
            })
            .collect(),
    );
}
pub(crate) fn text(a: &mut App, c: &egui::Context, id: egui::ViewportId, s: &str) {
    frame(a, c, id, vec![]);
    frame(a, c, id, vec![egui::Event::Text(s.into())]);
}
/// Runs the palette command `command` from viewport `id`.
pub(crate) fn command(a: &mut App, c: &egui::Context, id: egui::ViewportId, command: &str) {
    use crate::view::palette::tests::{name_of, row_of};
    key(a, c, id, Key::P, Modifiers::CTRL | Modifiers::SHIFT);
    let name = name_of(a, command);
    text(a, c, id, &name);
    for _ in 0..row_of(a, command) {
        key(a, c, id, Key::ArrowDown, Modifiers::NONE);
    }
    key(a, c, id, Key::Enter, Modifiers::NONE);
}
