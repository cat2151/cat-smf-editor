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
