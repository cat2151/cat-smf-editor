use eframe::egui;

pub(super) fn context() -> egui::Context {
    let c = egui::Context::default();
    c.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
        let id = viewport.ids.this;
        if id != super::viewport() {
            return;
        }
        let events = ctx
            .data_mut(|d| d.remove_temp::<Vec<egui::Event>>(egui::Id::new("help-input")))
            .unwrap_or_default();
        let geometry = ctx.data(|d| d.get_temp::<[f32; 4]>(egui::Id::new("help-geometry")));
        let mut input = egui::RawInput {
            viewport_id: id,
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 1800.0),
            )),
            ..Default::default()
        };
        input.viewports.insert(
            id,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                focused: Some(false),
                outer_rect: geometry.map(|[x, y, w, h]| {
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w + 16.0, h + 40.0))
                }),
                inner_rect: geometry.map(|[x, y, w, h]| {
                    egui::Rect::from_min_size(egui::pos2(x + 8.0, y + 32.0), egui::vec2(w, h))
                }),
                ..Default::default()
            },
        );
        let out = ctx.run(input, |ctx| (viewport.viewport_ui_cb)(ctx));
        let texts: Vec<String> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("help-texts"), texts);
            d.insert_temp(egui::Id::new("help-builder"), viewport.builder.clone());
        });
    });
    c
}

pub(super) fn texts(c: &egui::Context) -> Vec<String> {
    c.data(|d| d.get_temp(egui::Id::new("help-texts")))
        .unwrap_or_default()
}

#[test]
fn native_help_never_activates_and_its_escape_closes_only_it() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    a.help_open = true;
    a.piano_roll.open = true;
    let c = context();
    let _ = c.run(Default::default(), |c| super::show(&mut a, c));
    let b: egui::ViewportBuilder = c
        .data(|d| d.get_temp(egui::Id::new("help-builder")))
        .unwrap();
    assert_eq!(b.active, Some(false));
    c.data_mut(|d| {
        d.insert_temp(
            egui::Id::new("help-input"),
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
    });
    let _ = c.run(Default::default(), |c| super::show(&mut a, c));
    assert!(!a.help_open);
    assert!(a.piano_roll.open);
}

#[test]
fn root_dialog_keeps_help_visible_and_owns_its_escape() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    a.config.navigation.departure = Some(crate::view::files::Action::New);
    let c = context();
    for key in [egui::Key::F1, egui::Key::Escape] {
        let _ = c.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |c| {
                super::input(&mut a, c);
                crate::view::commands::route(&mut a, c);
                super::show(&mut a, c);
            },
        );
        assert!(a.help_open);
        assert!(c
            .data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new("help-builder")))
            .is_some());
    }
    assert!(a.config.navigation.departure.is_none());
}

#[test]
fn dragging_help_is_never_pulled_back_to_the_previous_frame() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    a.help_open = true;
    let c = context();
    let mut previous: Option<egui::ViewportBuilder> = None;
    for step in 0..6 {
        let shift = step as f32 * 7.0;
        let rect = [100.0 + shift, 50.0 + shift, 600.0 + shift, 500.0 + shift];
        c.data_mut(|d| d.insert_temp(egui::Id::new("help-geometry"), rect));
        let _ = c.run(Default::default(), |c| super::show(&mut a, c));
        let now: egui::ViewportBuilder = c
            .data(|d| d.get_temp(egui::Id::new("help-builder")))
            .unwrap();
        if let Some(mut previous) = previous {
            // eframe applies `previous.patch(next)` to the live native window every frame.
            let (commands, _) = previous.patch(now.clone());
            assert!(
                !commands.iter().any(|c| matches!(
                    c,
                    egui::ViewportCommand::OuterPosition(_) | egui::ViewportCommand::InnerSize(_)
                )),
                "step {step}: {commands:?}"
            );
        }
        previous = Some(now);
    }
    // Closing keeps the dragged place for the next opening.
    a.help_open = false;
    let _ = c.run(Default::default(), |c| super::show(&mut a, c));
    assert_eq!(a.config.help_window, Some([135.0, 85.0, 635.0, 535.0]));
}
