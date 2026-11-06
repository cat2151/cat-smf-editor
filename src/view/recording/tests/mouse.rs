use super::super::play as playing;
use super::*;
use cat_smf_editor::score::pattern::recording::Pad;
use window::Pointer;

fn pointer(at: egui::Pos2, pressed: Option<bool>) -> Vec<egui::Event> {
    let mut events = vec![egui::Event::PointerMoved(at)];
    events.extend(pressed.map(|pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }));
    events
}

#[test]
fn a_press_on_a_key_follows_the_pointer_across_keys_until_released() {
    let ctx = egui::Context::default();
    let keys = [
        (
            Pad::Pitch(60),
            egui::Rect::from_min_size(egui::pos2(100., 100.), egui::vec2(20., 20.)),
        ),
        (
            Pad::Pitch(62),
            egui::Rect::from_min_size(egui::pos2(120., 100.), egui::vec2(20., 20.)),
        ),
    ];
    let run = |events| {
        let mut found = None;
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let keys: Vec<_> = keys
                        .iter()
                        .map(|&(pad, rect)| {
                            (
                                pad,
                                ui.interact(
                                    rect,
                                    egui::Id::new(rect.min.x as i32),
                                    egui::Sense::drag(),
                                ),
                            )
                        })
                        .collect();
                    found = window::pointed(ui, &keys);
                });
            },
        );
        found
    };
    let (a, b, outside) = (
        egui::pos2(110., 110.),
        egui::pos2(130., 110.),
        egui::pos2(300., 300.),
    );
    run(vec![]);
    // A press that began outside every key never sounds one.
    assert_eq!(run(pointer(outside, Some(true))), None);
    assert_eq!(run(pointer(a, None)), None);
    assert_eq!(run(pointer(a, Some(false))), None);
    assert_eq!(run(pointer(a, Some(true))), Some(Pad::Pitch(60)));
    assert_eq!(run(pointer(b, None)), Some(Pad::Pitch(62)));
    assert_eq!(run(pointer(outside, None)), None);
    assert_eq!(run(pointer(a, None)), Some(Pad::Pitch(60)));
    assert_eq!(run(pointer(a, Some(false))), None);
}

#[test]
fn a_piano_key_held_by_the_mouse_survives_the_keyboard_losing_focus() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    let s = a.patterns.recording.session.as_mut().unwrap();
    // The test has no pointer, so the piano window would let go of the key it holds.
    s.piano_open = false;
    playing::point(s, Pointer::Piano, Some(Pad::Pitch(70)));
    assert!(recorder(&a).sounding(60) && recorder(&a).sounding(70));
    // Clicking the piano window takes the focus from the keyboard window.
    frame(&mut a, &ctx, vec![], false);
    assert!(!recorder(&a).sounding(60) && recorder(&a).sounding(70));
    let s = a.patterns.recording.session.as_mut().unwrap();
    playing::point(s, Pointer::Piano, Some(Pad::Pitch(72)));
    assert!(!recorder(&a).sounding(70) && recorder(&a).sounding(72));
    let s = a.patterns.recording.session.as_mut().unwrap();
    playing::point(s, Pointer::Piano, None);
    assert!(!(0..128).any(|p| recorder(&a).sounding(p)));
}

#[test]
fn a_take_records_what_the_mouse_plays() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Space, true, Modifiers::NONE)],
        true,
    );
    let s = a.patterns.recording.session.as_mut().unwrap();
    playing::point(s, Pointer::Pads, Some(Pad::Row { row: 0, column: 0 }));
    s.clock -= Duration::from_millis(500);
    playing::point(s, Pointer::Pads, None);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Escape, true, Modifiers::NONE)],
        true,
    );
    assert!(a.patterns.recording.is_open());
    let music = &a.loaded.as_ref().unwrap().music;
    let c = Catalog::read(music).unwrap();
    let p = &c.patterns[&a.patterns.selected.unwrap()];
    let b = body::extract(&music.tracks[p.track].events, p.range().unwrap()).unwrap();
    let on: Vec<u8> = b
        .iter()
        .filter(|e| e.status == 0x90)
        .map(|e| e.data[0])
        .collect();
    assert_eq!(on, [60]);
}

#[test]
fn the_container_menu_switches_to_a_drum_container_after_a_phrase_key() {
    let mut a = app();
    for _ in 0..2 {
        a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
            .unwrap();
    }
    let melodic = a.loaded.as_ref().unwrap().music.tracks.len() - 1;
    let drums = melodic - 1;
    a.loaded.as_mut().unwrap().patches[drums].drums = Some(Default::default());
    a.track = melodic;
    a.config.phrase_loop = true;
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let mut state = std::mem::take(&mut a.patterns.recording);
    let s = state.session.as_mut().unwrap();
    playing::point(s, Pointer::Piano, Some(Pad::Pitch(60)));
    playing::point(s, Pointer::Piano, None);
    assert!(!playing::armed(s));
    let index = s.sounds.iter().position(|x| x.track == drums).unwrap();
    assert_ne!(s.selected, index);
    assert!(!playing::button(&mut a, s, window::Button::Sound(index)));
    a.patterns.recording = state;
    assert_eq!(session(&a).selected, index);
    assert!(session(&a).error.is_none(), "{:?}", session(&a).error);
    assert!(recorder(&a).settings().drums);
}

fn plus(a: &mut App) {
    let mut state = std::mem::take(&mut a.patterns.recording);
    assert!(!playing::button(
        a,
        state.session.as_mut().unwrap(),
        window::Button::NewSound
    ));
    let s = state.session.as_mut().unwrap();
    assert_eq!(s.new_sound, Some(0));
    s.new_sound = None;
    assert!(!playing::button(a, s, window::Button::CreateSound(false)));
    a.patterns.recording = state;
}

#[test]
fn the_container_menu_plus_adds_a_container_without_patterns_and_plays_it() {
    let mut a = app();
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    assert_eq!(session(&a).sounds.len(), 1);
    plus(&mut a);
    let s = session(&a);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.sounds.len(), 2);
    assert_eq!(s.selected, 1);
    let (id, track) = (s.sounds[1].container.unwrap(), s.sounds[1].track);
    assert_eq!(a.patterns.container, Some(id));
    assert!(a.audition.live_active());
    assert!(a.audition.live_patches.iter().any(|(t, _)| *t == track));
    let music = &a.loaded.as_ref().unwrap().music;
    assert!(Catalog::read(music).unwrap().patterns.is_empty());
    assert_eq!(music.tracks[track].name().as_deref(), Some("Init Saw 2"));
    // The next frames neither list it twice nor close the keyboard.
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    assert!(a.patterns.recording.is_open());
    assert_eq!(session(&a).sounds.len(), 2);
    assert!(recorder(&a).sounding(60));
    assert!(a.undo.available());
}

#[test]
fn the_container_menu_plus_turns_the_stand_in_sound_into_the_new_container() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    assert_eq!(session(&a).sounds[0].container, None);
    plus(&mut a);
    let s = session(&a);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.sounds.len(), 1);
    assert_eq!(s.sounds[0].container, a.patterns.container);
    assert!(s.sounds[0].container.is_some());
}

#[test]
fn a_container_added_elsewhere_joins_the_keyboard_without_taking_the_selection() {
    let mut a = app();
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let id = crate::view::patterns::library::new_container(&mut a, false).unwrap();
    frame(&mut a, &ctx, vec![], true);
    let s = session(&a);
    assert!(a.patterns.recording.is_open());
    assert_eq!(s.sounds.len(), 2);
    assert_eq!(s.sounds[1].container, Some(id));
    assert_eq!(s.selected, 0);
    assert!(a
        .audition
        .live_patches
        .iter()
        .any(|(t, _)| *t == s.sounds[1].track));
}
