//! Fixed, pitch-aligned keys. A gesture can cross keys and split panes.
use super::*;

pub(super) const WIDTH: f32 = 48.0;

#[derive(Default)]
pub(super) struct State {
    held: bool,
    sounding: Option<(usize, u8)>,
}

pub(super) fn show(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    clip: egui::Rect,
    layout: &Layout,
    state: &mut State,
    slot: usize,
) -> Option<u8> {
    let response = ui.interact(
        rect.intersect(clip),
        crate::view::palette::preview_id(ui.ctx(), egui::Id::new(("piano-roll-keyboard", slot))),
        egui::Sense::click_and_drag(),
    );
    if ui.input(|i| i.pointer.primary_pressed()) && response.is_pointer_button_down_on() {
        state.held = true;
    }
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let row_height = layout.size.y / f32::from(layout.high - layout.low + 1);
    let pitch = pointer
        .filter(|p| {
            state.held
                && ui.is_enabled()
                && rect.intersect(clip).contains(*p)
                && p.y < rect.bottom()
        })
        .map(|p| layout.high - ((p.y - rect.top()) / row_height).floor() as u8);
    for key in layout.low..=layout.high {
        let black = matches!(key % 12, 1 | 3 | 6 | 8 | 10);
        let key_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left(),
                rect.top() + f32::from(layout.high - key) * row_height,
            ),
            egui::vec2(WIDTH, row_height),
        );
        ui.painter().rect(
            key_rect,
            0.0,
            if pitch == Some(key) && ui.input(|i| i.pointer.primary_down()) {
                crate::view::pitch_keyboard::roll_color(key, None)
            } else if black {
                Color32::from_gray(25)
            } else {
                Color32::from_gray(205)
            },
            egui::Stroke::new(1.0_f32, Color32::from_gray(65)),
            egui::StrokeKind::Inside,
        );
        if black && !(pitch == Some(key) && ui.input(|i| i.pointer.primary_down())) {
            ui.painter().rect_filled(
                key_rect.with_max_x(rect.left() + WIDTH * 0.65).shrink(1.0),
                0.0,
                Color32::BLACK,
            );
        }
    }
    pitch
}

pub(super) fn finish(app: &mut App, ctx: &egui::Context, target: Option<(usize, u8)>) {
    // An explicit transport stop/start can cancel the previous held gesture.
    if app.piano_roll.keyboard.sounding.is_some() && !app.audition.key_active() {
        app.piano_roll.keyboard = State::default();
        return;
    }
    let down = ctx.input(|i| {
        i.focused
            && !i
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(false)))
            && i.pointer.primary_down()
    });
    let state = &mut app.piano_roll.keyboard;
    if !down {
        state.held = false;
    }
    let target = target.filter(|_| down && state.held);
    if state.sounding == target {
        return;
    }
    let previous = state.sounding.take();
    if let Some((track, pitch)) = target {
        let take = crate::play::sung::audition::key(app, track, pitch);
        let loaded = app.loaded.as_ref().unwrap();
        if app
            .audition
            .play_key(track, &loaded.patches[track], pitch, take)
        {
            app.piano_roll.keyboard.sounding = target;
        }
    } else if previous.is_some() {
        app.audition.stop_key();
    }
}

pub(super) fn cancel(app: &mut App, ctx: &egui::Context) {
    app.piano_roll.keyboard.held = false;
    finish(app, ctx, None);
}

#[cfg(test)]
pub(super) fn sounding(app: &App) -> Option<(usize, u8)> {
    app.piano_roll.keyboard.sounding
}
