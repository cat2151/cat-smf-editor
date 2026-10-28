//! A modal over a suspended input owner. Transport and workers keep updating.
use super::commands::{self, Command, Screen};
use crate::app::App;
use eframe::egui::{self, Key, Modifiers};
mod history;
mod input;
use input::clear;
pub(crate) use input::{frame_token, suspended};
mod insert_event;
mod origin;
mod pattern_args;
pub(super) mod row;
mod same_measure;
mod views;
use origin::Origin;
pub(crate) use views::preview::{
    active as preview_active, id as preview_id, window as preview_window,
};
#[cfg(test)]
pub(crate) mod tests;

#[derive(Default)]
pub struct State {
    history: history::History,
    origin: Option<Origin>,
    query: String,
    search: commands::SearchQuery,
    selected: usize,
    commands: Vec<Command>,
    reveal: bool,
    composing: bool,
    geometry: Option<(f32, f32)>,
    handled: Option<(egui::ViewportId, u64)>,
    views: Option<views::State>,
    insert_event: Option<insert_event::State>,
    same_measure: Option<same_measure::State>,
    pattern_args: Option<pattern_args::State>,
}
impl State {
    pub(crate) fn browsing_views(&self) -> bool {
        self.views.is_some()
    }
    pub fn is_open(&self) -> bool {
        self.origin.is_some()
    }
    pub(crate) fn owns_root(&self) -> bool {
        self.origin
            .as_ref()
            .is_some_and(|o| o.viewport == egui::ViewportId::ROOT)
    }
}

pub(crate) use same_measure::open as open_same_measure;

/// Save the real input, then give background widgets an inert input state.
pub(crate) fn begin(a: &mut App, ctx: &egui::Context) -> Option<egui::InputState> {
    if (a.palette.browsing_views()
        || a.palette.pattern_args.is_some()
        || a.palette.same_measure.is_some()
        || a.palette.insert_event.is_some())
        && ctx.viewport_id() == egui::ViewportId::ROOT
        && a.palette
            .origin
            .as_ref()
            .is_some_and(|o| o.viewport != egui::ViewportId::ROOT)
    {
        a.palette.origin = Some(Origin::capture(a, ctx));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    let token = frame_token(ctx);
    let opening=ctx.input(|i|i.events.iter().any(|e|matches!(e,egui::Event::Key{key:Key::P,pressed:true,repeat:false,modifiers,..} if commands::matches(*modifiers,Modifiers::CTRL|Modifiers::SHIFT))));
    if !a.palette.is_open() && opening && a.palette.handled != Some(token) {
        let origin = Origin::capture(a, ctx);
        a.palette = State {
            history: history::History::load(a),
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
    let argument_focus = ctx.memory(|m| m.focused());
    let focus_id = egui::Id::new(("palette-argument-focus", ctx.viewport_id()));
    ctx.data_mut(|d| d.insert_temp(focus_id, argument_focus));
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
    let focus_id = egui::Id::new(("palette-argument-focus", ctx.viewport_id()));
    if palette.pattern_args.is_some()
        && let Some(id) = ctx
            .data(|d| d.get_temp::<Option<egui::Id>>(focus_id))
            .flatten()
    {
        ctx.memory_mut(|m| m.request_focus(id));
    }
    let token = frame_token(ctx);
    let fresh = palette.handled != Some(token);
    palette.handled = Some(token);
    let mut execute = false;
    let mut cancel = false;
    let invalid = !origin.current(a, ctx);
    if palette.pattern_args.is_some() {
        pattern_args::handle(a, ctx, &mut palette, fresh, invalid);
        a.palette = palette;
        clear(ctx);
        return;
    }
    if let Some(state) = &mut palette.same_measure {
        if invalid || same_measure::show(a, ctx, state, fresh) {
            if invalid {
                a.load_error = Some("Same measure target changed".into());
            }
            origin.restore_focus(ctx);
            palette.origin = None;
            palette.same_measure = None;
            palette.commands.clear();
        }
        a.palette = palette;
        clear(ctx);
        return;
    }
    if palette.insert_event.is_some() {
        if invalid {
            palette.insert_event = None;
        } else {
            let done = insert_event::show(a, ctx, palette.insert_event.as_mut().unwrap(), fresh);
            if done {
                origin.restore_focus(ctx);
                palette.origin = None;
                palette.insert_event = None;
                palette.commands.clear();
            }
            a.palette = palette;
            clear(ctx);
            return;
        }
    }
    if palette.views.is_some() {
        if invalid {
            palette.views = None;
        } else {
            let done = views::show(a, ctx, palette.views.as_mut().unwrap(), fresh);
            if done {
                if let Some(origin) = &palette.origin {
                    origin.restore_focus(ctx);
                }
                palette.origin = None;
                palette.views = None;
                palette.commands.clear();
            }
            a.palette = palette;
            clear(ctx);
            return;
        }
    }
    if invalid {
        palette.commands = commands::candidates(a, ctx);
        palette.origin = Some(Origin::capture_with_focus(a, ctx, origin));
        palette.selected = 0;
        palette.reveal = true;
    }
    let events = ctx.input(|i| i.events.clone());
    let ime_enter = palette.composing || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
    let mut filtered = filter(
        &palette.commands,
        &palette.query,
        &mut palette.search,
        &palette.history,
    );
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
            {
                if !ime_enter
                    && let Some(action) = super::list_navigation::movement(*key, *modifiers)
                {
                    palette.selected = action.apply(palette.selected, filtered.len());
                    palette.reveal = true;
                    continue;
                }
                if !commands::matches(*modifiers, Modifiers::NONE) {
                    continue;
                }
                match key {
                    Key::Escape if !ime_enter => cancel = true,
                    Key::Enter if !*repeat && !ime_enter && !invalid => execute = true,
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
                        key: Key::Enter | Key::Escape,
                        ..
                    }
                ) && !ime_enter)
                && !(matches!(e, egui::Event::Key { key, modifiers, .. }
                    if super::list_navigation::movement(*key, *modifiers).is_some())
                    && !ime_enter)
        })
    });
    if !fresh {
        clear(ctx);
    }
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
    if !cancel && let Some(index) = chosen {
        let id = palette.commands[index].id.clone();
        if pattern_args::choose(a, ctx, &mut palette, &id) {
            a.palette = palette;
            return;
        }
    }
    if !cancel
        && let Some(index) = chosen
        && matches!(
            palette.commands[index].id.as_str(),
            "insert-same-measure" | "generate-same-measures"
        )
    {
        match same_measure::State::new(a, palette.commands[index].id == "generate-same-measures") {
            Ok(state) => palette.same_measure = Some(state),
            Err(error) => a.load_error = Some(error),
        }
        a.palette = palette;
        clear(ctx);
        ctx.request_repaint();
        return;
    }
    if !cancel
        && let Some(state) =
            chosen.and_then(|i| insert_event::State::for_command(&palette.commands[i].id))
    {
        palette.insert_event = Some(state);
        a.palette = palette;
        clear(ctx);
        ctx.request_repaint();
        return;
    }
    if !cancel && chosen.is_some_and(|i| palette.commands[i].id == "browse-views") {
        palette.views = Some(views::State::new(a));
        a.palette = palette;
        clear(ctx);
        ctx.request_repaint();
        return;
    }
    if cancel || chosen.is_some() {
        clear(ctx);
        if let Some(origin) = &palette.origin {
            origin.restore_focus(ctx);
        }
        let completed = cancel
            || chosen.is_some_and(|index| commands::execute(a, ctx, &palette.commands[index]));
        if completed {
            if !cancel && let Some(index) = chosen {
                palette
                    .history
                    .record(&palette.query, &palette.commands[index]);
                if let Err(error) = palette.history.save(a) {
                    a.config.error = Some(error);
                }
            }
            palette.origin = None;
            palette.commands.clear();
            a.palette = palette;
            ctx.request_repaint();
            return;
        }
        palette.commands = commands::candidates(a, ctx);
        palette.selected = 0;
        palette.reveal = true;
        filtered = filter(
            &palette.commands,
            &palette.query,
            &mut palette.search,
            &palette.history,
        );
    }
    let id = egui::Id::new("command-palette");
    let (width, left) = *palette.geometry.get_or_insert_with(|| {
        let style = ctx.style();
        let text_width = palette
            .commands
            .iter()
            .map(|command| row::width(ctx, command))
            .fold(0.0, f32::max);
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
            filtered = filter(
                &palette.commands,
                &palette.query,
                &mut palette.search,
                &palette.history,
            );
        }
        super::list_navigation::position(ui, palette.selected, filtered.len());
        egui::ScrollArea::vertical()
            .id_salt("palette-results")
            .max_height((ctx.screen_rect().bottom() - ui.cursor().top() - 24.0).max(1.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (row, &index) in filtered.iter().enumerate() {
                    let command = &palette.commands[index];
                    let response = row::show(ui, command, row == palette.selected);
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

fn filter(
    commands: &[Command],
    query: &str,
    search: &mut commands::SearchQuery,
    history: &history::History,
) -> Vec<usize> {
    search.update(query);
    let mut result: Vec<_> = commands
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.matches_query(search).then_some(i))
        .collect();
    result.sort_by_key(|&i| history.rank(query, &commands[i]));
    result
}
