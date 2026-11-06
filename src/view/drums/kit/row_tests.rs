use super::*;
use commands::{apply, Action};
fn app() -> App {
    let mut a = super::super::kit_edit_tests::fixture(128);
    let mut patch = a.loaded.as_ref().unwrap().patches[1].clone();
    patch.kit.as_mut().unwrap().rows = None;
    crate::player::selection::store_track(&mut a, 1, &patch).unwrap();
    super::super::open(&mut a);
    sync(&mut a);
    a
}
#[test]
fn default_four_rows_add_above_empty_assignment_and_undo_survive_save() {
    let mut a = app();
    assert_eq!(
        rows(&a).iter().map(|r| r.note).collect::<Vec<_>>(),
        drum_kit::layout::DEFAULT
    );
    apply(&mut a, Action::AddRows);
    assert!(a.drum_editor.error.is_none(), "{:?}", a.drum_editor.error);
    assert_eq!(
        rows(&a).iter().map(|r| r.note).collect::<Vec<_>>(),
        [
            None,
            None,
            None,
            None,
            Some(49),
            Some(42),
            Some(38),
            Some(36)
        ]
    );
    let before = a.loaded.as_ref().unwrap().music.clone();
    apply(&mut a, Action::Toggle);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&before).unwrap(),
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
    row_edit::assign(&mut a, 127).unwrap();
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[1]
            .kit
            .as_ref()
            .unwrap()
            .rows
            .as_ref()
            .unwrap()[0],
        Some(127)
    );
    let bytes = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let restored = cat_smf_editor::ffi::parse(&bytes).unwrap();
    assert_eq!(
        drum_kit::read(&restored.tracks[1])
            .unwrap()
            .unwrap()
            .rows
            .as_ref()
            .unwrap()[1],
        None
    );
    a.undo_music().unwrap();
    sync(&mut a);
    assert_eq!(rows(&a)[0].note, None);
}
#[test]
fn assignment_remaps_container_pattern_hits_and_offs_atomically() {
    use cat_smf_editor::score::pattern::{self, operations};
    let mut a = app();
    let mut music = a.loaded.as_ref().unwrap().music.clone();
    music.tracks[1].events.push(pattern::event(
        0,
        &pattern::Record::Container(pattern::ContainerRecord {
            version: pattern::VERSION,
            id: 1,
        }),
    ));
    let id = operations::create(&mut music, 1, None).unwrap();
    let context = pattern::editing::Context {
        track: 1,
        pattern: id,
        measure: 1,
        placement: None,
        scene: None,
        standalone: true,
    };
    let view = context.project(&music).unwrap();
    let mut edited = view.clone();
    drum_kit::toggle_step(&mut edited, 1, 36, 1, 0, 64).unwrap();
    music = context.map(&music, &view, &edited).unwrap();
    a.edit_tracks(|m| {
        *m = music;
        Ok(())
    })
    .unwrap();
    crate::view::patterns::library::edit(&mut a, id).unwrap();
    super::super::display(&mut a);
    a.drum_editor.kit.slot = 3;
    sync(&mut a);
    apply(&mut a, Action::Common(super::super::commands::Op::Loop));
    assert!(a.drum_editor.loop_enabled);
    row_edit::assign(&mut a, 35).unwrap();
    assert!(a.drum_editor.loop_enabled);
    assert_eq!(a.drum_editor.kit.slot, 3);
    assert_eq!(a.drum_editor.kit.note, Some(35));
    let events = &a.loaded.as_ref().unwrap().music.tracks[1].events;
    assert!(events
        .iter()
        .any(|e| cat_smf_editor::score::note_view::note_on(e) && e.data == [35, 64]));
    assert!(!events
        .iter()
        .any(|e| (cat_smf_editor::score::note_view::note_on(e)
            || cat_smf_editor::score::note_view::note_off(e))
            && e.data[0] == 36));
    assert!(super::super::is_open(&a));
    a.undo_music().unwrap();
    assert!(a.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .any(|e| cat_smf_editor::score::note_view::note_on(e) && e.data == [36, 64]));
}

#[test]
fn name_click_and_keyboard_picker_preview_ime_cancel_confirm() {
    use super::tests::{frame, key};
    let mut a = app();
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![], 1200.0);
    frame(&mut a, &ctx, vec![], 1200.0);
    let rect = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new(("kit-name", 0usize)))
            .unwrap()
    });
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::PointerMoved(rect.center()),
            egui::Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        1200.0,
    );
    frame(
        &mut a,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: rect.center(),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        1200.0,
    );
    assert!(a.drum_editor.kit.picker.is_some());
    assert!(a.audition.auditioned.is_empty());
    assert_eq!(
        crate::view::commands::owner(&a, &ctx),
        crate::view::commands::Screen::DrumInput
    );
    key(&mut a, &ctx, Key::Space, egui::Modifiers::NONE);
    assert_eq!(a.audition.auditioned, [49]);
    key(&mut a, &ctx, Key::R, egui::Modifiers::NONE);
    assert!(!a.drum_editor.loop_enabled);
    key(&mut a, &ctx, Key::Escape, egui::Modifiers::NONE);
    assert!(a.drum_editor.kit.picker.is_none());
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
    key(&mut a, &ctx, Key::N, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::Slash, egui::Modifiers::NONE);
    frame(&mut a, &ctx, vec![egui::Event::Text("35".into())], 1200.0);
    frame(
        &mut a,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("あ".into()))],
        1200.0,
    );
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    assert!(a.drum_editor.kit.picker.is_some());
    frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::Ime(egui::ImeEvent::Preedit(String::new())),
            egui::Event::Ime(egui::ImeEvent::Disabled),
        ],
        1200.0,
    );
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::Space, egui::Modifiers::NONE);
    assert_eq!(a.audition.auditioned.last(), Some(&35));
    key(&mut a, &ctx, Key::Enter, egui::Modifiers::NONE);
    assert!(a.drum_editor.error.is_none(), "{:?}", a.drum_editor.error);
    assert!(a.drum_editor.kit.picker.is_none());
    assert_eq!(rows(&a)[0].note, Some(35));
    key(&mut a, &ctx, Key::A, egui::Modifiers::NONE);
    key(&mut a, &ctx, Key::N, egui::Modifiers::NONE);
    assert!(a.drum_editor.kit.picker.is_some());
    key(&mut a, &ctx, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(rows(&a)[0].note, None);
}

#[test]
fn picker_reveals_selected_note_without_manual_scrolling() {
    use super::tests::{frame, key};
    let mut a = app();
    let ctx = egui::Context::default();
    key(&mut a, &ctx, Key::N, egui::Modifiers::NONE);
    for _ in 0..80 {
        key(&mut a, &ctx, Key::ArrowDown, egui::Modifiers::NONE);
    }
    for _ in 0..4 {
        frame(&mut a, &ctx, vec![], 1200.0);
    }
    let rect = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new(("kit-picker-note", 0u8)))
            .unwrap()
    });
    let clip = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new("kit-picker-clip"))
            .unwrap()
    });
    assert!(
        clip.contains(rect.min) && clip.contains(rect.max),
        "{rect:?} outside {clip:?}"
    );
    assert!(a.audition.auditioned.is_empty());
    assert_eq!(rows(&a).len(), 4);
}
