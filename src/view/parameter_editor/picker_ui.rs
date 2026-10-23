use super::*;
pub(super) fn picker_ui(app: &mut App, ctx: &egui::Context) {
    let mut confirm = false;
    let mut close = false;
    let mut changed = false;
    let state = &mut app.parameter_editor;
    let p = state.picker.as_mut().unwrap();
    let mut indices = filtered(p);
    p.selected = p.selected.min(indices.len().saturating_sub(1));
    egui::Modal::new(egui::Id::new("parameter-picker")).show(ctx, |ui| {
        ui.set_width(560.0f32.min((ctx.screen_rect().width() - 40.0).max(200.0)));
        ui.horizontal(|ui| {
            ui.heading(format!("{} / Automation", p.title));
            if ui.button("×").clicked() {
                close = true;
            }
        });
        if p.help {
            egui::Grid::new("parameter-picker-help").show(ui, |ui| {
                crate::view::help::key_groups(
                    ui,
                    &[(
                        egui::Color32::LIGHT_BLUE,
                        &[
                            ("↑ ↓", "Parameter選択・Preview"),
                            ("PageUp / PageDown", "10候補移動"),
                            ("Ctrl+Home / Ctrl+End", "候補先頭 / 末尾"),
                            ("Shift+Space", "演奏停止"),
                            ("Esc", "閉じる"),
                        ],
                    )],
                );
            });
            return;
        }
        let response = ui.add(
            // Search remains the text owner while navigating parameter rows.
            egui::TextEdit::singleline(&mut p.query)
                .hint_text("Parameter…")
                .desired_width(f32::INFINITY),
        );
        if p.focus {
            response.request_focus();
            p.focus = false;
        }
        if response.changed() {
            p.selected = 0;
            p.reveal = true;
            changed = true;
            indices = filtered(p);
        }
        crate::view::list_navigation::position(ui, p.selected, indices.len());
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                for (row, &i) in indices.iter().enumerate() {
                    let v = &p.parameters[i];
                    let response =
                        ui.selectable_label(row == p.selected, format!("{} / {}", v.path, v.name));
                    if row == p.selected {
                        ui.small(super::value_label(
                            &app.audition,
                            p.owner,
                            p.layer,
                            v,
                            v.current,
                        ));
                    }
                    if row == p.selected && p.reveal {
                        response.scroll_to_me(Some(egui::Align::Center));
                        p.reveal = false;
                    }
                    if response.clicked() {
                        p.selected = row;
                        changed = true;
                    }
                }
            });
        if let Some(e) = &state.error {
            ui.colored_label(egui::Color32::LIGHT_RED, e);
        }
        confirm = ui
            .add_enabled_ui(!indices.is_empty(), |ui| {
                crate::view::palette::row::action(
                    ui,
                    "Create automation track".into(),
                    "Enter".into(),
                    false,
                    true,
                )
            })
            .inner
            .clicked();
    });
    if close {
        preview::stop(app);
        app.parameter_editor.picker = None;
        crate::view::layers::open(app);
    } else if changed {
        preview::start(app);
    }
    if confirm && let Err(e) = create(app) {
        app.parameter_editor.error = Some(e);
    }
}

/// Preserve IME editing keys and consume no text intended for the query.
pub fn keep_ime(app: &mut App, ctx: &egui::Context) {
    let Some(p) = &mut app.parameter_editor.picker else {
        return;
    };
    let events = ctx.input(|i| i.events.clone());
    let mut ime = p.composing;
    for event in &events {
        if let egui::Event::Ime(e) = event {
            ime = true;
            match e {
                egui::ImeEvent::Preedit(text) => p.composing = !text.is_empty(),
                egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => p.composing = false,
                _ => {}
            }
        }
    }
    if ime
        || events
            .iter()
            .any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Paste(_)))
    {
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::Key {
                        key: egui::Key::Enter
                            | egui::Key::Escape
                            | egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::PageUp
                            | egui::Key::PageDown
                            | egui::Key::Home
                            | egui::Key::End,
                        ..
                    }
                )
            })
        });
    }
}
