use super::*;

#[test]
fn total_ticks_remain_in_raw_view_and_status_but_not_notes_status() {
    let mut app = fixture();
    playback::start(&mut app, Mode::Track);
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.5);
    let ctx = egui::Context::default();
    for note_mode in [true, false] {
        app.note_mode = note_mode;
        let output = render(&mut app, &ctx, None);
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"MBT"));
        assert_eq!(texts.contains(&"Tick"), !note_mode);
        let status = texts.iter().find(|t| t.contains("Playing")).unwrap();
        assert!(!status.contains("MBT"));
        assert!(status.contains("00:00.500 / 00:04.000"));
        assert!(status.contains("1|2|  0"));
        assert!(status.contains(" / "));
        assert!(!texts
            .iter()
            .any(|text| matches!(*text, "Follow (f)" | "Mixer (m)" | "Stop (Shift+Space)")));
        assert_eq!(status.contains(" · Tick "), !note_mode);
    }
}

#[test]
fn total_keeps_milliseconds_and_whole_song_duration_during_repeat() {
    let mut app = fixture();
    app.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .last_mut()
        .unwrap()
        .tick = 3841;
    transport::repeat::start(&mut app, (1920, 2400));
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.1);
    let ctx = egui::Context::default();
    let output = render(&mut app, &ctx, None);
    assert!(
        output.shapes.iter().any(|s| matches!(&s.shape,
        egui::epaint::Shape::Text(t) if t.galley.text().contains("Playing 00:02.100 / 00:04.001")))
    );
}

#[test]
fn transport_position_pads_tick_with_spaces_and_colors_only_the_beat() {
    let mut app = fixture();
    playback::start(&mut app, Mode::Track);
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.5011);
    let ctx = egui::Context::default();
    let output = render(&mut app, &ctx, None);
    let text = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text().contains("Playing") => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(
        text.galley.text().contains("1|2|  1"),
        "{}",
        text.galley.text()
    );
    let job = &text.galley.job;
    for section in &job.sections {
        let content = &job.text[section.byte_range.clone()];
        let is_beat = content == "2";
        assert_eq!(
            section.format.color,
            if is_beat {
                crate::view::beat_color(2)
            } else if content == "▶ Playing" {
                egui::Color32::LIGHT_GREEN
            } else {
                egui::Color32::LIGHT_GRAY
            }
        );
    }
}

fn status_text(output: &egui::FullOutput) -> &egui::epaint::TextShape {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t)
                if t.galley.text().starts_with("Stopped")
                    || t.galley.text().starts_with("▶ Playing") =>
            {
                Some(t)
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "{:?}",
                output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::epaint::Shape::Text(t) => Some(t.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            )
        })
}

fn text_x(text: &egui::epaint::TextShape, part: &str) -> f32 {
    let index = text.galley.text().find(part).unwrap();
    let glyph = text.galley.text()[..index].chars().count();
    text.pos.x + text.galley.rows[0].glyphs[glyph].pos.x
}

#[test]
fn stopped_cursor_time_and_position_align_with_playback_and_refresh_after_editing() {
    for note_mode in [true, false] {
        let mut app = fixture();
        app.note_mode = note_mode;
        app.track_view.cursor_tick = Some(1921);
        let ctx = egui::Context::default();
        render(&mut app, &ctx, None);
        let stopped = render(&mut app, &ctx, None);
        let stopped = status_text(&stopped);
        assert!(
            stopped
                .galley
                .text()
                .contains("Stopped 00:02.001 / 00:04.000 · 2|1|  1"),
            "{}",
            stopped.galley.text()
        );
        assert_eq!(stopped.galley.text().contains(" · Tick 1921"), !note_mode);
        playback::start(&mut app, Mode::Position);
        transport::advance(&mut app);
        app.audition.set_test_seconds(0.0001);
        let playing = render(&mut app, &ctx, None);
        let playing = status_text(&playing);
        for part in ["00:02.001", "2|1|  1"] {
            assert!((text_x(stopped, part) - text_x(playing, part)).abs() < 0.1);
        }
        transport::stop(&mut app);
        app.loaded.as_mut().unwrap().music.tracks[0].events.insert(
            0,
            Event {
                delta: 0,
                tick: 0,
                status: 0xff,
                meta_type: 0x51,
                data: vec![0x0f, 0x42, 0x40],
            },
        );
        app.loaded.as_mut().unwrap().refresh();
        app.generation += 1;
        let updated = render(&mut app, &ctx, None);
        assert!(status_text(&updated)
            .galley
            .text()
            .contains("00:04.002 / 00:08.000"));
    }
}

#[test]
fn measure_padding_keeps_beat_and_tick_aligned_between_cursor_one_and_playback_one_hundred() {
    let mut app = fixture();
    app.loaded.as_mut().unwrap().music.tracks.push(Track {
        events: vec![Event {
            delta: 0,
            tick: 200 * 1920,
            status: 0xff,
            meta_type: 0x2f,
            data: vec![],
        }],
    });
    app.loaded.as_mut().unwrap().refresh();
    app.generation += 1;
    app.track_view.cursor_tick = Some(1);
    let ctx = egui::Context::default();
    render(&mut app, &ctx, None);
    let stopped = render(&mut app, &ctx, None);
    let stopped = status_text(&stopped);
    assert!(
        stopped.galley.text().contains(" ·   1|1|  1"),
        "{}",
        stopped.galley.text()
    );
    playback::start(&mut app, Mode::Track);
    transport::advance(&mut app);
    // A displayed position may differ from the cursor while playback is running.
    app.playback.active.as_mut().unwrap().end = 400.0;
    app.audition.set_test_seconds(198.0011);
    let playing = render(&mut app, &ctx, None);
    let playing = status_text(&playing);
    assert!(
        playing.galley.text().contains(" · 100|1|  1"),
        "{}",
        playing.galley.text()
    );
    assert!((text_x(stopped, "|1|  1") - text_x(playing, "|1|  1")).abs() < 0.1);
}

#[test]
fn hour_visibility_follows_whole_song_length_and_refreshes_after_editing() {
    let mut app = fixture();
    app.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .last_mut()
        .unwrap()
        .tick = 3_600_000;
    let ctx = egui::Context::default();
    render(&mut app, &ctx, None);
    let long = render(&mut app, &ctx, None);
    assert!(status_text(&long)
        .galley
        .text()
        .contains("00:00:00.000 / 01:02:30.000"));
    app.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .last_mut()
        .unwrap()
        .tick = 3840;
    app.generation += 1;
    let short = render(&mut app, &ctx, None);
    assert!(status_text(&short)
        .galley
        .text()
        .contains("Stopped 00:00.000 / 00:04.000 · 1|1|  0"));
}
