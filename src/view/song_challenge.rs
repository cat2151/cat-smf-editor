use crate::app::App;
use eframe::egui::{self, Color32, Key};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) enum State {
    #[default]
    Off,
    Duration(DurationInput),
    Running {
        start: Instant,
        limit: Duration,
    },
    Finished {
        shown: Option<Instant>,
        error: Option<String>,
    },
}

pub(crate) struct DurationInput {
    minutes: String,
    focus: bool,
    invalid: bool,
}

impl State {
    pub(crate) fn ask_duration(&mut self) {
        *self = Self::Duration(DurationInput {
            minutes: "15".into(),
            focus: true,
            invalid: false,
        });
    }

    pub(crate) fn modal_open(&self) -> bool {
        matches!(self, Self::Duration(_) | Self::Finished { .. })
    }
}

pub(super) fn advance(app: &mut App, ctx: &egui::Context) {
    let State::Running { start, limit } = app.song_challenge else {
        return;
    };
    let remaining = limit.saturating_sub(start.elapsed());
    if remaining.is_zero() {
        let error = app.remember_loaded().and_then(|_| app.saved.save()).err();
        if let Some(error) = &error {
            app.saved.auto_save_failed(error.clone());
        }
        app.song_challenge = State::Finished { shown: None, error };
        return;
    }
    let fraction = remaining.subsec_nanos();
    let seconds = remaining.as_secs() + u64::from(fraction != 0);
    countdown(ctx, &format!("あと{}分{}秒", seconds / 60, seconds % 60));
    ctx.request_repaint_after(if fraction == 0 {
        Duration::from_secs(1)
    } else {
        Duration::from_nanos(fraction.into())
    });
}

fn countdown(ctx: &egui::Context, text: &str) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("song-challenge-countdown"),
    ));
    let galley = painter.layout_no_wrap(
        text.into(),
        egui::FontId::proportional(48.0),
        Color32::WHITE,
    );
    let position = ctx.screen_rect().right_bottom() - galley.size() - egui::vec2(16.0, 16.0);
    for offset in [
        (-2.0, -2.0),
        (0.0, -2.0),
        (2.0, -2.0),
        (-2.0, 0.0),
        (2.0, 0.0),
        (-2.0, 2.0),
        (0.0, 2.0),
        (2.0, 2.0),
    ] {
        painter.galley_with_override_text_color(
            position + egui::vec2(offset.0, offset.1),
            galley.clone(),
            Color32::BLACK,
        );
    }
    painter.galley(position, galley, Color32::WHITE);
}

const CLOSED: &str = "song-challenge-closed-frame";

pub(crate) fn closed_this_frame(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp(egui::Id::new(CLOSED))) == Some(super::modal::frame_token(ctx))
}

pub(super) fn dialog(app: &mut App, ctx: &egui::Context) {
    let next = match &mut app.song_challenge {
        State::Duration(input) => duration_dialog(ctx, input),
        State::Finished { shown, error } => finished_dialog(ctx, shown, error.as_deref()),
        _ => None,
    };
    if let Some(next) = next {
        app.song_challenge = next;
        let frame = super::modal::frame_token(ctx);
        ctx.data_mut(|data| data.insert_temp(egui::Id::new(CLOSED), frame));
        ctx.request_repaint();
    }
}

fn duration(minutes: &str) -> Option<Duration> {
    let minutes = minutes.trim();
    if !minutes.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let seconds = minutes.parse::<u64>().ok()?.checked_mul(60)?;
    (seconds > 0).then(|| Duration::from_secs(seconds))
}

fn duration_dialog(ctx: &egui::Context, input: &mut DurationInput) -> Option<State> {
    let field = egui::Id::new("song-challenge-minutes");
    let mut confirm = take_key(ctx, Key::Enter);
    let mut cancel = take_key(ctx, Key::Escape);
    super::presentation::modal(ctx, egui::Id::new("song-challenge-duration")).show(ctx, |ui| {
        ui.heading("15-Minute Song Challenge");
        ui.label("制限時間（分）");
        let mut output = egui::TextEdit::singleline(&mut input.minutes)
            .id(field)
            .desired_width(180.0)
            .show(ui);
        if input.focus {
            output.response.request_focus();
            output
                .state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(input.minutes.chars().count()),
                )));
            output.state.store(ctx, field);
            input.focus = false;
        }
        if output.response.changed() {
            input.invalid = false;
        }
        if input.invalid {
            ui.colored_label(Color32::LIGHT_RED, "1以上の整数を入力してください");
        }
        ui.horizontal(|ui| {
            confirm |= ui.button("開始 · Enter").clicked();
            cancel |= ui.button("キャンセル · Esc").clicked();
        });
    });
    if cancel {
        ctx.memory_mut(|memory| memory.surrender_focus(field));
        return Some(State::Off);
    }
    if confirm {
        if let Some(limit) = duration(&input.minutes) {
            ctx.memory_mut(|memory| memory.surrender_focus(field));
            return Some(State::Running {
                start: Instant::now(),
                limit,
            });
        }
        input.invalid = true;
        ctx.memory_mut(|memory| memory.request_focus(field));
        ctx.request_repaint();
    }
    None
}

fn finished_dialog(
    ctx: &egui::Context,
    shown: &mut Option<Instant>,
    error: Option<&str>,
) -> Option<State> {
    let wait = Duration::from_secs(1)
        .saturating_sub(shown.as_ref().map_or(Duration::ZERO, Instant::elapsed));
    let ready = wait.is_zero();
    let mut confirm = take_key(ctx, Key::Enter) && ready;
    let id = egui::Id::new("song-challenge-finished");
    super::presentation::modal(ctx, id)
        .area(egui::Modal::default_area(id).fade_in(false))
        .show(ctx, |ui| {
            if ui.is_visible() {
                shown.get_or_insert_with(Instant::now);
            }
            ui.set_width(360.0);
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.add(
                    egui::Label::new(egui::RichText::new("Time's Up!").size(54.0).strong())
                        .selectable(false),
                );
                if let Some(error) = error {
                    ui.add(
                        egui::Label::new(egui::RichText::new(error).color(Color32::LIGHT_RED))
                            .selectable(false),
                    );
                }
                ui.add_space(16.0);
                ui.allocate_ui(egui::vec2(100.0, 32.0), |ui| {
                    if ready {
                        confirm |= ui.button("OK").on_hover_text("Enter").clicked();
                    }
                });
                ui.add_space(24.0);
            });
        });
    if !ready {
        ctx.request_repaint_after(wait);
    }
    confirm.then_some(State::Off)
}

fn take_key(ctx: &egui::Context, wanted: Key) -> bool {
    ctx.input_mut(|input| {
        let mut pressed = false;
        input.events.retain(|event| {
            if let egui::Event::Key {
                key,
                pressed: down,
                repeat,
                modifiers,
                ..
            } = event
                && *key == wanted
                && *modifiers == egui::Modifiers::NONE
            {
                pressed |= *down && !*repeat;
                return false;
            }
            true
        });
        pressed
    })
}

#[cfg(test)]
mod tests;
