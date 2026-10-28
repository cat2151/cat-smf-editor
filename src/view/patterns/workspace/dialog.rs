use super::*;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Create,
    Scene,
    ResizeScene,
}
pub struct Dialog {
    pub viewport: egui::ViewportId,
    purpose: Purpose,
    song: u64,
    scene: Option<u64>,
    name: String,
    length: String,
    query: String,
    search: crate::view::commands::SearchQuery,
    selected: usize,
    composing: bool,
    fresh: bool,
    error: Option<String>,
}
impl Dialog {
    pub fn new(a: &App, ctx: &egui::Context, purpose: Purpose, pane: Pane) -> Self {
        let mut d = Self {
            viewport: ctx.viewport_id(),
            purpose,
            song: a.song_id,
            scene: scene_id(a, pane),
            name: String::new(),
            length: "1".into(),
            query: String::new(),
            search: Default::default(),
            selected: 0,
            composing: false,
            fresh: true,
            error: None,
        };
        if purpose == Purpose::ResizeScene
            && let Some(m) = a.loaded.as_ref().map(|l| &l.music)
            && let Ok(c) = scene::read(m)
            && let Some(s) = c.scenes.iter().find(|s| Some(s.id) == d.scene)
        {
            d.length = s
                .meters
                .iter()
                .map(|m| format!("{}/{}", m.numerator, 1u64 << m.exponent))
                .collect::<Vec<_>>()
                .join(" ");
        }
        d
    }
    fn choices(&self, a: &App) -> Vec<(PatternOrScene, String)> {
        choices(a, Pane::Song)
            .into_iter()
            .filter(|(p, _)| matches!(p, PatternOrScene::Scene(_)))
            .collect()
    }
    fn confirm(
        &self,
        a: &mut App,
        ctx: &egui::Context,
        choice: Option<PatternOrScene>,
    ) -> Result<(), String> {
        if a.song_id != self.song {
            return Err("Arrangement input target changed".into());
        }
        match self.purpose {
            Purpose::ResizeScene => {
                let id = self.scene.ok_or("Select a Scene")?;
                let division = a.loaded.as_ref().ok_or("No SMF is loaded")?.music.division;
                let meters =
                    cat_smf_editor::score::pattern::body::parse_meters(&self.length, division)?;
                mutate(a, |m| scene::sizing::scene(m, id, meters))?;
            }
            Purpose::Create => {
                let n = self
                    .length
                    .trim()
                    .parse()
                    .map_err(|_| "Enter a positive scene length")?;
                let mut id = None;
                mutate(a, |m| {
                    model::create_scene(m, &self.name, n, 1).map(|v| id = Some(v))
                })?;
                open_scene(a, id.ok_or("Missing scene")?)?;
                focus(a, ctx, Pane::Scene);
            }
            Purpose::Scene => {
                let Some(PatternOrScene::Scene(id)) = choice else {
                    return Err("Select a scene".into());
                };
                open_scene(a, id)?;
                focus(a, ctx, Pane::Scene);
            }
        }
        Ok(())
    }
}
pub fn show(a: &mut App, ctx: &egui::Context) {
    let Some(mut d) = a.patterns.workspace.dialog.take() else {
        return;
    };
    if d.viewport != ctx.viewport_id() {
        a.patterns.workspace.dialog = Some(d);
        return;
    }
    if d.song != a.song_id {
        a.patterns.workspace.error = Some("Arrangement input target changed".into());
        crate::play::transport::stop(a);
        return;
    }
    let events = ctx.input(|i| i.events.clone());
    let ime = d.composing || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
    let mut confirm = false;
    let mut cancel = false;
    for e in &events {
        if let egui::Event::Ime(e) = e {
            match e {
                egui::ImeEvent::Preedit(s) => d.composing = !s.is_empty(),
                egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => d.composing = false,
                _ => {}
            }
        }
        if !ime
            && let egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = e
            && crate::view::commands::matches(*modifiers, Modifiers::NONE)
        {
            confirm |= *key == Key::Enter;
            cancel |= *key == Key::Escape;
            if *key == Key::ArrowDown
                && ctx.memory(|m| m.focused() == Some(egui::Id::new("arrangement-query")))
            {
                d.selected = d.selected.saturating_add(1);
            }
            if *key == Key::ArrowUp
                && ctx.memory(|m| m.focused() == Some(egui::Id::new("arrangement-query")))
            {
                d.selected = d.selected.saturating_sub(1);
            }
        }
    }
    if cancel {
        crate::play::transport::stop(a);
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        crate::view::comma_menu::swallow(ctx);
        return;
    }
    if !ime {
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::Key {
                        key: Key::Enter | Key::Escape,
                        ..
                    }
                )
            })
        });
    }
    let all = d.choices(a);
    d.search.update(&d.query);
    let mut candidates = all
        .iter()
        .filter(|(_, name)| d.search.matches_text(name))
        .collect::<Vec<_>>();
    let mut ready = true;
    egui::Modal::new(egui::Id::new("arrangement-input")).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.heading(match d.purpose {
            Purpose::Create => "Create scene",
            Purpose::Scene => "Edit scene",
            Purpose::ResizeScene => "Scene measures / meter",
        });
        if d.purpose == Purpose::Create {
            for (label, value, id) in [
                ("Name", &mut d.name, "arrangement-name"),
                ("Measures", &mut d.length, "arrangement-length"),
            ] {
                ui.label(label);
                if d.fresh && id == "arrangement-name" {
                    ctx.memory_mut(|m| m.request_focus(egui::Id::new(id)));
                }
                let r = ui.add(egui::TextEdit::singleline(value).id(egui::Id::new(id)));
                #[cfg(test)]
                ctx.data_mut(|d| d.insert_temp(egui::Id::new(("arrangement-field", id)), r.rect));
                if d.fresh && id == "arrangement-name" {
                    r.request_focus();
                }
            }
        } else if d.purpose == Purpose::ResizeScene {
            ui.label("Measures (4/4 3/4)");
            let id = egui::Id::new("arrangement-length");
            ready = ui.is_enabled() && ctx.memory(|m| m.allows_interaction(ui.layer_id()));
            if d.fresh {
                // The modal's first sizing pass disables widgets and drops focus.
                // Keep selection/focus initialization until its interactive pass.
                use egui::text::{CCursor, CCursorRange};
                let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                state.cursor.set_char_range(Some(CCursorRange::two(
                    CCursor::new(0),
                    CCursor::new(d.length.chars().count()),
                )));
                state.store(ctx, id);
                ctx.memory_mut(|m| m.request_focus(id));
            }
            let response = ui.add(egui::TextEdit::singleline(&mut d.length).id(id));
            if d.fresh {
                response.request_focus();
            }
        } else {
            ui.label("Scene");
            if d.fresh {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("arrangement-query")));
            }
            let r = ui.add(
                egui::TextEdit::singleline(&mut d.query).id(egui::Id::new("arrangement-query")),
            );
            if d.fresh {
                r.request_focus();
            }
            if r.changed() {
                d.search.update(&d.query);
                d.selected = 0;
                candidates = all
                    .iter()
                    .filter(|(_, name)| d.search.matches_text(name))
                    .collect();
            }
            d.selected = d.selected.min(candidates.len().saturating_sub(1));
            egui::ScrollArea::vertical()
                .drag_to_scroll(false)
                .max_height((ctx.screen_rect().height() - 130.0).clamp(40.0, 200.0))
                .show(ui, |ui| {
                    for (i, (part, label)) in candidates.iter().enumerate() {
                        if ui.selectable_label(i == d.selected, label).clicked() {
                            d.selected = i;
                            match part {
                                PatternOrScene::Pattern(id) => {
                                    let r = preview::start(a, *id);
                                    result(a, r);
                                }
                                PatternOrScene::Scene(id) => {
                                    crate::play::arrangement::start(a, Some(*id))
                                }
                            }
                        }
                    }
                });
        }
        if let Some(e) = &d.error {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        let r = ui.button(if d.purpose == Purpose::Create {
            "Enter · Create"
        } else {
            "Enter · Confirm"
        });
        confirm |= r.clicked();
        #[cfg(test)]
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("arrangement-confirm"), r.rect));
    });
    d.fresh &= !ready;
    if confirm {
        let choice = candidates.get(d.selected).map(|(p, _)| *p);
        match d.confirm(a, ctx, choice) {
            Ok(()) => return,
            Err(e) => d.error = Some(e),
        }
    }
    a.patterns.workspace.dialog = Some(d);
}
