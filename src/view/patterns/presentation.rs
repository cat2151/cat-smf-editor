//! Shared identity for the event pane and the focused ordinary note editor.
use crate::app::App;
use eframe::egui::{self, Color32};

pub(crate) const ACCENT: Color32 = Color32::from_rgb(0xae, 0x81, 0xff);
pub fn name(a: &App, track: usize) -> Option<String> {
    let edit = super::mode::editing(a, track)?;
    let (catalog, _) = a.loaded.as_ref()?.arrangement.as_ref()?;
    Some(catalog.patterns.get(&edit.pattern)?.name.clone())
}
pub fn frame(a: &App, track: usize) -> egui::Frame {
    if super::mode::source(a, track) {
        egui::Frame::NONE
            .stroke(egui::Stroke::new(2.0_f32, ACCENT))
            .inner_margin(6.0)
    } else {
        egui::Frame::NONE
    }
}
pub fn modal_frame(a: &App, ctx: &egui::Context, track: usize) -> egui::Frame {
    let mut frame = egui::Frame::popup(&ctx.style());
    if super::mode::source(a, track) {
        frame.stroke = egui::Stroke::new(2.0_f32, ACCENT);
    }
    frame
}
pub fn label(ui: &mut egui::Ui, name: &str) {
    ui.colored_label(
        ACCENT,
        egui::RichText::new(format!("Pattern · {name}")).strong(),
    );
}
/// Hands the keys to the main window's event list.
pub fn focus_root(a: &mut App, ctx: &egui::Context) {
    use crate::view::dock::{focus, ViewKind};
    if matches!(focus::focused(a), ViewKind::Song | ViewKind::Scene) {
        focus::set(a, ViewKind::EventList);
    }
    a.patterns.root_focus = true;
    crate::view::presentation::send_to(ctx, egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
    ctx.request_repaint_of(egui::ViewportId::ROOT);
}
/// A child window's command repeats the focus request from the root frame.
pub fn refocus_root(a: &mut App, ctx: &egui::Context) {
    if a.patterns.root_focus {
        crate::view::presentation::send_to(
            ctx,
            egui::ViewportId::ROOT,
            egui::ViewportCommand::Focus,
        );
        a.patterns.root_focus = false;
    }
}

pub fn heading(a: &App, ui: &mut eframe::egui::Ui, track: usize) {
    if !a.note_mode {
        return;
    }
    if let Some(context) = super::mode::editing(a, track)
        && let Some(l) = &a.loaded
        && let Some((catalog, collection)) = &l.arrangement
        && let Some(pattern) = catalog.patterns.get(&context.pattern)
    {
        let scene = context
            .scene
            .and_then(|id| collection.scenes.iter().find(|s| s.id == id))
            .map(|s| format!("{} · ", s.name))
            .unwrap_or_default();
        if context.standalone {
            if let Some(patch) = l.patches.get(pattern.track) {
                ui.label(
                    patch
                        .layers
                        .iter()
                        .map(|l| l.label())
                        .collect::<Vec<_>>()
                        .join(" · "),
                );
            }
        } else {
            ui.strong(format!(
                "{scene}Track {} · meas. {}",
                track + 1,
                context.measure
            ));
        }
    }
}
