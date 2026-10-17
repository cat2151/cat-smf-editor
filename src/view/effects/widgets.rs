use super::{Action, Target};
use crate::app::App;
use cat_smf_editor::chain_settings::PluginState;
use eframe::egui::{self, Color32, RichText};

const WIDTH: f32 = 340.0;
const NODE_HEIGHT: f32 = 28.0;
const GAP: f32 = 18.0;

pub(super) fn show(
    app: &mut App,
    ctx: &egui::Context,
    target: Target,
    chain: &[PluginState],
) -> Vec<Action> {
    let mut actions = Vec::new();
    let source = source(app, target);
    let auto_gain = app.audition.auto_gain_enabled();
    let error = app.player_selection.error.clone();
    let state = &mut app.effect_editor;
    egui::Modal::new(egui::Id::new("effect-chain")).show(ctx, |ui| {
        if state.help {
            super::keyboard::help(state, ui);
            return;
        }
        ui.set_width(WIDTH);
        ui.horizontal(|ui| {
            ui.heading(match target {
                Target::Track(_) => "Track effects",
                Target::Master => "Master effects",
            });
            if ui.button("+").clicked() {
                actions.push(Action::Browse(chain.len()));
            }
            if ui.button("×").clicked() {
                actions.push(Action::Close);
            }
        });
        egui::ScrollArea::vertical()
            .max_height((ctx.screen_rect().height() - 120.0).max(80.0))
            .show(ui, |ui| {
                group(ui, &source.0, &source.1);
                connector(ui);
                let mut rects = Vec::new();
                for (slot, effect) in chain.iter().enumerate() {
                    let (rect, action) = node(ui, state, slot, effect);
                    actions.extend(action);
                    rects.push(rect);
                    connector(ui);
                }
                actions.extend(drag(ui, state, &rects));
                match target {
                    Target::Track(_) => {
                        sink(ui, "Mixer", true);
                    }
                    Target::Master => {
                        if sink(ui, "Auto gain", auto_gain).clicked() {
                            actions.push(Action::AutoGain);
                        }
                        connector(ui);
                        sink(ui, "Output", true);
                    }
                }
            });
        if let Some(error) = &error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
    });
    actions
}

/// The group title and its members: a track's layers, or the master's tracks.
fn source(app: &App, target: Target) -> (String, Vec<String>) {
    let Some(loaded) = &app.loaded else {
        return (String::new(), Vec::new());
    };
    match target {
        Target::Track(track) => (
            loaded.track_titles.get(track).cloned().unwrap_or_default(),
            loaded.patches.get(track).map_or_else(Vec::new, |patch| {
                patch.layers.iter().map(|l| l.label().to_owned()).collect()
            }),
        ),
        Target::Master => ("Tracks".into(), loaded.track_titles.clone()),
    }
}

fn group(ui: &mut egui::Ui, title: &str, members: &[String]) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(WIDTH - 16.0);
        ui.add(egui::Label::new(RichText::new(title).strong()).truncate());
        for member in members {
            ui.add(egui::Label::new(RichText::new(member).weak()).truncate())
                .on_hover_text(member);
        }
    });
}

fn connector(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(WIDTH, GAP), egui::Sense::hover());
    let stroke = egui::Stroke::new(1.5_f32, ui.visuals().weak_text_color());
    ui.painter()
        .arrow(rect.center_top(), egui::vec2(0.0, GAP - 2.0), stroke);
}

/// A fixed end of the chain; `lit` false draws it dimmed.
fn sink(ui: &mut egui::Ui, label: &str, lit: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(WIDTH, NODE_HEIGHT), egui::Sense::click());
    let visuals = ui.visuals();
    let color = if lit {
        visuals.strong_text_color()
    } else {
        visuals.weak_text_color()
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, visuals.faint_bg_color);
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0_f32, color),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Button.resolve(ui.style()),
        color,
    );
    response
}

/// One effect: the body selects, double-click replaces and dragging reorders.
fn node(
    ui: &mut egui::Ui,
    state: &mut super::State,
    slot: usize,
    effect: &PluginState,
) -> (egui::Rect, Vec<Action>) {
    let mut actions = Vec::new();
    let (rect, body) = ui.allocate_exact_size(
        egui::vec2(WIDTH, NODE_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    let selected = state.selected == slot;
    if selected && state.reveal {
        body.scroll_to_me(Some(egui::Align::Center));
        state.reveal = false;
    }
    if body.clicked() || body.drag_started() {
        state.selected = slot;
    }
    if body.double_clicked() {
        actions.push(Action::Browse(slot));
    }
    if body.drag_started() {
        state.drag = Some(slot);
    }
    let visuals = ui.visuals().clone();
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);
    let stroke = if selected {
        egui::Stroke::new(2.0_f32, crate::view::ACTIVE_PANE)
    } else {
        egui::Stroke::new(1.0_f32, visuals.widgets.noninteractive.bg_stroke.color)
    };
    painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
    let name = crate::play::mixer::effect_name(effect);
    let color = if effect.bypass {
        crate::play::mixer::EFFECT_COLOR.gamma_multiply(0.35)
    } else {
        crate::play::mixer::EFFECT_COLOR
    };
    let inner = rect.shrink2(egui::vec2(8.0, 0.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    if child
        .push_id(("remove", slot), |ui| ui.button("−"))
        .inner
        .clicked()
    {
        state.selected = slot;
        actions.push(Action::Remove(slot));
    }
    if bypass_button(&mut child, effect.bypass).clicked() {
        state.selected = slot;
        actions.push(Action::Bypass(slot));
    }
    child.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.add(
            egui::Label::new(RichText::new(format!("{} {name}", slot + 1)).color(color))
                .truncate()
                .selectable(false),
        )
        .on_hover_text(name);
    });
    (rect, actions)
}

/// A power symbol; lit while the effect is bypassed.
fn bypass_button(ui: &mut egui::Ui, bypassed: bool) -> egui::Response {
    let size = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let visuals = ui.style().interact_selectable(&response, bypassed);
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, visuals.weak_bg_fill);
    let center = rect.center();
    let radius = size * 0.28;
    let stroke = egui::Stroke::new(1.5_f32, visuals.fg_stroke.color);
    // The gap at the top leaves room for the stem.
    let arc: Vec<_> = (0..=24)
        .map(|i| {
            let angle = -std::f32::consts::FRAC_PI_2
                + 0.6
                + (std::f32::consts::TAU - 1.2) * i as f32 / 24.0;
            center + radius * egui::vec2(angle.cos(), angle.sin())
        })
        .collect();
    painter.add(egui::Shape::line(arc, stroke));
    painter.line_segment(
        [
            center - egui::vec2(0.0, radius * 1.25),
            center - egui::vec2(0.0, radius * 0.2),
        ],
        stroke,
    );
    response.on_hover_text("Bypass")
}

/// Reorders on release; while dragging, a line marks where the effect lands.
fn drag(ui: &mut egui::Ui, state: &mut super::State, rects: &[egui::Rect]) -> Option<Action> {
    let from = state.drag?;
    let pointer = ui.ctx().pointer_interact_pos()?;
    let to = rects
        .iter()
        .enumerate()
        .filter(|(i, rect)| *i != from && rect.center().y < pointer.y)
        .count();
    if ui.input(|i| i.pointer.primary_down()) {
        let edge = if to >= rects.len().saturating_sub(1) {
            rects.last()?.bottom() + 2.0
        } else if to < from {
            rects[to].top() - 2.0
        } else {
            rects[to].bottom() + 2.0
        };
        let x = rects[0].x_range();
        ui.painter().hline(
            x,
            edge,
            egui::Stroke::new(2.0_f32, crate::view::ACTIVE_PANE),
        );
        return None;
    }
    state.drag = None;
    Some(Action::Move { from, to })
}
