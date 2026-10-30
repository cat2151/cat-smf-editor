use super::*;
use crate::view::{piano_roll, tempo_curve::Target as Kind};
use cat_smf_editor::score::automation::{validate_parameter, ParameterSpec};
use egui::Color32;

pub(super) fn resolve(app: &mut App) -> Result<(), String> {
    let c = current(app)?;
    let loaded = app.loaded.as_ref().unwrap();
    let owner = curve_owner(app)?;
    app.parameter_editor.owner = Some(owner);
    let patch = loaded.patches[owner].clone();
    let key = plugin(&patch, c.target.layer)?;
    if key.format != c.target.plugin_format || key.id != c.target.plugin_id {
        return Err("Automation instrument changed".into());
    }
    let p = catalog(app, owner, &patch, c.target.layer)?
        .into_iter()
        .find(|p| p.stable_id == c.target.parameter_id)
        .ok_or("Automation parameter is unavailable")?;
    validate_parameter(
        &c,
        ParameterSpec {
            stable_id: &p.stable_id,
            minimum: p.minimum,
            maximum: p.maximum,
            normalized: p.normalized,
            eligible: p.eligible,
        },
    )?;
    let labels = (0..=4)
        .map(|i| {
            let v = c.minimum + (c.maximum - c.minimum) * f64::from(i) / 4.0;
            (
                v,
                super::axis_label(&app.audition, owner, c.target.layer, &p, v),
            )
        })
        .collect();
    app.parameter_editor.labels = labels;
    app.parameter_editor.labels_axis = Some(super::super::tempo_curve::Axis {
        low: c.minimum,
        high: c.maximum,
    });
    app.parameter_editor.parameter = Some(p);
    app.parameter_editor.selected = (!c.points.is_empty()).then_some((Kind::Point, 0));
    Ok(())
}
pub(super) fn show(app: &mut App, ctx: &egui::Context) {
    let Ok(c) = current(app) else {
        app.parameter_editor.close();
        return;
    };
    let loaded = app.loaded.as_ref().unwrap();
    let variation = app.parameter_editor.variation_layer.is_some();
    let owner = if variation {
        Some(app.track)
    } else {
        curve_owner(app).ok()
    };
    let title = owner.map_or_else(
        || "Missing owner".into(),
        |o| loaded.track_titles[o].clone(),
    );
    let layer = owner.map_or_else(
        || "Missing layer".into(),
        |o| layer_label(&loaded.patches[o], c.target.layer),
    );
    let layout = Some(owner.unwrap_or(app.track))
        .map(|o| piano_roll::layout_of(loaded, &[o], o, crate::play::playback::location(app).0));
    let playing = owner.and_then(|o| crate::play::transport::tick_for(app, o));
    let repeat = app.playback.repeat;
    let follow = app.playback.follow;
    let mut open = true;
    let mut edited = None;
    let mut range_action = None;
    let mut seek = None;
    let mut navigated = false;
    let mut cancel = false;
    let mut confirm = false;
    let mut value_text = None;
    let s = &mut app.parameter_editor;
    let axis = s.viewport.axis.unwrap_or(super::super::tempo_curve::Axis {
        low: c.minimum,
        high: c.maximum,
    });
    if variation {
        s.labels = (0..=c.maximum as usize)
            .map(|i| (i as f64, format!("Variation {}", i + 1)))
            .collect();
    } else if s.labels_axis != Some(axis) {
        s.labels = (0..=4)
            .map(|i| {
                let value = axis.low + (axis.high - axis.low) * f64::from(i) / 4.0;
                let text = s
                    .parameter
                    .as_ref()
                    .zip(owner)
                    .map(|(p, owner)| {
                        super::axis_label(&app.audition, owner, c.target.layer, p, value)
                    })
                    .unwrap_or_else(|| value.to_string());
                (value, text)
            })
            .collect();
        s.labels_axis = Some(axis);
    }
    let selected_value = s.input.as_ref().map(|i| i.value).or_else(|| {
        s.selected
            .and_then(|(k, i)| {
                if k == Kind::Point {
                    c.points.get(i)
                } else {
                    c.controls.get(i)
                }
            })
            .map(|p| p.value)
    });
    if let (Some(p), Some(owner), Some(v)) = (&s.parameter, owner, selected_value) {
        value_text = Some(super::value_label(
            &app.audition,
            owner,
            c.target.layer,
            p,
            v,
        ));
    }
    if variation {
        value_text = selected_value.map(|v| format!("Variation {}", v + 1.0));
    }
    let window_title = format!("{} / {} / {} / {}", title, layer, c.path, c.name);
    let maximized = if variation {
        app.config.macro_variation_curve_maximized
    } else {
        app.config.parameter_curve_maximized
    };
    super::window::window(ctx, maximized, window_title.clone())
        .open(&mut open)
        .show(ctx, |ui| {
            if maximized {
                // The footer may be empty; egui otherwise shrinks the window to its contents.
                ui.set_min_size(ui.available_size());
                ui.heading(&window_title);
            }
            if let Some(e) = &s.error {
                ui.colored_label(Color32::LIGHT_RED, e);
            }
            if let Some(layout) = &layout {
                let (edit, action, tick, navigation) = ui
                    .add_enabled_ui(s.input.is_none(), |ui| {
                        super::canvas::show(ui, s, &c, layout, playing, follow, repeat)
                    })
                    .inner;
                edited = edit;
                range_action = action;
                seek = tick;
                navigated = navigation;
            }
            if variation && super::variation::point_input(s) && s.input.is_some() {
                // Point values are chosen in the foreground variation overlay.
            } else if let Some(input) = &mut s.input {
                ui.separator();
                ui.horizontal(|ui| {
                    if !variation {
                        ui.label("Tick");
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut input.tick)
                                .id(egui::Id::new("parameter-tick-input"))
                                .desired_width(110.0),
                        );
                        if input.focus {
                            r.request_focus();
                            input.focus = false;
                        }
                    }
                    if let Some(text) = &value_text {
                        ui.label(text);
                    }
                    confirm = ui.button("Apply  Enter").clicked();
                    cancel = ui.button("Cancel  Esc").clicked();
                });
            } else if let Some((kind, index)) = s.selected {
                let point = if kind == Kind::Point {
                    c.points.get(index)
                } else {
                    c.controls.get(index)
                };
                if let Some(p) = point {
                    ui.label(format!(
                        "{}  ·  {}",
                        p.tick,
                        value_text
                            .as_deref()
                            .map_or_else(|| p.value.to_string(), str::to_owned)
                    ));
                }
            }
        });
    if !open {
        app.parameter_editor.close();
    }
    if let Some(c) = edited {
        if variation && c.points.len() > current(app).map_or(0, |old| old.points.len()) {
            app.parameter_editor.draft = Some(c);
            begin_input(app);
        } else {
            store(app, c);
        }
    }
    if let Some(a) = range_action {
        super::super::timeline_repeat::apply(app, a);
    }
    if navigated {
        app.playback.follow = false;
    }
    if let Some((tick, audition)) = seek {
        if navigated {
            crate::view::timeline_domain::click(
                crate::view::timeline_controls::Screen::Parameter,
                app,
                app.track,
                tick,
            );
        } else if crate::play::transport::timeline::running(app) {
            crate::play::transport::timeline::seek(app, app.track, tick);
        } else {
            crate::cursor::moves::at_tick(app, tick);
            if audition {
                crate::play::playback::start(app, crate::play::playback::Mode::BeatOnly);
            }
        }
    }
    if cancel {
        app.parameter_editor.input = None;
        app.parameter_editor.draft = None;
    }
    if confirm {
        commit_input(app);
    }
    if variation {
        super::variation::overlay(app, ctx, c.maximum);
    }
}
pub(super) fn begin_input(app: &mut App) {
    let Ok(c) = app
        .parameter_editor
        .draft
        .clone()
        .map(Ok)
        .unwrap_or_else(|| current(app))
    else {
        return;
    };
    let Some((kind, i)) = app.parameter_editor.selected else {
        return;
    };
    let point = if kind == Kind::Point {
        c.points.get(i)
    } else {
        c.controls.get(i)
    };
    if let Some(p) = point {
        app.parameter_editor.input = Some(Input {
            tick: p.tick.to_string(),
            value: if super::variation::point_input(&app.parameter_editor) {
                p.value.round()
            } else {
                p.value
            },
            focus: true,
            reveal: true,
        });
    }
}
/// Adjust a preview in the plugin's value space without a text value field.
pub(super) fn adjust_value(app: &mut App, direction: f64) {
    let Ok(c) = current(app) else {
        return;
    };
    if super::variation::point_input(&app.parameter_editor) {
        if let Some(input) = &mut app.parameter_editor.input {
            input.value = (input.value + direction)
                .clamp(c.minimum, c.maximum)
                .round();
            input.reveal = true;
        }
        return;
    }
    if app.parameter_editor.variation_layer.is_some() {
        if let Some(input) = &mut app.parameter_editor.input {
            let next = input.value + direction * (c.maximum - c.minimum) / 100.0;
            if next.is_finite() {
                input.value = next;
            }
        }
        return;
    }
    if let Some(input) = &mut app.parameter_editor.input
        && let Ok(domain) = c.domain()
        && let Ok(value) = domain.value(input.value)
        && let Ok(position) = domain.position(value)
    {
        let next = cat_smf_editor::host::parameters::domain::Position::new(
            (position.get() + direction / 100.0).clamp(0.0, 1.0),
        )
        .unwrap();
        input.value = domain.api(next).get();
    }
}
pub(super) fn commit_input(app: &mut App) {
    let result = (|| {
        let i = app
            .parameter_editor
            .input
            .as_ref()
            .ok_or("No value input")?;
        let tick = i.tick.parse::<u64>().map_err(|_| "Invalid integer tick")?;
        let value = i.value;
        let (kind, index) = app.parameter_editor.selected.ok_or("No selected point")?;
        let mut c = app
            .parameter_editor
            .draft
            .clone()
            .map(Ok)
            .unwrap_or_else(|| current(app))?;
        if kind == Kind::Point {
            c.move_point(index, tick, value)?;
        } else {
            c.move_control(index, tick, value)?;
        }
        if app.parameter_editor.variation_layer.is_some() {
            for (segment, (pair, control)) in c.points.windows(2).zip(&mut c.controls).enumerate() {
                if kind == Kind::Point
                    && app.parameter_editor.draft.is_some()
                    && (segment == index || segment + 1 == index)
                {
                    control.value = (pair[0].value + pair[1].value) / 2.0;
                }
            }
        }
        c.validate()?;
        Ok(c)
    })();
    match result {
        Ok(c) => {
            app.parameter_editor.input = None;
            app.parameter_editor.draft = None;
            app.parameter_editor.error = None;
            store(app, c);
        }
        Err(e) => app.parameter_editor.error = Some(e),
    }
}
pub(super) fn select(app: &mut App, next: bool, control: bool) {
    let Ok(c) = current(app) else {
        return;
    };
    let kind = if control { Kind::Control } else { Kind::Point };
    let count = if control {
        c.controls.len()
    } else {
        c.points.len()
    };
    if count == 0 {
        app.parameter_editor.selected = None;
        return;
    }
    let previous = app
        .parameter_editor
        .selected
        .filter(|(k, _)| *k == kind)
        .map(|(_, i)| i);
    let index = previous.map_or(0, |i| {
        if next {
            (i + 1) % count
        } else {
            (i + count - 1) % count
        }
    });
    app.parameter_editor.selected = Some((kind, index));
}
pub(super) fn add(app: &mut App) {
    let Ok(mut c) = current(app) else {
        return;
    };
    let tick = crate::play::playback::location(app).0;
    let value = c
        .value_at(tick as f64)
        .or_else(|| app.parameter_editor.parameter.as_ref().map(|p| p.current))
        .unwrap_or(c.minimum);
    let value = if app.parameter_editor.variation_layer.is_some() {
        value.round()
    } else {
        value
    };
    match c.add(tick, value) {
        Ok(i) => {
            app.parameter_editor.selected = Some((Kind::Point, i));
            app.parameter_editor.draft = Some(c);
            begin_input(app);
        }
        Err(e) => app.parameter_editor.error = Some(e),
    }
}
pub(super) fn remove(app: &mut App) {
    let Some((Kind::Point, index)) = app.parameter_editor.selected else {
        return;
    };
    let Ok(mut c) = current(app) else {
        return;
    };
    match c.remove(index) {
        Ok(()) => {
            app.parameter_editor.selected = (!c.points.is_empty())
                .then_some((Kind::Point, index.min(c.points.len().saturating_sub(1))));
            store(app, c);
        }
        Err(e) => app.parameter_editor.error = Some(e),
    }
}
