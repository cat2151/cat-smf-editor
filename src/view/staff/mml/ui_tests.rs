use super::super::*;
use crate::app::App;
use cat_smf_editor::model::{Event, Music, Track};

fn fixture() -> App {
    let mut tracks = vec![Track {
        events: vec![Event {
            tick: 3840,
            delta: 3840,
            status: 0xff,
            meta_type: 0x2f,
            data: vec![],
        }],
    }];
    for pitches in [[60, 62], [64, 65]] {
        let events = pitches
            .into_iter()
            .enumerate()
            .flat_map(|(m, pitch)| {
                [
                    (m as u64 * 1920, 0x90, 80),
                    (m as u64 * 1920 + 1920, 0x80, 0),
                ]
                .map(|(tick, status, velocity)| Event {
                    tick,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![pitch, velocity],
                })
            })
            .collect();
        tracks.push(Track { events });
    }
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks,
    })
}

fn render(app: &mut App, ctx: &egui::Context) -> egui::FullOutput {
    crate::view::note_edit::tests::render(app, ctx, vec![])
}

fn mml(output: &egui::FullOutput) -> Vec<&egui::epaint::TextShape> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(t)
                if t.galley
                    .job
                    .sections
                    .first()
                    .is_some_and(|s| s.format.font_id == egui::FontId::monospace(14.0))
                    && t.galley
                        .job
                        .text
                        .starts_with(['l', 'r', 'c', 'd', 'e', 'f', 'g', 'a', 'b', ' '])
                    && t.galley
                        .job
                        .text
                        .chars()
                        .all(|c| "lcdefgabr+-. 0123456789'<>".contains(c)) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn conductor_renders_all_tracks_colored_and_only_one_measure_in_visible_bottom_panel() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let output = render(&mut app, &ctx);
    let texts = mml(&output);
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0].galley.job.text, "e1");
    assert_eq!(texts[1].galley.job.text, "c1");
    for (text, pitch) in texts.iter().zip([64, 60]) {
        let color = Palette::tinted(crate::view::pitch_keyboard::roll_color(pitch, None));
        assert!(text
            .galley
            .job
            .sections
            .iter()
            .any(|s| s.format.color == color));
        assert!(text.pos.y > app.staff.viewport.panel.unwrap().bottom());
        assert!(text.pos.y + text.galley.size().y < 900.0);
        assert!(output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Text(t)
            if t.pos == text.pos && s.clip_rect.contains(t.pos + egui::vec2(1.0, 1.0)))));
    }
    // Scrolling the score independently does not select or move MML measures.
    app.staff.viewport.pending = Some(egui::vec2(300.0, 30.0));
    let scrolled = render(&mut app, &ctx);
    assert_eq!(
        mml(&scrolled)
            .iter()
            .map(|t| t.galley.job.text.as_str())
            .collect::<Vec<_>>(),
        ["e1", "c1"]
    );
    assert_eq!(mml(&scrolled)[0].pos.x, texts[0].pos.x);
}

/// A click stops playback and A–B, then plays only the clicked row's track in the
/// shown measure, without leaving the current track.
#[test]
fn clicking_a_row_plays_only_that_track_in_the_shown_measure() {
    let mut app = fixture();
    app.track = 1;
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let output = render(&mut app, &ctx);
    let row = mml(&output)[0];
    assert_eq!(row.galley.job.text, "e1");
    let pos = row.pos + row.galley.size() / 2.0;
    crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
    app.playback.repeat = Some((0, 3840));
    for pressed in [true, false] {
        crate::view::note_edit::tests::render(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(app.track, 1);
    assert_eq!(app.playback.repeat, None);
    let active = &app
        .playback
        .pending
        .as_ref()
        .expect("measure audition")
        .active;
    assert_eq!(active.mode, crate::play::playback::Mode::Measure);
    assert_eq!(active.start_seconds, 0.0);
    assert_eq!(active.end, active.tempos[1].duration(0, 1920));
    assert_eq!(
        active
            .sources
            .iter()
            .map(|(track, s)| (*track, s.notes.iter().map(|n| n.pitch).collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
        [(2, vec![64])]
    );
}

#[test]
fn keyboard_track_and_measure_navigation_updates_colors_and_the_single_measure() {
    use crate::cursor::split::tests::press;
    let mut app = fixture();
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    press(
        &mut app,
        &ctx,
        &[(
            egui::Key::ArrowRight,
            egui::Modifiers::CTRL | egui::Modifiers::COMMAND,
        )],
    );
    assert_eq!(app.track, 1);
    let output = render(&mut app, &ctx);
    let texts = mml(&output);
    assert_eq!(texts.len(), 2);
    for (text, pitch) in texts.iter().zip([64, 60]) {
        let expected = if pitch == 60 {
            Palette::tinted(crate::view::pitch_keyboard::roll_color(pitch, None))
        } else {
            Color32::from_gray(140)
        };
        assert!(text
            .galley
            .job
            .sections
            .iter()
            .all(|s| s.format.color == expected));
    }
    press(&mut app, &ctx, &[(egui::Key::L, egui::Modifiers::NONE)]);
    assert_eq!(crate::play::playback::location(&app).0, 1920);
    let next = render(&mut app, &ctx);
    assert_eq!(
        mml(&next)
            .iter()
            .map(|t| t.galley.job.text.as_str())
            .collect::<Vec<_>>(),
        ["f1", "d1"]
    );
    press(&mut app, &ctx, &[(egui::Key::H, egui::Modifiers::NONE)]);
    assert_eq!(crate::play::playback::location(&app).0, 0);
}

#[test]
fn silent_measure_keeps_other_tracks_gray_and_empty_track_restores_colors() {
    let mut app = fixture();
    // Track 2 has notes only in the next measure; track 3 still sounds here.
    let mut music = app.loaded.take().unwrap().music;
    music.tracks[1].events.retain(|event| event.tick >= 1920);
    app = crate::view::tests::app(music);
    app.track = 1;
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let output = render(&mut app, &ctx);
    assert_eq!(mml(&output).len(), 1);
    assert!(mml(&output)[0]
        .galley
        .job
        .sections
        .iter()
        .all(|s| s.format.color == Color32::from_gray(140)));
    for shape in &output.shapes {
        if let egui::Shape::Text(t) = &shape.shape
            && t.galley.job.text == "track3"
        {
            assert_eq!(
                t.galley.job.sections[0].format.color,
                Color32::from_gray(140)
            );
        }
    }
    app.track = 0;
    let output = render(&mut app, &ctx);
    let expected = Palette::tinted(crate::view::pitch_keyboard::roll_color(64, None));
    assert!(mml(&output)[0]
        .galley
        .job
        .sections
        .iter()
        .all(|s| s.format.color == expected));
}

#[test]
fn measure_selection_uses_time_signature_boundaries() {
    let mut app = fixture();
    app.loaded.as_mut().unwrap().music.tracks[0].events.insert(
        0,
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x58,
            data: vec![3, 2, 24, 8],
        },
    );
    let loaded = app.loaded.as_mut().unwrap();
    loaded.time = cat_smf_editor::score::musical_time::MusicalTime::new(&loaded.music);
    assert_eq!(measure(loaded, 0, 1439), Some((0, 1440)));
    assert_eq!(measure(loaded, 1, 1440), Some((1440, 2880)));
}

#[test]
fn raw_and_split_modes_do_not_render_mml() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    app.note_mode = false;
    assert!(mml(&render(&mut app, &ctx)).is_empty());
    app.note_mode = true;
    app.track_view.split = Some([0, 1]);
    assert!(mml(&render(&mut app, &ctx)).is_empty());
}

fn render_size(app: &mut App, ctx: &egui::Context, height: f32) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, height),
            )),
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

#[test]
fn taller_windows_keep_the_staff_at_its_natural_height() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    render_size(&mut app, &ctx, 900.0);
    let normal = render_size(&mut app, &ctx, 900.0);
    let staff_height = app.staff.viewport.panel.unwrap().height();
    let mml_top = mml(&normal)[0].pos.y;
    render_size(&mut app, &ctx, 1300.0);
    let tall = render_size(&mut app, &ctx, 1300.0);
    assert!((app.staff.viewport.panel.unwrap().height() - staff_height).abs() < 1.0);
    assert!((mml(&tall)[0].pos.y - mml_top).abs() < 1.0);
    assert_eq!(
        app.staff.viewport.scroll.y, 0.0,
        "natural height fits the score"
    );
}

#[test]
fn twenty_four_mml_rows_fit_without_scrolling_past_a_large_empty_staff() {
    let tracks = (0..24)
        .map(|_| fixture().loaded.unwrap().music.tracks.pop().unwrap())
        .collect();
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks,
    });
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let output = render(&mut app, &ctx);
    assert_eq!(mml(&output).len(), 24);
    let visible = output
        .shapes
        .iter()
        .filter(|shape| {
            matches!(&shape.shape, egui::Shape::Text(t)
            if t.galley.job.text == "e1"
                && shape.clip_rect.contains_rect(egui::Rect::from_min_size(t.pos, t.galley.size())))
        })
        .count();
    assert_eq!(
        visible, 24,
        "all rows must actually be inside the MML viewport"
    );
}

#[test]
fn wide_pitch_ranges_yield_space_to_mml_and_remain_vertically_scrollable() {
    let tracks = (0..12)
        .map(|i| {
            let mut track = fixture().loaded.unwrap().music.tracks.pop().unwrap();
            for event in &mut track.events {
                event.data[0] = if i == 0 { 0 } else { 127 };
            }
            track
        })
        .collect();
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks,
    });
    let ctx = egui::Context::default();
    render_size(&mut app, &ctx, 650.0);
    let output = render_size(&mut app, &ctx, 650.0);
    let visible = output
        .shapes
        .iter()
        .filter(|shape| {
            matches!(&shape.shape, egui::Shape::Text(t)
            if (t.galley.job.text == "c1" || t.galley.job.text == "g1")
                && shape.clip_rect.contains_rect(egui::Rect::from_min_size(t.pos, t.galley.size())))
        })
        .count();
    assert_eq!(visible, 12);
    app.staff.viewport.pending = Some(egui::vec2(0.0, 10000.0));
    render_size(&mut app, &ctx, 650.0);
    assert!(
        app.staff.viewport.scroll.y > 0.0,
        "overflow notes remain reachable"
    );
}

#[test]
fn vertical_zoom_keeps_the_staff_area_and_the_zoom_bar() {
    fn vertical_bar(output: &egui::FullOutput) -> egui::Rect {
        let center = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.job.text == "zoom"
                        && t.galley.job.sections[0].format.font_id.size == 9.0 =>
                {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap();
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.rect.width() <= crate::view::timeline_zoom::BAR
                        && r.rect.height() > r.rect.width()
                        && r.rect.contains(center) =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .unwrap()
    }
    let mut app = fixture();
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let mut output = render(&mut app, &ctx);
    let original_bar = vertical_bar(&output).height();
    let original_staff = app.staff.viewport.panel.unwrap().height();
    for _ in 0..5 {
        let pos = vertical_bar(&output).center();
        for pressed in [true, false] {
            crate::view::note_edit::tests::render(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        output = render(&mut app, &ctx);
    }
    assert!(app.staff.viewport.zoom().y < 0.2);
    assert_eq!(app.staff.viewport.panel.unwrap().height(), original_staff);
    assert_eq!(vertical_bar(&output).height(), original_bar);
    assert_eq!(mml(&output).len(), 2);
}

#[path = "grid_tests.rs"]
mod grid;
