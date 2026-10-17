use super::super::keyboard::{sounding, WIDTH};
use super::*;

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    }
}

fn key(app: &crate::app::App, note: egui::Rect) -> egui::Pos2 {
    egui::pos2(
        app.piano_roll.viewport.panel.unwrap().left() - WIDTH / 2.0,
        note.center().y,
    )
}

#[test]
fn press_glissando_unused_keys_exit_reentry_and_release_do_not_edit_the_song() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let c = key(&app, all_notes(&output)[0]);
    let e = key(&app, all_notes(&output)[1]);
    let unused = c - egui::vec2(0.0, ROW_HEIGHT);
    let generation = app.generation;
    let cursor = app.selected_event;
    at(
        &mut app,
        &ctx,
        11.0,
        vec![egui::Event::PointerMoved(c), button(c, true)],
    );
    assert_eq!(sounding(&app), Some((0, 60)));
    assert_eq!(app.audition.auditioned, [60]);
    at(
        &mut app,
        &ctx,
        11.1,
        vec![egui::Event::PointerMoved(unused)],
    );
    assert_eq!(sounding(&app), Some((0, 61)));
    assert_eq!(app.audition.auditioned, [61]);
    at(&mut app, &ctx, 11.2, vec![egui::Event::PointerMoved(e)]);
    assert_eq!(sounding(&app), Some((0, 64)));
    assert_eq!(app.audition.auditioned, [64]);
    at(
        &mut app,
        &ctx,
        11.3,
        vec![egui::Event::PointerMoved(e + egui::vec2(WIDTH, 0.0))],
    );
    assert_eq!(sounding(&app), None);
    at(&mut app, &ctx, 11.4, vec![egui::Event::PointerMoved(c)]);
    assert_eq!(sounding(&app), Some((0, 60)));
    at(&mut app, &ctx, 11.5, vec![button(c, false)]);
    assert_eq!(sounding(&app), None);
    assert_eq!(app.generation, generation);
    assert_eq!(app.selected_event, cursor);
    assert!(app.playback.active.is_none() && app.playback.pending.is_none());
}

#[test]
fn closing_or_losing_focus_stops_a_held_note_and_hover_cannot_start_it() {
    for close in [false, true] {
        let mut app = arpeggio();
        let (ctx, output) = opened_in(&mut app);
        let c = key(&app, all_notes(&output)[0]);
        at(&mut app, &ctx, 11.0, vec![egui::Event::PointerMoved(c)]);
        assert_eq!(sounding(&app), None);
        at(&mut app, &ctx, 11.1, vec![button(c, true)]);
        assert_eq!(sounding(&app), Some((0, 60)));
        if close {
            press(&mut app, &ctx, egui::Key::Escape);
        } else {
            at(
                &mut app,
                &ctx,
                11.2,
                vec![egui::Event::WindowFocused(false)],
            );
        }
        assert_eq!(sounding(&app), None);
    }
}

#[test]
fn split_keyboards_use_their_own_track_and_follow_scrolled_zoomed_note_rows() {
    let mut app = super::split::fixture();
    let (ctx, _) = opened_in(&mut app);
    let zoom = super::navigation::zone(&app, 1);
    super::navigation::click(&mut app, &ctx, 11.0, zoom, false);
    app.piano_roll.viewport.pending = Some(egui::vec2(200.0, 30.0));
    let output = at(&mut app, &ctx, 12.0, vec![]);
    let notes = all_notes(&output);
    let high = key(&app, notes[1]);
    let low = key(&app, notes[3]);
    at(
        &mut app,
        &ctx,
        12.1,
        vec![egui::Event::PointerMoved(high), button(high, true)],
    );
    assert_eq!(sounding(&app), Some((2, 84)));
    at(&mut app, &ctx, 12.2, vec![egui::Event::PointerMoved(low)]);
    assert_eq!(sounding(&app), Some((1, 48)));
    at(&mut app, &ctx, 12.3, vec![button(low, false)]);
    assert_eq!(sounding(&app), None);
}

#[test]
fn playback_started_while_holding_a_key_survives_its_release() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let c = key(&app, all_notes(&output)[0]);
    at(
        &mut app,
        &ctx,
        11.0,
        vec![egui::Event::PointerMoved(c), button(c, true)],
    );
    assert_eq!(sounding(&app), Some((0, 60)));
    press(&mut app, &ctx, egui::Key::Space);
    assert_eq!(sounding(&app), None);
    at(&mut app, &ctx, 12.0, vec![button(c, false)]);
    assert!(app.playback.active.is_some());
    assert!(app.audition.seconds().is_some());
}

#[test]
fn live_keys_preserve_playback_clock_repeat_and_song_through_release_and_close() {
    for close in [false, true] {
        let mut app = arpeggio();
        let (ctx, output) = opened_in(&mut app);
        let c = key(&app, all_notes(&output)[0]);
        app.playback.follow = false;
        crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
        crate::play::transport::advance(&mut app);
        let repeat = (0, 1920);
        app.playback.repeat = Some(repeat);
        app.audition.set_test_seconds(0.25);
        let generation = app.generation;
        at(
            &mut app,
            &ctx,
            11.0,
            vec![egui::Event::PointerMoved(c), button(c, true)],
        );
        assert_eq!(sounding(&app), Some((0, 60)));
        assert!(app.playback.active.is_some());
        assert_eq!(app.audition.seconds(), Some(0.25));
        assert_eq!(app.playback.repeat, Some(repeat));
        let d = c - egui::vec2(0.0, ROW_HEIGHT);
        app.audition.set_test_seconds(0.5);
        at(&mut app, &ctx, 11.1, vec![egui::Event::PointerMoved(d)]);
        assert_eq!(sounding(&app), Some((0, 61)));
        assert_eq!(app.audition.seconds(), Some(0.5));
        if close {
            press(&mut app, &ctx, egui::Key::Escape);
        } else {
            at(&mut app, &ctx, 11.2, vec![button(d, false)]);
        }
        assert_eq!(sounding(&app), None);
        assert!(!app.audition.key_active());
        assert!(app.playback.active.is_some());
        assert_eq!(app.audition.seconds(), Some(0.5));
        assert_eq!(app.playback.repeat, Some(repeat));
        assert_eq!(app.generation, generation);
    }
}

#[test]
fn held_keys_use_the_existing_twelve_pitch_colors_including_unused_black_keys() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let c = key(&app, all_notes(&output)[0]);
    for pitch in 60..72 {
        let pos = c - egui::vec2(0.0, f32::from(pitch - 60) * ROW_HEIGHT);
        let mut events = vec![egui::Event::PointerMoved(pos)];
        if pitch == 60 {
            events.push(button(pos, true));
        }
        let output = at(&mut app, &ctx, 11.0 + f64::from(pitch - 60) * 0.1, events);
        assert_eq!(sounding(&app), Some((0, pitch)));
        let fills = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                Shape::Rect(rect) if rect.rect.contains(pos) => Some(rect.fill),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            fills.last().copied(),
            Some(crate::view::pitch_keyboard::roll_color(pitch, None))
        );
    }
}
