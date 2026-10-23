//! Persistent parent columns and selection-driven child columns, without scrolling.
use super::*;

pub(super) fn show(
    app: &App,
    ctx: &egui::Context,
    state: &mut State,
    fresh: bool,
) -> Option<Target> {
    let mut path = state.parents.clone();
    path.push((state.level, state.selected));
    let mut target = state.entries.get(state.selected).map(|e| e.target);
    while let Some(Target::Level(level)) = target {
        let entries = catalog::entries(app, level);
        target = entries.first().map(|e| e.target);
        path.push((level, 0));
    }
    let screen = ctx.screen_rect();
    let backdrop = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("views-backdrop"));
    ctx.move_to_top(backdrop);
    ctx.layer_painter(backdrop)
        .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(160));
    let width = (screen.width() * 0.15).clamp(120.0, 240.0);
    let mut left = screen.left();
    let pointer_moved = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
    for (depth, &(level, selected)) in path.iter().enumerate() {
        let entries = catalog::entries(app, level);
        let title = match level {
            Level::Tracks => "Views".to_owned(),
            Level::Track(track) => app.loaded.as_ref().unwrap().track_titles[track].clone(),
            Level::Layer(track, layer) => crate::view::parameter_editor::layer_label(
                &app.loaded.as_ref().unwrap().patches[track],
                layer,
            ),
        };
        let shown = egui::Area::new(egui::Id::new(("views-list", depth)))
            .order(egui::Order::Tooltip)
            .fixed_pos(egui::pos2(left, screen.top() + 8.0))
            .constrain(false)
            .show(ctx, |ui| {
                let mut frame = egui::Frame::popup(ui.style());
                if depth == state.parents.len() {
                    frame.stroke = ui.visuals().selection.stroke;
                }
                frame.show(ui, |ui| {
                    ui.set_width(width);
                    ui.add(egui::Label::new(title).truncate());
                    if state.help && depth == 0 {
                        egui::Grid::new("view-palette-help").show(ui, crate::view::help::view_rows);
                        return;
                    }
                    if state.help {
                        return;
                    }
                    let row_height = ui.spacing().interact_size.y.max(
                        ui.text_style_height(&egui::TextStyle::Button)
                            + 6.0
                            + 2.0 * ui.spacing().button_padding.y,
                    ) + ui.spacing().item_spacing.y;
                    let rows = ((screen.bottom() - ui.cursor().top() - 16.0) / row_height)
                        .floor()
                        .max(1.0) as usize;
                    ui.horizontal_top(|ui| {
                        for (column, chunk) in entries.chunks(rows).enumerate() {
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                for (row, entry) in chunk.iter().enumerate() {
                                    let index = column * rows + row;
                                    let response = super::super::row::action(
                                        ui,
                                        entry.label.clone(),
                                        String::new(),
                                        index == selected,
                                        false,
                                    );
                                    if fresh
                                        && (response.clicked()
                                            || (pointer_moved && response.hovered()))
                                    {
                                        state.parents = path[..depth].to_vec();
                                        state.level = level;
                                        state.entries = catalog::entries(app, level);
                                        state.selected = index;
                                        ctx.request_repaint();
                                    }
                                }
                            });
                        }
                    });
                });
            });
        left = shown.response.rect.right() + 4.0;
    }
    target
}
