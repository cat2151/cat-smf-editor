use super::navigation::{click, zone};
use super::*;

pub(super) fn fixture() -> crate::app::App {
    let low = with_notes(&[(0, 480, 40), (960, 480, 48)])
        .loaded
        .take()
        .unwrap()
        .music
        .tracks
        .remove(0);
    let high = with_notes(&[(0, 480, 72), (960, 480, 84)])
        .loaded
        .take()
        .unwrap()
        .music
        .tracks
        .remove(0);
    let conductor = cat_smf_editor::model::Track {
        events: vec![event(15360, 0xff, vec![])],
    };
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![conductor, low, high],
    });
    app.select_track(1);
    app.track_view.split = Some([1, 2]);
    app
}

#[test]
fn higher_range_is_on_top_in_equal_halves_and_both_panes_edit_their_own_track() {
    let mut app = fixture();
    let (ctx, output) = opened_in(&mut app);
    let tracks = super::super::panes::tracks(&app, app.loaded.as_ref().unwrap());
    assert_eq!(tracks, vec![2, 1]);
    let panel = app.piano_roll.viewport.panel.unwrap();
    let rects = all_notes(&output);
    assert_eq!(rects.len(), 4);
    assert!(rects[..2].iter().all(|r| r.center().y < panel.center().y));
    assert!(rects[2..].iter().all(|r| r.center().y > panel.center().y));
    let pos = zone(&app, 0);
    click(&mut app, &ctx, 11.0, pos, true);
    let output = at(&mut app, &ctx, 11.1, vec![]);
    let notes = all_notes(&output);
    assert!((notes[0].width() - 64.0).abs() < 0.1);
    assert_eq!(notes[0].width(), notes[2].width());
    for (time, index, track) in [(12.0, 0, 2), (13.0, 2, 1)] {
        let output = at(&mut app, &ctx, time, vec![]);
        let note = all_notes(&output)[index];
        super::drag::drag(
            &mut app,
            &ctx,
            time + 0.1,
            note.center(),
            note.center() + egui::vec2(12.0, 0.0),
        );
        let events = &app.loaded.as_ref().unwrap().music.tracks[track].events;
        assert_eq!(
            events
                .iter()
                .find(|e| cat_smf_editor::score::note_view::note_on(e))
                .unwrap()
                .tick,
            90
        );
    }
    assert_eq!(app.track_view.split, Some([1, 2]));
}

#[test]
fn duplicate_tracks_empty_tracks_and_tied_ranges_keep_stable_order() {
    let mut app = fixture();
    app.track_view.split = Some([1, 1]);
    let (ctx, output) = opened_in(&mut app);
    assert_eq!(all_notes(&output).len(), 4);
    let note = all_notes(&output)[2];
    super::drag::drag(
        &mut app,
        &ctx,
        11.0,
        note.center(),
        note.center() + egui::vec2(24.0, 0.0),
    );
    assert!(app.piano_roll.error.is_none());
    assert_eq!(
        app.loaded.as_ref().unwrap().music.tracks[1].events[0].tick,
        120
    );
    app.track_view.split = Some([0, 1]);
    assert_eq!(
        super::super::panes::tracks(&app, app.loaded.as_ref().unwrap()),
        vec![1, 0]
    );
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.tracks[2] = loaded.music.tracks[1].clone();
    loaded.note_views[2] =
        cat_smf_editor::score::note_view::NoteView::new(&loaded.music.tracks[2].events);
    app.track_view.split = Some([2, 1]);
    assert_eq!(
        super::super::panes::tracks(&app, app.loaded.as_ref().unwrap()),
        vec![2, 1]
    );
}

#[test]
fn resizing_keeps_two_equal_clipped_panes_and_repeat_replays_all_tracks() {
    let mut app = fixture();
    let (ctx, _) = opened_in(&mut app);
    for (time, size) in [
        (11.0, egui::vec2(1100.0, 700.0)),
        (12.0, egui::vec2(1500.0, 900.0)),
    ] {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                time: Some(time),
                ..Default::default()
            },
            |ctx| crate::view::show(&mut app, ctx),
        );
        let panel = app.piano_roll.viewport.panel.unwrap();
        let halves: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r)
                    if (r.rect.height() - panel.height() / 2.0).abs() < 0.1
                        && (r.rect.width() - panel.width()).abs() < 0.1
                        && r.stroke.width == 1.0 =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(halves.len(), 2);
        assert!((halves[0].bottom() - halves[1].top()).abs() < 0.1);
        let mut scroll_style = ctx.style().spacing.scroll;
        scroll_style.floating = false;
        assert!((panel.right() + scroll_style.allocated_width() - size.x).abs() < 0.1);
    }
    let output = at(&mut app, &ctx, 13.0, vec![]);
    let note = all_notes(&output)[0];
    let row = app.piano_roll.repeat_rect.unwrap();
    let from = egui::pos2(note.left(), row.center().y);
    super::drag::drag(&mut app, &ctx, 14.0, from, from + egui::vec2(192.0, 0.0));
    assert_eq!(app.playback.repeat, Some((0, 960)));
    let active = app.playback.active.as_ref().unwrap();
    assert_eq!(active.sources.len(), 3);
    assert!(active.sources[1..].iter().all(|(_, s)| !s.notes.is_empty()));
    app.audition.set_test_seconds(1.0);
    at(&mut app, &ctx, 15.0, vec![]);
    assert_eq!(
        app.playback.pending.as_ref().unwrap().active.start_seconds,
        0.0
    );
    press(&mut app, &ctx, egui::Key::R);
    assert!(!app.piano_roll.open);
    assert_eq!(app.playback.repeat, Some((0, 960)));
}
