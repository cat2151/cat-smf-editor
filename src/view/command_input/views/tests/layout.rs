use super::*;

fn rect(ctx: &egui::Context, depth: usize) -> egui::Rect {
    ctx.memory(|m| m.area_rect(egui::Id::new(("views-list", depth))).unwrap())
}

#[test]
fn track_children_appear_without_right_and_lists_fit_their_contents() {
    let mut a = fixture();
    let c = egui::Context::default();
    browse(&mut a, &c);
    select(&mut a, Target::Level(Level::Track(1)));
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    assert_eq!(a.command_input.views.as_ref().unwrap().level, Level::Tracks);
    let root = rect(&c, 0);
    let children = rect(&c, 1);
    assert!(root.left() < 1.0);
    assert!(children.left() >= root.right());
    assert!(children.height() > 80.0 && children.height() < 200.0);
    assert!(root.height() > children.height());
    let preview = c.memory(|m| m.area_rect(egui::Id::new("views-preview-window")).unwrap());
    assert!(
        (preview.center() - egui::pos2(750.0, 450.0)).length() < 3.0,
        "{preview:?}"
    );
    assert!(preview.width() < 1400.0 && preview.height() < 800.0);
    assert_eq!(
        c.layer_id_at(preview.center()).unwrap().id,
        egui::Id::new("views-preview-window")
    );
    assert_eq!(a.track, 0);
}

#[test]
fn long_lists_wrap_instead_of_clipping_rows_or_scrolling() {
    let mut music = tracks::new_song();
    for _ in 0..35 {
        tracks::add(&mut music).unwrap();
    }
    let mut a = crate::view::tests::app(music);
    let c = egui::Context::default();
    browse(&mut a, &c);
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    let root = rect(&c, 0);
    assert!(root.width() > 400.0, "{root:?}");
    assert!(root.bottom() < 900.0, "{root:?}");
}

#[test]
fn piano_and_tempo_preview_ignore_maximize_and_preserve_editors_on_cancel() {
    let mut a = fixture();
    let c = egui::Context::default();
    a.track_view.cursor_tick = Some(240);
    a.piano_roll.open_at_cursor();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    browse(&mut a, &c);
    select(&mut a, Target::View(View::Tempo));
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    let pos = egui::pos2(900.0, 650.0);
    frame(&mut a, &c, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            &mut a,
            &c,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(a.piano_roll.open);
    assert!(!a.tempo_curve.open);
    select(&mut a, Target::Level(Level::Track(1)));
    press(&mut a, &c, Key::ArrowRight);
    select(&mut a, Target::View(View::Piano(1)));
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    let preview = c.memory(|m| m.area_rect(egui::Id::new("views-preview-window")).unwrap());
    assert!(
        (preview.center() - egui::pos2(750.0, 450.0)).length() < 3.0,
        "{preview:?}"
    );
    assert!(a.config.piano_roll_maximized && a.config.tempo_curve_maximized);
    assert_eq!(a.track, 0);
    assert_eq!(a.track_view.cursor_tick, Some(240));
    press(&mut a, &c, Key::Escape);
    press(&mut a, &c, Key::Escape);
    assert!(!crate::view::modal::is_open(&a));
    assert!(a.piano_roll.open);
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}
