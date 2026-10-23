use super::*;
pub fn show(app: &mut App, ctx: &egui::Context) {
    let preview = crate::view::palette::preview_active(ctx);
    if !preview {
        refresh(app);
    }
    let Some(s) = &mut app.macro_editor.session else {
        return;
    };
    let help_open = app.macro_editor.help;
    let mut open = true;
    let mut confirm = false;
    let mut cancel = false;
    let mut step_anchor = None;
    let window = if preview {
        crate::view::palette::preview_window(ctx, s.title.clone())
    } else {
        egui::Window::new(&s.title)
            .id(egui::Id::new("macro-editor"))
            .collapsible(false)
            .resizable(false)
    };
    window.open(&mut open).show(ctx, |ui| {
        if help_open {
            ui.disable();
        }
        egui::Grid::new("macro-rows")
            .num_columns(2)
            .max_col_width(620.0)
            .show(ui, |ui| {
                for (i, row) in s.rows.iter().enumerate() {
                    if ui
                        .selectable_label(i == s.selected, &row.curve.name)
                        .clicked()
                    {
                        s.selected = i;
                    }
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(280.0, 26.0),
                        egui::Sense::click_and_drag(),
                    );
                    if (response.dragged() || response.clicked())
                        && let Some(p) = response.interact_pointer_pos()
                    {
                        row.set(f64::from((p.x - rect.left()) / rect.width()));
                        s.selected = i;
                    }
                    if i == s.selected {
                        step_anchor = Some(egui::pos2(
                            rect.left() + rect.width() * row.get() as f32,
                            rect.top(),
                        ));
                    }
                    let color = if i == s.selected {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().widgets.inactive.bg_fill
                    };
                    ui.painter()
                        .rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
                    let mut bar = rect;
                    bar.max.x = bar.min.x + bar.width() * row.get() as f32;
                    if row.bipolar {
                        let zero = rect.center().x;
                        let end = bar.max.x;
                        bar.min.x = zero.min(end);
                        bar.max.x = zero.max(end);
                        ui.painter().line_segment(
                            [
                                egui::pos2(zero, rect.top()),
                                egui::pos2(zero, rect.bottom()),
                            ],
                            egui::Stroke::new(1.0_f32, ui.visuals().text_color()),
                        );
                    }
                    ui.painter().rect_filled(bar, 3.0, color);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Macro {:+.3}  ·  pos {:.3}", row.signal(), row.get()),
                        egui::FontId::monospace(14.0),
                        ui.visuals().text_color(),
                    );
                    ui.end_row();
                    ui.label("");
                    let text = row.readout();
                    if row.readback.as_ref().is_some_and(|(raw, _)| {
                        row.curve.domain().is_ok_and(|d| d.value(*raw).is_err())
                    }) {
                        ui.colored_label(ui.visuals().error_fg_color, text);
                    } else {
                        ui.small(text);
                    }
                    ui.end_row();
                }
            });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("Variation");
            for i in 0..s.variations.len().max(1) {
                if ui
                    .selectable_label(i == s.variation, (i + 1).to_string())
                    .clicked()
                {
                    s.switch_variation(i);
                }
            }
            if ui
                .add_enabled(s.variations.len() < 128, egui::Button::new("複製 [D]"))
                .clicked()
            {
                s.duplicate_variation();
            }
        });
        ui.horizontal(|ui| {
            confirm = ui.button("確定・終了 [Enter]").clicked();
            cancel = ui.button("取消 [Esc]").clicked();
        });
        if let Some(error) = &app.macro_editor.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    });
    step_toast(ctx, &mut s.keyboard, step_anchor);
    if app.macro_editor.help {
        let mut help = true;
        egui::Window::new("Macro edit — ヘルプ")
            .id(egui::Id::new("macro-help-overlay"))
            .open(&mut help)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("macro-help").show(ui, |ui| {
                    crate::view::help::key_groups(
                        ui,
                        &[(
                            egui::Color32::from_rgb(0x66, 0xd9, 0xef),
                            &[
                                ("j k", "Macro選択"),
                                ("h l", "値"),
                                ("1–9… + h j k l", "反復回数"),
                                ("[ / ]", "変化量 ÷2 / ×2"),
                                ("Home / End", "最小 / 最大"),
                                ("PageUp / PageDown", "Variation切替"),
                                ("r", "全Macroランダム・Variation範囲"),
                                ("Shift+R", "全Macroランダム・全範囲"),
                            ],
                        )],
                    );
                });
                crate::view::help::close_guide(ui);
            });
        app.macro_editor.help = help;
    }
    if !open || cancel {
        finish(app, false);
    } else if confirm {
        finish(app, true);
    }
}

fn step_toast(ctx: &egui::Context, keyboard: &mut keyboard::State, anchor: Option<egui::Pos2>) {
    let Some(until) = keyboard.step_toast_until else {
        return;
    };
    let remaining = until - ctx.input(|i| i.time);
    if remaining <= 0.0 {
        keyboard.step_toast_until = None;
        return;
    }
    let Some(anchor) = anchor else {
        return;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("macro-step-toast"),
    ));
    let accent = egui::Color32::from_rgb(0x66, 0xd9, 0xef);
    let text = painter.layout_no_wrap(
        format!("変化量 {}", keyboard.step),
        egui::FontId::monospace(14.0),
        egui::Color32::from_rgb(0xdf, 0xf7, 0xfb),
    );
    let padding = egui::vec2(10.0, 6.0);
    let size = text.size() + padding * 2.0;
    let screen = ctx.screen_rect().shrink(6.0);
    let position = egui::pos2(
        (anchor.x + 8.0).clamp(screen.left(), (screen.right() - size.x).max(screen.left())),
        (anchor.y - 8.0 - size.y).clamp(screen.top(), (screen.bottom() - size.y).max(screen.top())),
    );
    let badge = egui::Rect::from_min_size(position, size);
    painter.line_segment(
        [anchor, egui::pos2(badge.left() + 6.0, badge.bottom())],
        egui::Stroke::new(1.5_f32, accent),
    );
    painter.circle_filled(anchor, 2.5, accent);
    painter.rect_filled(
        badge.translate(egui::vec2(0.0, 3.0)),
        6.0,
        egui::Color32::from_black_alpha(80),
    );
    painter.rect(
        badge,
        6.0,
        egui::Color32::from_rgb(0x18, 0x2b, 0x30),
        egui::Stroke::new(1.0_f32, accent),
        egui::StrokeKind::Inside,
    );
    painter.galley(badge.min + padding, text, egui::Color32::WHITE);
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
}

impl Row {
    pub(super) fn signal(&self) -> f64 {
        cat_smf_editor::surge_macros::Polarity::from_bipolar(self.bipolar)
            .signal(self.get())
            .unwrap()
    }
    fn readout(&self) -> String {
        let declared = if self.bipolar {
            "bipolar [-1, 1]"
        } else {
            "unipolar [0, 1]"
        };
        let domain = self.curve.domain().unwrap();
        let sent = self.saved().points[0].value;
        let read = self
            .readback
            .as_ref()
            .map(|(raw, text)| domain.describe(*raw, text.as_deref().map_err(String::as_str)))
            .unwrap_or_else(|| "Plugin ? · API read ?".into());
        format!(
            "{declared} · send {} {sent} · read {read}",
            self.curve.value_space.label()
        )
    }
}
fn refresh(app: &mut App) {
    let Some(s) = &app.macro_editor.session else {
        return;
    };
    if s.readback_at
        .is_some_and(|at| at.elapsed() < std::time::Duration::from_millis(150))
        || !app.audition.has_parameter_owner(s.owner, &s.patch)
    {
        return;
    }
    let (owner, layer, patch) = (s.owner, s.layer, s.patch.clone());
    let catalog = app.audition.parameter_catalog(owner, &patch, layer);
    let s = app.macro_editor.session.as_mut().unwrap();
    s.readback_at = Some(std::time::Instant::now());
    match catalog {
        Ok(parameters) => {
            for row in &mut s.rows {
                row.readback = parameters
                    .iter()
                    .find(|p| p.stable_id == row.curve.target.parameter_id)
                    .map(|p| {
                        (
                            p.current,
                            app.audition.parameter_text(
                                owner,
                                layer,
                                p.binding.with_value(row.saved().points[0].value),
                            ),
                        )
                    });
            }
        }
        Err(error) => app.macro_editor.error = Some(error),
    }
}
