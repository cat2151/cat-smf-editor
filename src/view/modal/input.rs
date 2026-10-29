use eframe::egui::{self, Modifiers};
pub(crate) fn frame_token(ctx: &egui::Context) -> (egui::ViewportId, u64) {
    (
        ctx.viewport_id(),
        ctx.cumulative_pass_nr()
            .saturating_sub(ctx.output(|o| o.num_completed_passes as u64)),
    )
}
pub(crate) fn suspended(ctx: &egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(egui::Id::new("modal-suspended"))
            .unwrap_or(false)
    })
}

pub(crate) fn clear(ctx: &egui::Context) {
    let suspend_pointer = suspended(ctx);
    ctx.input_mut(|i| {
        i.events.clear();
        i.keys_down.clear();
        // Background widgets receive an inert pointer during suspension. After
        // the modal has consumed the real input, keep its held-button state so
        // a release next frame can complete the click.
        if suspend_pointer {
            i.pointer = Default::default();
        }
        i.modifiers = Modifiers::NONE;
        i.raw.events.clear();
        i.raw.dropped_files.clear();
        i.raw_scroll_delta = egui::Vec2::ZERO;
        i.smooth_scroll_delta = egui::Vec2::ZERO;
    });
}
