use super::*;
mod reload;

fn fixture_app() -> (App, tempfile::TempDir) {
    let mut a = app();
    let dir = worded(&mut a, &commands::descriptions::tests::document());
    (a, dir)
}

fn draw(a: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let time = ctx.input(|i| i.time) + 1.0;
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |c| super::super::super::show(a, c),
    )
}

fn has_text(shapes: &[egui::epaint::ClippedShape], expected: &str) -> bool {
    fn contains(shape: &egui::Shape, expected: &str) -> bool {
        match shape {
            egui::Shape::Text(t) => t.galley.text().contains(expected),
            egui::Shape::Vec(shapes) => shapes.iter().any(|s| contains(s, expected)),
            _ => false,
        }
    }
    shapes.iter().any(|s| contains(&s.shape, expected))
}

#[test]
fn japanese_explanations_are_searchable_by_migemo_and_multiple_words() {
    let (mut a, _dir) = fixture_app();
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    let commands = &a.palette.commands;
    let mut search = commands::SearchQuery::default();
    for q in [
        "試聴",
        "shichou",
        "SHICHOU",
        "pattern shichou",
        "conductor テンポ",
        "haichi pattern",
    ] {
        let found = filter(commands, q, &mut search, &history::History::default());
        assert!(
            found.iter().any(|&i| commands[i].id == "pattern.preview"),
            "{q}"
        );
    }
    assert!(!commands.iter().any(|c| c.id == "pattern-audition-tempo"));
}

#[test]
fn descriptions_extend_results_without_hiding_named_commands_or_learned_choices() {
    let (mut a, _dir) = fixture_app();
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    let commands = &a.palette.commands;
    let mut search = commands::SearchQuery::default();
    let mut history = history::History::default();
    let found = filter(commands, "pattern", &mut search, &history);
    assert_eq!(commands[found[0]].id, "song.open");
    let record = commands
        .iter()
        .position(|c| c.id == "pattern.record")
        .unwrap();
    assert!(found.contains(&record));
    history.record("pattern", &commands[record]);
    let found = filter(commands, "pattern", &mut search, &history);
    assert_eq!(commands[found[0]].id, "pattern.record");
    let create = commands.iter().find(|c| c.id == "pattern.create").unwrap();
    history.record("create pattern", create);
    let found = filter(commands, "pattern song", &mut search, &history);
    assert_eq!(commands[found[0]].id, "song.open");
}

#[test]
fn f1_is_local_toggles_description_and_preserves_query_navigation_and_execution() {
    let (mut a, _dir) = fixture_app();
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    query(&mut a, &ctx, "preview pattern");
    press(&mut a, &ctx, Key::F1);
    assert!(a.palette.description_open);
    assert!(!a.help_open);
    assert_eq!(a.palette.query, "preview pattern");
    assert!(a.playback.pending.is_none());
    let output = draw(&mut a, &ctx, vec![]);
    assert!(has_text(&output.shapes, "Songカーソル位置"));
    press(&mut a, &ctx, Key::F1);
    assert!(!a.palette.description_open);
    press(&mut a, &ctx, Key::F1);
    press(&mut a, &ctx, Key::ArrowDown);
    assert!(!a.palette.description_open);
    press(&mut a, &ctx, Key::F1);
    press(&mut a, &ctx, Key::Escape);
    assert!(a.palette.is_open());
    assert!(!a.palette.description_open);
    press(&mut a, &ctx, Key::F1);
    press(&mut a, &ctx, Key::Enter);
    assert!(!a.palette.is_open());
    assert!(a.command_input.is_open());
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.command_input.is_open());
}

#[test]
fn hover_shows_the_same_explanation_without_executing_or_selecting() {
    let (mut a, _dir) = fixture_app();
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    query(&mut a, &ctx, "preview pattern");
    draw(&mut a, &ctx, vec![]);
    let rect = ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("palette-row", "pattern.preview"))))
        .unwrap();
    draw(&mut a, &ctx, vec![egui::Event::PointerMoved(rect.center())]);
    let mut found = false;
    for _ in 0..4 {
        found |= has_text(&draw(&mut a, &ctx, vec![]).shapes, "Songカーソル位置");
    }
    assert!(found);
    assert_eq!(a.palette.selected, 0);
    assert!(!a.palette.description_open);
    assert!(a.playback.pending.is_none());
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.palette.is_open());
}

#[test]
fn ime_and_empty_results_do_not_open_descriptions_or_leak_f1_to_help() {
    let (mut a, _dir) = fixture_app();
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::Ime(egui::ImeEvent::Preedit("し".into())),
            key(Key::F1, Modifiers::NONE),
        ],
    );
    assert!(!a.palette.description_open);
    assert!(!a.help_open);
    frame(
        &mut a,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    query(&mut a, &ctx, "no-such-description-result");
    press(&mut a, &ctx, Key::F1);
    assert!(!a.palette.description_open);
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.palette.is_open());
}
