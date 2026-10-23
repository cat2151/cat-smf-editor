//! Variation-specific endpoints, using the common scalar curve canvas and commands.
use super::*;
use cat_smf_editor::score::macro_curve;

pub(crate) fn open(app: &mut App, layer: u64) {
    open_with_playback(app, layer, true);
}
pub(super) fn open_view(app: &mut App, layer: u64) {
    open_with_playback(app, layer, false);
}
fn open_with_playback(app: &mut App, layer: u64, play: bool) {
    match current(app, layer) {
        Ok(c) => {
            app.parameter_editor = State {
                open: true,
                variation_layer: Some(layer),
                owner: Some(app.track),
                selected: (!c.points.is_empty())
                    .then_some((super::super::tempo_curve::Target::Point, 0)),
                ..Default::default()
            };
            app.piano_roll.open = false;
            app.tempo_curve.open = false;
            app.layer_editor.open = false;
            app.comma_menu.open = false;
            app.help_open = false;
            let end = app
                .loaded
                .as_ref()
                .and_then(|l| l.music.tracks[app.track].events.last())
                .map_or(0, |e| e.tick);
            if play && end > 0 {
                crate::play::transport::repeat::start(app, (0, end));
            }
        }
        Err(e) => app.player_selection.error = Some(e),
    }
}

pub(super) fn current(app: &App, layer: u64) -> Result<Curve, String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let track = loaded
        .music
        .tracks
        .get(app.track)
        .ok_or("Missing macro owner")?;
    macro_curve::expand(track)?;
    let values = cat_smf_editor::score::macro_values::read(track)?;
    let set = values
        .variations
        .iter()
        .find(|v| v.layer == layer)
        .filter(|v| v.snapshots.len() >= 2)
        .ok_or("Create at least two Macro variations first")?;
    let template = &set.snapshots[0][0];
    let key = plugin(&loaded.patches[app.track], layer)?;
    if template.target.plugin_format != key.format || template.target.plugin_id != key.id {
        return Err("Macro variation instrument changed".into());
    }
    let mut c = macro_curve::read(track)?
        .into_iter()
        .find(|c| c.target.layer == layer)
        .unwrap_or_else(|| Curve {
            version: 1,
            target: Target {
                group: 1,
                layer,
                // This is an index axis, not a native plugin parameter domain.
                plugin_format: "CLAP".into(),
                plugin_id: format!("{}:{}", key.format, key.id),
                parameter_id: "macro-variation".into(),
            },
            value_space: ValueSpace::Plain,
            minimum: 0.0,
            maximum: (set.snapshots.len() - 1) as f64,
            name: "Macro variation curve".into(),
            path: "Macros".into(),
            points: vec![],
            controls: vec![],
        });
    c.maximum = (set.snapshots.len() - 1) as f64;
    c.validate()?;
    Ok(c)
}

pub(super) fn point_input(s: &State) -> bool {
    s.variation_layer.is_some()
        && s.selected
            .is_some_and(|(kind, _)| kind == super::super::tempo_curve::Target::Point)
}

pub(super) fn overlay(app: &mut App, ctx: &egui::Context, maximum: f64) {
    if !point_input(&app.parameter_editor) || app.parameter_editor.input.is_none() {
        return;
    }
    let mut confirm = false;
    let mut cancel = false;
    let mut open = true;
    egui::Window::new("Macro variation")
        .id(egui::Id::new("macro-variation-point"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .open(&mut open)
        .show(ctx, |ui| {
            let input = app.parameter_editor.input.as_mut().unwrap();
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for i in 0..=maximum as usize {
                        let response = ui.selectable_label(
                            input.value == i as f64,
                            format!("Variation {}", i + 1),
                        );
                        if response.clicked() {
                            input.value = i as f64;
                            confirm = true;
                        }
                        if input.reveal && input.value == i as f64 {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                });
            input.reveal = false;
            ui.horizontal(|ui| {
                confirm |= ui.button("Apply  Enter").clicked();
                cancel |= ui.button("Cancel  Esc").clicked();
            });
        });
    if !open || cancel {
        app.parameter_editor.input = None;
        app.parameter_editor.draft = None;
    } else if confirm {
        super::curve::commit_input(app);
    }
}
