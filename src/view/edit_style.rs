//! A first editing destination for each newly created song; never persisted.
use super::commands::{Command, Screen};
use crate::app::App;
use eframe::egui::{self, Key};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Pattern,
    Direct,
}

pub fn open(app: &mut App) {
    app.edit_style = Some(Style::Pattern);
}

fn choose(app: &mut App, ctx: &egui::Context, style: Style) {
    app.edit_style = None;
    match style {
        Style::Pattern => {
            let request =
                super::commands::Request::new(app, super::commands::Operation::PatternWork);
            if let Err(error) = super::commands::submit(app, ctx, request) {
                app.load_error = Some(error);
            }
        }
        Style::Direct => super::busy::request(app, ctx, super::busy::Task::MmlInput),
    }
}

pub fn collect(out: &mut Vec<Command>) {
    for (id, name, key, style) in [
        ("style-pattern", "Patternスタイル", Key::P, Style::Pattern),
        ("style-direct", "Directスタイル", Key::D, Style::Direct),
    ] {
        out.push(Command::plain(
            Screen::EditStyle,
            id,
            name,
            &[key],
            move |a, c| {
                choose(a, c, style);
            },
        ));
    }
    out.push(Command::plain(
        Screen::EditStyle,
        "style-select",
        "スタイル選択",
        &[
            Key::ArrowUp,
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::Tab,
        ],
        |a, _| {
            a.edit_style = Some(match a.edit_style {
                Some(Style::Pattern) => Style::Direct,
                _ => Style::Pattern,
            });
        },
    ));
    out.push(Command::plain(
        Screen::EditStyle,
        "style-confirm",
        "確定",
        &[Key::Enter],
        |a, c| {
            if let Some(style) = a.edit_style {
                choose(a, c, style);
            }
        },
    ));
    out.push(Command::plain(
        Screen::EditStyle,
        "style-cancel",
        "閉じる",
        &[Key::Escape],
        |a, _| {
            a.edit_style = None;
        },
    ));
}

/// Keep the normal panels and score rendering, while the dialog owns all input.
pub fn suspend_background(app: &App, ctx: &egui::Context) -> Option<egui::InputState> {
    if app.edit_style.is_none() || super::commands::owner(app, ctx) != Screen::EditStyle {
        return None;
    }
    let input = ctx.input(Clone::clone);
    ctx.input_mut(|i| {
        i.events.clear();
        i.raw.events.clear();
        i.raw.dropped_files.clear();
        i.keys_down.clear();
        i.pointer = Default::default();
        i.modifiers = Default::default();
        i.raw_scroll_delta = egui::Vec2::ZERO;
        i.smooth_scroll_delta = egui::Vec2::ZERO;
    });
    Some(input)
}

pub fn show(app: &mut App, ctx: &egui::Context, input: Option<egui::InputState>) {
    let Some(input) = input else {
        if app.edit_style.is_some() {
            ctx.request_repaint();
        }
        return;
    };
    ctx.input_mut(|i| *i = input);
    super::commands::route(app, ctx);
    if let Some(selected) = app.edit_style {
        let mut choice = None;
        egui::Modal::new(egui::Id::new("edit-style")).show(ctx, |ui| {
            ui.set_min_width(300.0);
            ui.heading("editスタイルを選んでください");
            for (style, label) in [
                (Style::Pattern, "P: Patternスタイル"),
                (Style::Direct, "D: Directスタイル"),
            ] {
                if ui.selectable_label(selected == style, label).clicked() {
                    choice = Some(style);
                }
            }
            ui.separator();
            ui.small("↑ ↓: 選択　Enter: 確定　Esc: 閉じる");
        });
        if let Some(style) = choice {
            choose(app, ctx, style);
        }
    }
    super::comma_menu::swallow(ctx);
}

#[cfg(test)]
mod tests;
