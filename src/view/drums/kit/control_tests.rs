use super::tests::{frame, key};
use super::*;

fn app() -> App {
    let mut a = super::super::kit_edit_tests::fixture(128);
    super::super::open(&mut a);
    sync(&mut a);
    a
}
fn wheel(a: &mut App, ctx: &egui::Context, id: egui::Id, up: bool) {
    frame(a, ctx, vec![], 1200.0);
    let rect = ctx.data(|d| d.get_temp::<egui::Rect>(id).unwrap());
    frame(
        a,
        ctx,
        vec![egui::Event::PointerMoved(rect.center())],
        1200.0,
    );
    frame(
        a,
        ctx,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, if up { 20.0 } else { -20.0 }),
            modifiers: egui::Modifiers::NONE,
        }],
        1200.0,
    );
}

#[test]
fn kit_velocity_wheel_uses_drum_decades_preserves_hits_and_is_undoable() {
    let mut a = app();
    let ctx = egui::Context::default();
    commands::apply(&mut a, commands::Action::Toggle);
    wheel(&mut a, &ctx, egui::Id::new(("kit-velocity", 0usize)), false);
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[1]
            .kit
            .as_ref()
            .unwrap()
            .velocity(0),
        90
    );
    assert_eq!(
        a.loaded.as_ref().unwrap().music.tracks[1]
            .events
            .iter()
            .find(|e| cat_smf_editor::score::note_view::note_on(e))
            .unwrap()
            .data[1],
        100
    );
    a.undo_music().unwrap();
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[1]
            .kit
            .as_ref()
            .unwrap()
            .velocity(0),
        100
    );
}

#[test]
fn kit_boundaries_have_drum_keys_wheel_and_input_ownership() {
    let mut a = app();
    let ctx = egui::Context::default();
    key(&mut a, &ctx, Key::CloseBracket, egui::Modifiers::SHIFT);
    assert_eq!((a.drum_editor.first, a.drum_editor.last), (1, 2));
    key(&mut a, &ctx, Key::CloseBracket, egui::Modifiers::NONE);
    assert_eq!((a.drum_editor.first, a.drum_editor.last), (2, 2));
    wheel(&mut a, &ctx, egui::Id::new(("kit-boundary", false)), false);
    assert_eq!((a.drum_editor.first, a.drum_editor.last), (1, 2));
    key(&mut a, &ctx, Key::Slash, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::CloseBracket, egui::Modifiers::SHIFT);
    key(&mut a, &ctx, Key::M, egui::Modifiers::NONE);
    assert_eq!((a.drum_editor.first, a.drum_editor.last), (1, 2));
    assert!(a.loaded.as_ref().unwrap().patches[1]
        .kit
        .as_ref()
        .unwrap()
        .muted
        .is_empty());
}

#[test]
fn kit_mute_keeps_events_roundtrips_remaps_and_undoes_without_loop_restart() {
    let mut a = app();
    let ctx = egui::Context::default();
    commands::apply(&mut a, commands::Action::Toggle);
    let events = a.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .filter(|e| e.status != 0xff)
        .map(|e| (e.tick, e.status, e.data.clone()))
        .collect::<Vec<_>>();
    key(&mut a, &ctx, Key::R, egui::Modifiers::NONE);
    crate::play::transport::advance(&mut a);
    let range = a.playback.repeat;
    key(&mut a, &ctx, Key::M, egui::Modifiers::NONE);
    assert!(a.playback.pending.is_none());
    assert_eq!(a.playback.repeat, range);
    let music = &a.loaded.as_ref().unwrap().music;
    assert_eq!(
        music.tracks[1]
            .events
            .iter()
            .filter(|e| e.status != 0xff)
            .map(|e| (e.tick, e.status, e.data.clone()))
            .collect::<Vec<_>>(),
        events
    );
    let restored =
        cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(music).unwrap()).unwrap();
    assert_eq!(
        drum_kit::read(&restored.tracks[1]).unwrap().unwrap().muted,
        [0]
    );
    a.undo_music().unwrap();
    assert!(a.loaded.as_ref().unwrap().patches[1]
        .kit
        .as_ref()
        .unwrap()
        .muted
        .is_empty());
    // An assigned row keeps its mute when its pitch changes.
    a.drum_editor.kit.query.clear();
    let mut patch = a.loaded.as_ref().unwrap().patches[1].clone();
    patch.kit.as_mut().unwrap().rows = Some(vec![Some(0)]);
    crate::player::selection::store_track(&mut a, 1, &patch).unwrap();
    sync(&mut a);
    controls::mute(&mut a).unwrap();
    row_edit::assign(&mut a, 127).unwrap();
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[1]
            .kit
            .as_ref()
            .unwrap()
            .muted,
        [127]
    );
}

#[test]
fn kit_swing_keyboard_edit_wheel_cancel_and_reload() {
    let mut a = app();
    let ctx = egui::Context::default();
    a.drum_editor.field = playback::measure(&a).unwrap().steps.len() + 3;
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::L, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    assert!(a.drum_editor.kit.input.as_ref().unwrap().swing);
    a.drum_editor.kit.input.as_mut().unwrap().text = "66".into();
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    wheel(&mut a, &ctx, egui::Id::new(("kit-swing", 0usize)), true);
    let music = &a.loaded.as_ref().unwrap().music;
    let restored =
        cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(music).unwrap()).unwrap();
    let kit = drum_kit::read(&restored.tracks[1]).unwrap().unwrap();
    assert!(kit.swing[&0].enabled);
    assert_eq!(kit.swing[&0].amount, 67);
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    a.drum_editor.kit.input.as_mut().unwrap().text = "30".into();
    key(&mut a, &ctx, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[1]
            .kit
            .as_ref()
            .unwrap()
            .swing[&0]
            .amount,
        67
    );
}

#[test]
fn swing_follows_names_and_keyboard_follows_that_order() {
    let mut a = app();
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![], 1200.0);
    }
    let rect = |key| {
        ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new((key, 0usize)))
                .unwrap()
        })
    };
    let cell = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new(("kit-cell", 0u8, 0usize)))
            .unwrap()
    });
    assert!(rect("kit-name").right() < rect("kit-swing-toggle").left());
    assert!(rect("kit-swing-toggle").right() < rect("kit-velocity").left());
    assert!(rect("kit-name").right() < rect("kit-velocity").left());
    assert!(rect("kit-velocity").right() < cell.left());
    a.drum_editor.field = 0;
    key(&mut a, &ctx, Key::L, egui::Modifiers::NONE);
    let steps = playback::measure(&a).unwrap().steps.len();
    assert_eq!(a.drum_editor.field, steps + 3);
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::L, egui::Modifiers::NONE);
    assert_eq!(a.drum_editor.field, steps + 4);
    key(&mut a, &ctx, Key::L, egui::Modifiers::NONE);
    assert_eq!(a.drum_editor.field, 1);
}
