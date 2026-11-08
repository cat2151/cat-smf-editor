use super::*;
use crate::view::commands::{self as shared, Screen};
pub(super) fn frame(a: &mut App, ctx: &egui::Context, events: Vec<egui::Event>, width: f32) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| show(a, ctx, None),
    );
}
pub(super) fn key(a: &mut App, ctx: &egui::Context, key: Key, modifiers: egui::Modifiers) {
    frame(
        a,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .collect(),
        1200.0,
    );
}
fn app(count: u8) -> App {
    let mut a = super::super::kit_edit_tests::fixture(count);
    super::super::open(&mut a);
    sync(&mut a);
    a
}
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
#[test]
fn kit_keyboard_audition_steps_pages_default_hit_velocity_undo_and_close() {
    let mut a = app(128);
    let ctx = egui::Context::default();
    let before = bytes(&a);
    assert_eq!(shared::owner(&a, &ctx), Screen::Drums);
    for _ in 0..3 {
        key(&mut a, &ctx, Key::H, egui::Modifiers::NONE);
    }
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    assert!(a.drum_editor.kit.picker.is_some());
    assert!(a.audition.auditioned.is_empty());
    key(&mut a, &ctx, Key::Space, egui::Modifiers::NONE);
    assert_eq!(a.audition.auditioned, [0]);
    key(&mut a, &ctx, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    for _ in 0..5 {
        key(&mut a, &ctx, Key::J, egui::Modifiers::NONE);
    }
    assert_eq!(a.drum_editor.kit.note, Some(5));
    assert_eq!(a.audition.auditioned, [0]);
    key(&mut a, &ctx, Key::Space, egui::Modifiers::NONE);
    assert_eq!(a.drum_editor.field, 2);
    key(&mut a, &ctx, Key::F10, egui::Modifiers::SHIFT);
    assert!(a.drum_editor.kit.menu);
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    a.drum_editor.kit.input.as_mut().unwrap().text = "64".into();
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    let hit = a.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .find(|e| cat_smf_editor::score::note_view::note_on(e))
        .unwrap();
    assert_eq!(hit.data, [5, 64]);
    key(&mut a, &ctx, Key::Z, egui::Modifiers::CTRL);
    let hit = a.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .find(|e| cat_smf_editor::score::note_view::note_on(e))
        .unwrap();
    assert_eq!(hit.data, [5, 100]);
    key(&mut a, &ctx, Key::PageDown, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::F, egui::Modifiers::NONE);
    assert_eq!(a.drum_editor.page, 1);
    key(&mut a, &ctx, Key::R, egui::Modifiers::NONE);
    assert!(a.drum_editor.loop_enabled);
    key(&mut a, &ctx, Key::Space, egui::Modifiers::SHIFT);
    assert!(!a.drum_editor.loop_enabled);
    key(&mut a, &ctx, Key::Escape, egui::Modifiers::NONE);
    assert!(!super::super::is_open(&a));
}
#[test]
fn kit_migemo_input_ime_no_key_leaks_and_note_identity_survive_filtering() {
    let mut a = app(128);
    let ctx = egui::Context::default();
    a.loaded.as_mut().unwrap().patches[1]
        .kit
        .as_mut()
        .unwrap()
        .notes[42]
        .name = "音色 フィルター Snare".into();
    a.drum_editor.kit.note = Some(42);
    a.drum_editor.kit.slot = 42;
    let before = bytes(&a);
    key(&mut a, &ctx, Key::Slash, egui::Modifiers::NONE);
    a.drum_editor.kit.query = "onshoku firuta snare".into();
    frame(&mut a, &ctx, vec![], 1200.0);
    assert_eq!(
        rows(&a).iter().map(|n| n.note).collect::<Vec<_>>(),
        [Some(42)]
    );
    key(&mut a, &ctx, Key::R, egui::Modifiers::NONE);
    assert!(!a.drum_editor.loop_enabled);
    frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::Ime(egui::ImeEvent::Preedit("あ".into())),
            egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        1200.0,
    );
    assert!(a.drum_editor.kit.query_before.is_some());
    frame(
        &mut a,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
        1200.0,
    );
    key(&mut a, &ctx, Key::Escape, egui::Modifiers::NONE);
    assert!(a.drum_editor.kit.query.is_empty());
    assert_eq!(a.drum_editor.kit.note, Some(42));
    assert_eq!(bytes(&a), before);
}
#[test]
fn kit_128_rows_long_names_narrow_scroll_and_browser_owner() {
    let mut a = app(128);
    let ctx = egui::Context::default();
    a.loaded.as_mut().unwrap().patches[1]
        .kit
        .as_mut()
        .unwrap()
        .notes[127]
        .name = "Long source note name ".repeat(20);
    a.drum_editor.kit.note = Some(127);
    a.drum_editor.kit.slot = 127;
    a.drum_editor.kit.reveal = true;
    for _ in 0..4 {
        frame(&mut a, &ctx, vec![], 360.0);
    }
    let rect = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new(("kit-cell", 127u8, 0usize)))
            .unwrap()
    });
    let clip = ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new("kit-clip")).unwrap());
    assert!(
        rect.intersects(clip),
        "selected row {rect:?} outside {clip:?}"
    );
    a.player_selection.kit_open = true;
    let before = bytes(&a);
    assert_eq!(shared::owner(&a, &ctx), Screen::Waiting);
    key(&mut a, &ctx, Key::Space, egui::Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    assert!(!a.drum_editor.loop_enabled);
}
