use super::*;
pub(super) struct Origin {
    pub viewport: egui::ViewportId,
    target: commands::Target,
    screen: Screen,
    focus: Option<egui::Id>,
    text: Option<egui::text_edit::TextEditState>,
    song: u64,
    generation: u64,
    row: Option<usize>,
    event: Option<usize>,
    position: Option<cat_smf_editor::score::musical_time::Position>,
    tick: Option<u64>,
}
impl Origin {
    pub fn capture(a: &App, ctx: &egui::Context) -> Self {
        let focus = ctx.memory(|m| m.focused());
        Self {
            viewport: ctx.viewport_id(),
            target: commands::Target::of(a),
            screen: commands::owner(a, ctx),
            focus,
            text: focus.and_then(|id| egui::text_edit::TextEditState::load(ctx, id)),
            song: a.song_id,
            generation: a.generation,
            row: a.cursor_row,
            event: a.selected_event,
            position: a.cursor_position.clone(),
            tick: a.track_view.cursor_tick,
        }
    }
    pub fn capture_with_focus(a: &App, ctx: &egui::Context, old: &Self) -> Self {
        let mut new = Self::capture(a, ctx);
        new.focus = old.focus;
        new.text = old.text.clone();
        new
    }
    pub fn restore_focus(&self, ctx: &egui::Context) {
        if let Some(id) = self.focus {
            if let Some(text) = &self.text {
                text.clone().store(ctx, id);
            }
            ctx.memory_mut(|m| m.request_focus(id));
        } else if let Some(id) = ctx.memory(|m| m.focused()) {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
    }
    pub fn restore_cursor(&self, a: &mut App) {
        if a.song_id == self.song && a.generation == self.generation {
            a.cursor_row = self.row;
            a.selected_event = self.event;
            a.cursor_position = self.position.clone();
            a.track_view.cursor_tick = self.tick;
        }
    }
    pub fn current(&self, a: &App, ctx: &egui::Context) -> bool {
        self.target == commands::Target::of(a) && self.screen == commands::owner(a, ctx)
    }
}
