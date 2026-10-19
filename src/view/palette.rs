//! A modal over a suspended input owner. Transport and workers keep updating.
use super::commands::{self, Command, Screen};
use crate::app::App;
use eframe::egui::{self, Key, Modifiers};
mod origin;
use origin::Origin;
#[cfg(test)]
pub(crate) mod tests;

#[derive(Default)]
pub struct State {
    origin: Option<Origin>,
    query: String,
    search: commands::SearchQuery,
    selected: usize,
    commands: Vec<Command>,
    reveal: bool,
    composing: bool,
    geometry: Option<(f32, f32)>,
    handled: Option<(egui::ViewportId, u64)>,
}
impl State {
    pub fn is_open(&self) -> bool {
        self.origin.is_some()
    }
}

pub(crate) fn frame_token(ctx: &egui::Context) -> (egui::ViewportId, u64) {
    (
        ctx.viewport_id(),
        ctx.cumulative_pass_nr()
            .saturating_sub(ctx.output(|o| o.num_completed_passes as u64)),
    )
}
pub(crate) fn suspended(ctx: &egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(egui::Id::new("palette-suspended"))
            .unwrap_or(false)
    })
}

/// Save the real input, then give background widgets an inert input state.
pub(crate) fn begin(a: &mut App, ctx: &egui::Context) -> Option<egui::InputState> {
    let token = frame_token(ctx);
    let opening=ctx.input(|i|i.events.iter().any(|e|matches!(e,egui::Event::Key{key:Key::P,pressed:true,repeat:false,modifiers,..} if commands::matches(*modifiers,Modifiers::CTRL|Modifiers::SHIFT))));
    if !a.palette.is_open() && opening && a.palette.handled != Some(token) {
        let origin = Origin::capture(a, ctx);
        a.palette = State {
            origin: Some(origin),
            commands: commands::candidates(a, ctx),
            reveal: true,
            ..Default::default()
        };
        a.palette.handled = Some(token);
        clear(ctx);
    }
    if !a.palette.is_open() {
        if a.palette.handled == Some(token) {
            clear(ctx);
        }
        return None;
    }
    let saved = ctx.input(Clone::clone);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("palette-suspended"), true));
    if let Some(origin) = &a.palette.origin
        && origin.viewport == ctx.viewport_id()
    {
        origin.restore_focus(ctx);
    }
    clear(ctx);
    Some(saved)
}

fn clear(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        i.events.clear();
        i.keys_down.clear();
        i.pointer = Default::default();
        i.modifiers = Modifiers::NONE;
        i.raw.events.clear();
        i.raw.dropped_files.clear();
        i.raw_scroll_delta = egui::Vec2::ZERO;
        i.smooth_scroll_delta = egui::Vec2::ZERO;
    });
}

pub(crate) fn end(a: &mut App, ctx: &egui::Context, input: Option<egui::InputState>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("palette-suspended"), false));
    let Some(input) = input else {
        return;
    };
    let mut palette = std::mem::take(&mut a.palette);
    let Some(origin) = &palette.origin else {
        a.palette = palette;
        return;
    };
    if origin.viewport != ctx.viewport_id() {
        a.palette = palette;
        clear(ctx);
        return;
    }
    origin.restore_cursor(a);
    ctx.input_mut(|i| *i = input);
    let token = frame_token(ctx);
    let fresh = palette.handled != Some(token);
    palette.handled = Some(token);
    let mut execute = false;
    let mut cancel = false;
    let invalid = !origin.current(a, ctx);
    if invalid {
        palette.commands = commands::candidates(a, ctx);
        palette.origin = Some(Origin::capture_with_focus(a, ctx, origin));
        palette.selected = 0;
        palette.reveal = true;
    }
    let events = ctx.input(|i| i.events.clone());
    let ime_enter = palette.composing || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
    if fresh {
        for event in &events {
            if let egui::Event::Ime(ime) = event {
                match ime {
                    egui::ImeEvent::Preedit(text) => palette.composing = !text.is_empty(),
                    egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => {
                        palette.composing = false
                    }
                    _ => {}
                }
            }
        }
        for event in &events {
            if let egui::Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
                && commands::matches(*modifiers, Modifiers::NONE)
            {
                match key {
                    Key::Escape if !ime_enter => cancel = true,
                    Key::Enter if !*repeat && !ime_enter && !invalid => execute = true,
                    Key::ArrowUp if !ime_enter => {
                        palette.selected = palette.selected.saturating_sub(1);
                        palette.reveal = true;
                    }
                    Key::ArrowDown if !ime_enter => {
                        palette.selected = palette.selected.saturating_add(1);
                        palette.reveal = true;
                    }
                    _ => {}
                }
            }
        }
    }
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            !matches!(e, egui::Event::Key { key: Key::P, .. })
                && !(matches!(
                    e,
                    egui::Event::Key {
                        key: Key::ArrowUp | Key::ArrowDown | Key::Enter | Key::Escape,
                        ..
                    }
                ) && !ime_enter)
        })
    });
    if !fresh {
        clear(ctx);
    }
    let mut filtered = filter(&palette.commands, &palette.query, &mut palette.search);
    palette.selected = palette.selected.min(filtered.len().saturating_sub(1));
    let search_input = events.iter().any(|e| {
        matches!(
            e,
            egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Ime(_)
        )
    });
    let chosen = (execute && !search_input)
        .then(|| filtered.get(palette.selected).copied())
        .flatten();
    if cancel || chosen.is_some() {
        clear(ctx);
        if let Some(origin) = &palette.origin {
            origin.restore_focus(ctx);
        }
        let completed = cancel
            || chosen.is_some_and(|index| commands::execute(a, ctx, &palette.commands[index]));
        if completed {
            palette.origin = None;
            palette.commands.clear();
            a.palette = palette;
            ctx.request_repaint();
            return;
        }
        palette.commands = commands::candidates(a, ctx);
        palette.selected = 0;
        palette.reveal = true;
        filtered = filter(&palette.commands, &palette.query, &mut palette.search);
    }
    let id = egui::Id::new("command-palette");
    let (width, left) = *palette.geometry.get_or_insert_with(|| {
        let style = ctx.style();
        let font = egui::TextStyle::Button.resolve(&style);
        let text_width = ctx.fonts(|fonts| {
            palette
                .commands
                .iter()
                .map(|command| {
                    fonts
                        .layout_no_wrap(row_text(command), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                })
                .fold(0.0, f32::max)
        });
        let margins = egui::Frame::popup(&style).total_margin().sum().x;
        let width = (text_width
            + 2.0 * style.spacing.button_padding.x
            + style.spacing.scroll.allocated_width())
        .ceil()
        .min((ctx.screen_rect().width() - 40.0 - margins).max(1.0));
        (
            width,
            ctx.screen_rect().center().x - (width + margins) / 2.0,
        )
    });
    let area = egui::Modal::default_area(id).anchor(
        egui::Align2::LEFT_TOP,
        egui::vec2(left - ctx.screen_rect().left(), 8.0),
    );
    egui::Modal::new(id).area(area).show(ctx, |ui| {
        ui.set_width(width);
        // Keep room to grow again after clearing a filter, without reserving it.
        ui.set_max_height((ctx.screen_rect().bottom() - ui.min_rect().top() - 24.0).max(1.0));
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("command-palette-query")));
        let response = ui.add(
            egui::TextEdit::singleline(&mut palette.query)
                .id(egui::Id::new("command-palette-query"))
                .desired_width(f32::INFINITY),
        );
        response.request_focus();
        if response.changed() {
            palette.selected = 0;
            palette.reveal = true;
            filtered = filter(&palette.commands, &palette.query, &mut palette.search);
        }
        egui::ScrollArea::vertical()
            .id_salt("palette-results")
            .max_height((ctx.screen_rect().bottom() - ui.cursor().top() - 24.0).max(1.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (row, &index) in filtered.iter().enumerate() {
                    let command = &palette.commands[index];
                    let response = ui.selectable_label(row == palette.selected, row_text(command));
                    if row == palette.selected && palette.reveal {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response.clicked() {
                        palette.selected = row;
                    }
                }
            });
    });
    palette.reveal = false;
    a.palette = palette;
    clear(ctx);
}

fn row_text(command: &Command) -> String {
    format!(
        "{}    {} ({})",
        command.label(),
        command.key_label(),
        command.screen.label()
    )
}

fn filter(commands: &[Command], query: &str, search: &mut commands::SearchQuery) -> Vec<usize> {
    search.update(query);
    commands
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.matches_query(search).then_some(i))
        .collect()
}
