//! Reading and commanding native windows other than the current one. egui answers for any
//! viewport id by creating its state, and a frame hands every such state to the backend, which
//! opens a native window for an id the frame did not show. So these reach only the windows that
//! `Native::show` showed in this frame or the previous one.
use super::{native_key, NativeState};
use eframe::egui;

/// An editor opened from a bound surface hands the keys back once modal windows restore.
pub(crate) fn focus_after_restoration(ctx: &egui::Context, id: egui::ViewportId) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("presentation-return-focus"), id));
}

pub(super) fn restored_focus(
    ctx: &egui::Context,
    target: Option<egui::ViewportId>,
) -> Option<egui::ViewportId> {
    ctx.data_mut(|d| {
        let previous = d.get_temp_mut_or_default::<Option<egui::ViewportId>>(egui::Id::new(
            "presentation-host",
        ));
        // Windows may activate an owned window on Visible(true); finish at the chosen host.
        let focus = (*previous != target)
            .then(|| target.or(*previous))
            .flatten();
        *previous = target;
        if target.is_none() {
            d.remove_temp::<egui::ViewportId>(egui::Id::new("presentation-return-focus"))
                .or(focus)
        } else {
            focus
        }
    })
}

impl NativeState {
    /// Starts root pass `pass`: the windows shown in the previous pass stay reachable.
    pub(super) fn begin(&mut self, pass: u64) {
        if self.pass == pass {
            return;
        }
        let shown = self.seen.drain(..).map(|(id, ..)| id);
        self.previous = if self.pass + 1 == pass {
            shown.collect()
        } else {
            Vec::new()
        };
        self.pass = pass;
        self.pending.clear();
    }
}

/// Whether the backend has window `id` in this frame: it was shown in this frame or the last.
fn reachable(ctx: &egui::Context, id: egui::ViewportId) -> bool {
    if id == egui::ViewportId::ROOT {
        return true;
    }
    let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
    ctx.data(|d| d.get_temp::<NativeState>(native_key()))
        .is_some_and(|s| {
            let seen = s.seen.iter().any(|(shown, ..)| *shown == id);
            if s.pass == pass {
                seen || s.previous.contains(&id)
            } else {
                s.pass + 1 == pass && seen
            }
        })
}

/// The input of window `id`; `None` while the backend does not have it.
pub(crate) fn input_for<R>(
    ctx: &egui::Context,
    id: egui::ViewportId,
    reader: impl FnOnce(&egui::InputState) -> R,
) -> Option<R> {
    reachable(ctx, id).then(|| ctx.input_for(id, reader))
}

/// Sends `command` to window `id`. A window the backend does not have yet gets it at the end
/// of the frame if the frame showed it; otherwise the command is dropped.
pub(crate) fn send_to(ctx: &egui::Context, id: egui::ViewportId, command: egui::ViewportCommand) {
    if reachable(ctx, id) {
        ctx.send_viewport_cmd_to(id, command);
        return;
    }
    let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
    ctx.data_mut(|d| {
        let state = d.get_temp_mut_or_default::<NativeState>(native_key());
        state.begin(pass);
        state.pending.push((id, command));
    });
}

/// Sends `Visible(visible)` to window `id` when the value differs from the last one it got, or
/// once per frame while the window was not shown in the previous frame (the backend may have made
/// a new native window). egui repaints a window for every command it is sent, so sending an
/// unchanged value every frame would keep redrawing.
pub(super) fn send_visible(ctx: &egui::Context, id: egui::ViewportId, visible: bool) {
    let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
    let send = ctx.data_mut(|d| {
        let state = d.get_temp_mut_or_default::<NativeState>(native_key());
        state.begin(pass);
        let new_window = !state.previous.contains(&id);
        match state.sent.iter_mut().find(|(sent, ..)| *sent == id) {
            Some(last) => {
                let send = last.1 != visible || (new_window && last.2 != pass);
                if send {
                    *last = (id, visible, pass);
                }
                send
            }
            None => {
                state.sent.push((id, visible, pass));
                true
            }
        }
    });
    if send {
        ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Visible(visible));
    }
}

/// Delivers the commands that waited for a window this frame showed.
pub(super) fn flush(ctx: &egui::Context) {
    let pending = ctx.data_mut(|d| {
        std::mem::take(
            &mut d
                .get_temp_mut_or_default::<NativeState>(native_key())
                .pending,
        )
    });
    for (id, command) in pending {
        if reachable(ctx, id) {
            ctx.send_viewport_cmd_to(id, command);
        }
    }
}

/// Counts window `id` as shown in the current frame, as `Native::show` does.
#[cfg(test)]
pub(crate) fn shown(ctx: &egui::Context, id: egui::ViewportId) {
    let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
    ctx.data_mut(|d| {
        let state = d.get_temp_mut_or_default::<NativeState>(native_key());
        state.begin(pass);
        state.seen.push((id, super::Native::Workspace, true));
    });
}
