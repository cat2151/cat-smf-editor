//! Companion geometry belongs to the app, independently of songs and edit state.
use crate::app::App;
use eframe::egui::{self, ViewportBuilder, ViewportCommand};

const TITLES: [&str; 2] = ["Scene", "Pattern / Song"];

fn defaults(title: &str) -> [f32; 4] {
    match title {
        "Scene" => [460.0, 40.0, 760.0, 440.0],
        "Pattern / Song" => [460.0, 520.0, 760.0, 520.0],
        _ => unreachable!(),
    }
}

pub fn builder(a: &App, title: &str) -> ViewportBuilder {
    let saved = a.config.pattern_windows.get(title).copied();
    let [x, y, width, height] = saved.unwrap_or_else(|| defaults(title));
    ViewportBuilder::default()
        .with_inner_size([width, height])
        .with_position([x, y])
}

fn reset_guard(title: &str) -> egui::Id {
    egui::Id::new(("pattern-window-reset", title))
}

pub fn remember(a: &mut App, ctx: &egui::Context, title: &str) {
    // Reset commands are applied by the native backend after this frame. Do not
    // put the old rectangle back while those commands are still pending.
    if ctx
        .data_mut(|d| d.remove_temp::<bool>(reset_guard(title)))
        .unwrap_or(false)
    {
        return;
    }
    let window = ctx.input(|i| {
        let viewport = i.viewport();
        Some((viewport.outer_rect?, viewport.inner_rect?))
    });
    if let Some((outer, inner)) = window {
        let window = [outer.min.x, outer.min.y, inner.width(), inner.height()];
        if window.iter().all(|v| v.is_finite()) && window[2] > 0.0 && window[3] > 0.0 {
            // Persist on app exit, rather than rewriting status throughout a drag.
            a.config.pattern_windows.insert(title.to_owned(), window);
        }
    }
}

pub fn reset(a: &mut App, ctx: &egui::Context) {
    for (title, id) in TITLES.into_iter().zip([
        super::workspace::Pane::Scene.viewport(),
        super::workspace::Pane::Song.viewport(),
    ]) {
        let [x, y, width, height] = defaults(title);
        a.config.pattern_windows.remove(title);
        ctx.send_viewport_cmd_to(id, ViewportCommand::OuterPosition(egui::pos2(x, y)));
        ctx.send_viewport_cmd_to(id, ViewportCommand::InnerSize(egui::vec2(width, height)));
        ctx.data_mut(|d| d.insert_temp(reset_guard(title), true));
    }
    a.config.persist();
}

#[cfg(test)]
mod tests;
