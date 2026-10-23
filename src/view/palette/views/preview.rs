//! Read-only rendering with separate viewport state; never open a plugin or start playback.
use super::*;

#[derive(Default)]
pub(super) struct State {
    view: Option<View>,
    piano: crate::view::piano_roll::State,
    tempo: crate::view::tempo_curve::State,
    parameter: crate::view::parameter_editor::State,
    staff: crate::view::staff::State,
}

pub(crate) fn active(ctx: &egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(egui::Id::new("views-preview"))
            .unwrap_or(false)
    })
}

pub(crate) fn id(ctx: &egui::Context, id: egui::Id) -> egui::Id {
    if active(ctx) {
        id.with("views-preview")
    } else {
        id
    }
}

pub(crate) fn window(
    ctx: &egui::Context,
    title: impl Into<egui::WidgetText>,
) -> egui::Window<'static> {
    let screen = ctx.screen_rect();
    let size = egui::vec2(
        (screen.width() * 0.62).min(1000.0),
        (screen.height() * 0.65).min(620.0),
    );
    egui::Window::new(title)
        .id(egui::Id::new("views-preview-window"))
        .order(egui::Order::Foreground)
        .enabled(false)
        .collapsible(false)
        .title_bar(false)
        .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
        .resizable(false)
        .movable(false)
        .fixed_rect(egui::Rect::from_center_size(screen.center(), size))
}

pub(super) fn show(app: &mut App, ctx: &egui::Context, state: &mut State, view: View) {
    if state.view != Some(view) {
        *state = State {
            view: Some(view),
            ..Default::default()
        };
        state.piano.open_at_cursor();
        state.tempo.open_at_cursor();
    }
    // Drawing gets inert input, its own editor state and track. The real owner is restored
    // before the palette handles another frame or an Enter confirmation.
    let input = ctx.input(Clone::clone);
    super::super::clear(ctx);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("views-preview"), true));
    let track = app.track;
    let tick = crate::play::playback::location(app).0;
    let track_view = app.track_view.clone();
    let note_mode = app.note_mode;
    let reset_scroll = app.reset_scroll;
    let reveal_cursor = app.reveal_cursor;
    let piano_open = app.piano_roll.open;
    let tempo_open = app.tempo_curve.open;
    let follow = app.playback.follow;
    app.track_view.cursor_tick = Some(tick);
    app.track = match view {
        View::Score(t)
        | View::Piano(t)
        | View::Instruments(t)
        | View::Effects(t)
        | View::Macros(t, _)
        | View::Variation(t, _) => t,
        _ => track,
    };
    if app.track != track
        && let Some(tracks) = &mut app.track_view.split
    {
        tracks[app.track_view.active] = app.track;
    }
    std::mem::swap(&mut app.piano_roll, &mut state.piano);
    std::mem::swap(&mut app.tempo_curve, &mut state.tempo);
    std::mem::swap(&mut app.parameter_editor, &mut state.parameter);
    std::mem::swap(&mut app.staff, &mut state.staff);
    match view {
        View::Piano(_) => crate::view::piano_roll::show(app, ctx),
        View::Tempo => crate::view::tempo_curve::show(app, ctx),
        View::Parameter(lane) => {
            crate::view::parameter_editor::show_preview(app, ctx, Some(lane), None);
        }
        View::Variation(_, layer) => {
            crate::view::parameter_editor::show_preview(app, ctx, None, Some(layer));
        }
        View::Instruments(_) => crate::view::layers::show_preview(app, ctx),
        View::Effects(_) => crate::view::effects::show_preview(app, ctx),
        View::Macros(_, layer) => crate::view::macro_editor::show_preview(app, ctx, layer),
        View::Mixer => {
            let saved = (
                app.playback.mixer_open,
                app.playback.mixer_focus,
                app.playback.mixer_track,
                app.playback.mixer_master,
            );
            crate::play::mixer::open(app);
            crate::play::mixer::show(app, ctx);
            (
                app.playback.mixer_open,
                app.playback.mixer_focus,
                app.playback.mixer_track,
                app.playback.mixer_master,
            ) = saved;
        }
        View::Score(_) | View::DoubleScore => {
            if view == View::DoubleScore {
                app.note_mode = true;
                crate::cursor::split::split_on(app);
            } else {
                app.track_view.split = None;
            }
            app.piano_roll.open = false;
            app.tempo_curve.open = false;
            window(ctx, "Preview").show(ctx, |ui| {
                crate::view::score_panes::show(app, ui, &mut false, &mut None);
            });
        }
    }
    ctx.move_to_top(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("views-preview-window"),
    ));
    std::mem::swap(&mut app.piano_roll, &mut state.piano);
    std::mem::swap(&mut app.tempo_curve, &mut state.tempo);
    std::mem::swap(&mut app.parameter_editor, &mut state.parameter);
    std::mem::swap(&mut app.staff, &mut state.staff);
    app.track = track;
    app.track_view = track_view;
    app.note_mode = note_mode;
    app.reset_scroll = reset_scroll;
    app.reveal_cursor = reveal_cursor;
    app.piano_roll.open = piano_open;
    app.tempo_curve.open = tempo_open;
    app.playback.follow = follow;
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("views-preview"), false));
    ctx.input_mut(|i| *i = input);
}
