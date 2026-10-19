//! Spectrum and Lissajous of the output, in a window of its own so the layer
//! overlay and the mixer stay usable beside it.
use crate::app::App;
use crate::play::analysis::Worker;
use cat_smf_editor::{correlation_ui, lissajous, lissajous_ui, spectrum, spectrum_ui};
use eframe::egui;
use std::time::Duration;

#[cfg(windows)]
mod owner;

/// Unique among the editor's windows; the owner is found by it.
const TITLE: &str = "Spectrum";

#[derive(Default)]
pub struct State {
    /// Running exactly while the window is open.
    worker: Option<Worker>,
    /// Outer position and inner size, saved on closing and on exit; saving every
    /// move would rewrite the status file throughout a drag.
    seen: Option<[f32; 4]>,
    /// Each opening creates a new OS window, which needs its owner again.
    owned: bool,
    /// Bumped to replace the OS window by one the system places afresh.
    placement: u64,
}

impl State {
    pub fn is_open(&self) -> bool {
        self.worker.is_some()
    }
}

pub fn toggle(app: &mut App) {
    let state = &mut app.analysis;
    if state.worker.take().is_some() {
        if let Some(window) = state.seen.take() {
            app.config.select_analysis_window(window);
        }
    } else {
        state.worker = Some(Worker::start());
        state.owned = false;
    }
    app.config.select_analysis_open(app.analysis.is_open());
}

/// For a window that ended up off every screen.
pub fn reset_window(app: &mut App) {
    app.config.forget_analysis_window();
    let state = &mut app.analysis;
    state.seen = None;
    state.placement += 1;
    state.owned = false;
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(worker) = &app.analysis.worker else {
        return;
    };
    let (spectrum, lissajous) = (worker.spectrum(), worker.lissajous());
    let (editor_focused, exiting) = ctx.input(|i| {
        let viewport = i.viewport();
        (
            viewport.focused.unwrap_or(false),
            viewport.close_requested(),
        )
    });
    // Opening must leave the keyboard with the editor and its overlays.
    let mut builder = egui::ViewportBuilder::default()
        .with_title(TITLE)
        .with_active(false)
        .with_inner_size([760.0, 260.0]);
    if let Some([x, y, width, height]) = app.config.analysis_window {
        builder = builder
            .with_position([x, y])
            .with_inner_size([width, height]);
    }
    let mut close = false;
    ctx.show_viewport_immediate(viewport(app.analysis.placement), builder, |ctx, _class| {
        let palette_input = crate::view::palette::begin(app, ctx);
        crate::view::commands::route(app, ctx);
        let (requested, outer, inner) = ctx.input(|i| {
            let viewport = i.viewport();
            (
                viewport.close_requested(),
                viewport.outer_rect,
                viewport.inner_rect,
            )
        });
        close = requested;
        if let (Some(outer), Some(inner)) = (outer, inner) {
            app.analysis.seen = Some([outer.min.x, outer.min.y, inner.width(), inner.height()]);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            body(ui, spectrum.as_deref(), lissajous.as_deref());
        });
        crate::view::palette::end(app, ctx, palette_input);
        // The child viewport is painted by the root update, which must keep running.
        ctx.request_repaint_after_for(Duration::from_millis(50), egui::ViewportId::ROOT);
        ctx.request_repaint_after(Duration::from_millis(50));
    });
    let state = &mut app.analysis;
    // The editor is the foreground window while it has focus.
    #[cfg(windows)]
    if !state.owned && editor_focused {
        state.owned = owner::adopt(TITLE);
    }
    if close {
        toggle(app);
    } else if exiting && let Some(window) = state.seen {
        app.config.select_analysis_window(window);
    }
}

fn viewport(placement: u64) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("output-analysis", placement))
}

/// Spectrum on the left; the square Lissajous and its correlation bar on the right.
fn body(
    ui: &mut egui::Ui,
    spectrum: Option<&spectrum::Display>,
    lissajous: Option<&lissajous::Display>,
) {
    const CORRELATION_HEIGHT: f32 = 28.0;
    let spacing = ui.spacing().item_spacing;
    let height = ui.available_height();
    let side = (height - CORRELATION_HEIGHT - spacing.y).max(0.0);
    ui.horizontal_top(|ui| {
        let width = (ui.available_width() - side - spacing.x).max(0.0);
        ui.allocate_ui(egui::vec2(width, height), |ui| {
            spectrum_ui::panel(ui, spectrum, true, height, false);
        });
        ui.allocate_ui(egui::vec2(side, height), |ui| {
            ui.vertical(|ui| {
                lissajous_ui::panel(ui, lissajous, true, side, false);
                correlation_ui::panel(ui, lissajous.and_then(|f| f.correlation), false);
            });
        });
    });
}
