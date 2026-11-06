use super::*;

#[test]
fn a_pad_held_by_the_mouse_over_the_repeating_phrase_keeps_sounding() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let pad = cat_smf_editor::score::pattern::recording::Pad::Row { row: 0, column: 0 };
    let at = ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("pad-rect", pad))))
        .unwrap()
        .center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(
        &mut a,
        &ctx,
        vec![egui::Event::PointerMoved(at), button(true)],
        true,
    );
    assert!(recorder(&a).held(pad));
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![], true);
        assert!(recorder(&a).held(pad), "{:?}", session(&a).sound().looper);
    }
    frame(&mut a, &ctx, vec![button(false)], true);
    assert!(!recorder(&a).held(pad));
}

#[test]
fn a_drum_phrase_goes_on_under_the_next_which_replaces_only_the_lanes_it_plays() {
    let (mut a, track) = container();
    a.loaded.as_mut().unwrap().patches[track].drums = Some(Default::default());
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let (kick, snare) = (36, 38);
    let length = measure(&a);
    let quarter = length / 4;
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    rewind(&mut a, quarter);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    rewind(&mut a, length);
    frame(&mut a, &ctx, vec![], true);
    let Looper::Looping { start, notes, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    let (start, kicks) = (*start, notes.clone());
    // The phrase's laps, wherever the test moves the session clock.
    let first = session(&a).clock + start;
    let lap = |_: &App, n: u32| first + length * n;
    // Three quarters through the first lap, the snare.
    rewind(&mut a, length / 2);
    due(&a, Instant::now());
    frame(&mut a, &ctx, vec![key(Key::A, true, Modifiers::NONE)], true);
    let Looper::Capturing { origin, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    assert_eq!(*origin, start);
    rewind(&mut a, quarter / 2);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::A, false, Modifiers::NONE)],
        true,
    );
    // The kick goes on under the capture, and the lap after it already has the snare.
    let heard = due(&a, lap(&a, 2) - Duration::from_millis(1));
    let Looper::Capturing { notes: taken, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    let mut both = kicks.clone();
    both.extend(taken.iter().copied());
    assert_eq!(started(&heard, &both), vec![kick, snare]);
    rewind(&mut a, length / 2);
    frame(&mut a, &ctx, vec![], true);
    let Looper::Looping {
        start: kept, notes, ..
    } = &session(&a).sound().looper
    else {
        panic!("{:?}", session(&a).sound().looper)
    };
    assert_eq!(*kept, start);
    let mut pitches: Vec<u8> = notes.iter().filter(|n| n.2 > 0).map(|n| n.1).collect();
    pitches.sort();
    assert_eq!(pitches, vec![kick, snare]);
    let notes = notes.clone();
    let lap3 = due(&a, lap(&a, 3));
    let later = due(&a, lap(&a, 3) + length - Duration::from_millis(1));
    assert!(!lap3.is_empty() || !later.is_empty());
    assert_eq!(started(&later, &notes), vec![kick, snare]);
}

#[test]
fn a_melodic_phrase_still_stops_under_the_next_key() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    frame(&mut a, &ctx, vec![key(Key::X, true, Modifiers::NONE)], true);
    let Looper::Capturing { under, .. } = &session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    assert!(under.is_none());
    assert!(!session(&a).sound().live.phrase.playing());
}

#[test]
fn the_drum_sequencer_frames_the_step_the_drum_phrase_plays() {
    let (mut a, track) = container();
    // Saved in the song, so the Loop pattern's write keeps the container a drum one.
    let drums = cat_smf_editor::score::drums::Settings::default();
    let patch = Patch {
        kit: None,
        layers: drums
            .rows
            .iter()
            .map(|r| cat_smf_editor::score::instrument::ResolvedLayer {
                settings: cat_smf_editor::instrument_layers::Layer {
                    id: r.layer,
                    ..Default::default()
                },
                instrument: None,
            })
            .collect(),
        drums: Some(drums),
        effects: vec![],
    };
    crate::player::selection::store_track(&mut a, track, &patch).unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let length = measure(&a);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    rewind(&mut a, length / 4);
    let up = key(Key::Z, false, Modifiers::NONE);
    frame(&mut a, &ctx, vec![up], true);
    rewind(&mut a, length);
    frame(&mut a, &ctx, vec![], true);
    let id = looping_pattern(&a);
    if !a.note_mode {
        crate::view::patterns::mode::toggle(&mut a);
    }
    crate::view::drums::display(&mut a);
    assert!(crate::view::drums::is_open(&a));
    let Looper::Looping { start, .. } = session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    // Into the sixth sixteenth of a lap, half a step past its start.
    let into = (session(&a).clock.elapsed() - start).as_nanos() % length.as_nanos();
    let to = length * 5 / 16 + length / 32;
    rewind(&mut a, length - Duration::from_nanos(into as u64) + to);
    let grid = egui::Context::default();
    let mut framed = None;
    for _ in 0..2 {
        let _ = grid.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 900.0),
                )),
                ..Default::default()
            },
            |c| crate::view::drums::show(&mut a, c),
        );
        let cells = grid
            .data(|d| {
                d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(egui::Id::new(
                    "drum-grid-bounds",
                ))
            })
            .unwrap()
            .2;
        framed = grid
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("drum-playing")))
            .map(|f| {
                (0..cells.len())
                    .find(|&i| f.contains_rect(cells[i]))
                    .unwrap()
                    - 2
            });
    }
    assert_eq!(a.patterns.editing.as_ref().map(|e| e.pattern), Some(id));
    assert_eq!(framed, Some(5));
}
