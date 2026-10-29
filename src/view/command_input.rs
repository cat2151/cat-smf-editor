//! Operation input, independent of command search and history.
use super::{
    commands,
    modal::{self, Origin},
};
use crate::app::App;
use eframe::egui::{self, Key, Modifiers};
use modal::clear;
pub(crate) mod guide_edit;
mod insert_event;
mod pattern_args;
mod same_measure;
mod views;
pub(crate) use views::preview::{
    active as preview_active, id as preview_id, window as preview_window,
};

#[derive(Default)]
pub struct State {
    origin: Option<Origin>,
    handled: Option<(egui::ViewportId, u64)>,
    views: Option<views::State>,
    insert_event: Option<insert_event::State>,
    same_measure: Option<same_measure::State>,
    pattern_args: Option<pattern_args::State>,
}
impl State {
    pub(crate) fn views_open(&self) -> bool {
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

pub(crate) fn begin(a: &mut App, ctx: &egui::Context) -> Option<egui::InputState> {
    if ctx.viewport_id() == egui::ViewportId::ROOT
        && a.command_input
            .origin
            .as_ref()
            .is_some_and(|o| o.viewport != egui::ViewportId::ROOT)
    {
        // The root renders the dialog. Preserve the initiating target and focus;
        // changing viewports must not silently rebind an externally changed target.
        a.command_input.origin.as_mut().unwrap().viewport = egui::ViewportId::ROOT;
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    let Some(origin) = &a.command_input.origin else {
        if a.command_input.handled == Some(modal::frame_token(ctx)) {
            clear(ctx);
        }
        return None;
    };
    Some(modal::suspend(ctx, origin))
}
pub(crate) fn end(a: &mut App, ctx: &egui::Context, input: Option<egui::InputState>) {
    let Some(input) = input else {
        return;
    };
    let mut state = std::mem::take(&mut a.command_input);
    let Some(origin) = &state.origin else {
        a.command_input = state;
        return;
    };
    if origin.viewport != ctx.viewport_id() {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("modal-suspended"), false));
        a.command_input = state;
        clear(ctx);
        return;
    }
    let valid = origin.target_current(a);
    if valid {
        origin.restore_cursor(a);
    }
    modal::resume(ctx, input);
    let token = modal::frame_token(ctx);
    let fresh = state.handled != Some(token);
    state.handled = Some(token);
    let done = if !valid {
        true
    } else if let Some(s) = &mut state.pattern_args {
        pattern_args::show(a, ctx, s, fresh)
    } else if let Some(s) = &mut state.insert_event {
        insert_event::show(a, ctx, s, fresh)
    } else if let Some(s) = &mut state.same_measure {
        same_measure::show(a, ctx, s, fresh)
    } else if let Some(s) = &mut state.views {
        views::show(a, ctx, s, fresh)
    } else {
        true
    };
    if done {
        origin.restore_focus(ctx);
        state = State {
            handled: Some(token),
            ..Default::default()
        };
    }
    a.command_input = state;
    clear(ctx);
}

pub(crate) fn start(
    a: &mut App,
    ctx: &egui::Context,
    operation: commands::Operation,
) -> Result<commands::Execution, String> {
    use commands::{Execution, Operation};
    let mut state = State::default();
    match operation {
        Operation::PatternWork => {
            super::patterns::matrix::enter(a, ctx)?;
            return Ok(Execution::Completed);
        }
        Operation::Pattern {
            action,
            pattern,
            value,
            container,
        } => {
            if action == super::patterns::Action::Create {
                if pattern.is_some() || value.is_some() {
                    return Err("Add pattern takes no existing pattern arguments".into());
                }
                if let Some(id) = container {
                    super::patterns::library::create_for_container(a, id)?;
                    super::patterns::library::input(a, ctx);
                } else {
                    super::patterns::library::begin_add(a, ctx)?;
                }
                return Ok(Execution::Completed);
            }
            if action == super::patterns::Action::AuditionTempo && pattern.is_some() {
                return Err("Audition tempo has no pattern target".into());
            }
            let mut input = pattern_args::State::new(a, action)?;
            input.preset(value, container)?;
            if input.supplied_tempo(a)? {
                return Ok(Execution::Completed);
            }
            if let Some(id) = pattern
                && input.apply(a, Some(id))?
            {
                return Ok(Execution::Completed);
            }
            state.pattern_args = Some(input);
        }
        Operation::InsertEvent { kind, text } => {
            if kind == Some(cat_smf_editor::edit::meta_input::Kind::Chord)
                && !guide_edit::on_guide(a)
            {
                return Err("Select the chord guide track".into());
            }
            if let (Some(kind), Some(text)) = (kind, text.as_ref()) {
                insert_event::apply(a, kind, text)?;
                return Ok(Execution::Completed);
            }
            state.insert_event = Some(insert_event::State {
                tick: crate::play::playback::location(a).0,
                kind,
                pending_text: kind.is_none().then(|| text.clone()).flatten(),
                query: if kind.is_some() {
                    text.unwrap_or_default()
                } else {
                    String::new()
                },
                ..Default::default()
            });
        }
        Operation::SameMeasure { batch, number } => {
            let mut input = same_measure::State::new(a, batch)?;
            if let Some(number) = number {
                input.set_number(number);
                input.apply(a)?;
                return Ok(Execution::Completed);
            }
            state.same_measure = Some(input);
        }
        Operation::Views => state.views = Some(views::State::new(a)),
        Operation::GuideEvent { moving } => {
            state.insert_event =
                Some(guide_edit::state(a, moving).ok_or("No guide event selected")?);
        }
    }
    state.origin = Some(Origin::capture(a, ctx));
    state.handled = Some(modal::frame_token(ctx));
    a.command_input = state;
    modal::mark_owned(ctx);
    clear(ctx);
    ctx.request_repaint();
    Ok(Execution::Input)
}
