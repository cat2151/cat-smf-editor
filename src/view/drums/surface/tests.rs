use super::*;

fn app(kit: bool) -> App {
    let mut a = if kit {
        super::super::kit_edit_tests::fixture(4)
    } else {
        super::super::tests::fixture()
    };
    super::super::open(&mut a);
    a
}

fn frame(a: &mut App, c: &egui::Context, time: f64, focused: bool) -> egui::FullOutput {
    let mut input = egui::RawInput {
        time: Some(time),
        screen_rect: Some(Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .focused = Some(focused);
    c.run(input, |ctx| super::super::show(a, ctx))
}

fn cell(c: &egui::Context, kit: bool, row: usize, step: usize) -> Rect {
    if kit {
        c.data(|d| {
            d.get_temp(egui::Id::new(("kit-cell", row as u8, step)))
                .unwrap()
        })
    } else {
        c.data(|d| d.get_temp::<(Rect, Rect, Vec<Rect>)>(egui::Id::new("drum-grid-bounds")))
            .unwrap()
            .2[(3 - row) * 18 + step + 2]
    }
}

#[test]
fn both_grids_follow_native_and_browser_focus_and_show_step_numbers() {
    for kit in [false, true] {
        let mut a = app(kit);
        let c = egui::Context::default();
        c.style_mut(|s| s.animation_time = 0.0);
        for (time, native, browser, color, width) in [
            (0.0, true, false, crate::view::ACTIVE_PANE, 3.0),
            (0.35, true, false, Color32::WHITE, 3.0),
            (0.35, false, false, crate::view::ACTIVE_PANE, 2.0),
            (0.35, true, true, crate::view::ACTIVE_PANE, 2.0),
        ] {
            a.player_selection.open = browser;
            let mut out = frame(&mut a, &c, time, native);
            for _ in 0..2 {
                out = frame(&mut a, &c, time, native);
            }
            let rect = cell(&c, kit, 0, 0);
            assert!(out.shapes.iter().any(|s| matches!(&s.shape,
                egui::epaint::Shape::Rect(r) if r.rect == rect && r.stroke == Stroke::new(width, color)
            )), "kit {kit}, native {native}, browser {browser}");
            assert_eq!(
                out.shapes.iter().any(|s| matches!(&s.shape,
                    egui::epaint::Shape::Rect(r) if r.rect == rect.shrink(width)
                        && r.stroke == Stroke::new(1.0_f32, Color32::from_gray(24))
                )),
                native && !browser
            );
            assert!(out.shapes.iter().any(|s| matches!(&s.shape,
                egui::epaint::Shape::Text(t) if t.galley.text() == "16" && t.pos.y < rect.top()
            )));
        }
    }
}

#[test]
fn both_grids_frame_playing_column_without_replacing_the_selection_border() {
    for kit in [false, true] {
        let mut a = app(kit);
        let c = egui::Context::default();
        c.style_mut(|s| s.animation_time = 0.0);
        frame(&mut a, &c, 0.0, true);
        super::super::tests::press(&mut a, &c, Key::F);
        a.drum_editor.loop_enabled = true;
        playback::restart(&mut a).unwrap();
        crate::play::transport::advance(&mut a);
        a.audition.set_test_seconds(0.01);
        let out = frame(&mut a, &c, 0.0, true);
        let playing: Rect = c.data(|d| d.get_temp(egui::Id::new("drum-playing")).unwrap());
        assert!(playing.contains_rect(cell(&c, kit, 0, 0)));
        assert!(playing.contains_rect(cell(&c, kit, 3, 0)));
        assert!(!playing.intersects(cell(&c, kit, 0, 1)));
        assert!(out.shapes.iter().any(|s| matches!(&s.shape,
            egui::epaint::Shape::Rect(r) if r.rect == cell(&c, kit, 0, 0)
                && r.stroke == Stroke::new(3.0_f32, crate::view::ACTIVE_PANE)
        )));
    }
}

#[test]
fn both_empty_patterns_highlight_first_beat_until_a_hit_is_entered() {
    for kit in [false, true] {
        use cat_smf_editor::score::pattern::{self, operations};
        let mut a = app(kit);
        let mut music = a.loaded.as_ref().unwrap().music.clone();
        music.tracks[1].events.push(pattern::event(
            0,
            &pattern::Record::Container(pattern::ContainerRecord {
                version: pattern::VERSION,
                id: 1,
            }),
        ));
        let id = operations::create(&mut music, 1, None).unwrap();
        a.edit_tracks(|m| {
            *m = music;
            Ok(())
        })
        .unwrap();
        a.note_mode = true;
        crate::view::patterns::library::edit(&mut a, id).unwrap();
        super::super::display(&mut a);
        let c = egui::Context::default();
        for _ in 0..3 {
            frame(&mut a, &c, 0.0, true);
        }
        let beat: Rect = c
            .data(|d| d.get_temp(egui::Id::new("drum-beacon")))
            .expect("empty pattern beacon");
        assert!(beat.contains_rect(cell(&c, kit, 0, 0)));
        assert!(beat.contains_rect(cell(&c, kit, 0, 3)));
        assert!(!beat.intersects(cell(&c, kit, 0, 4)));
        super::super::tests::press(&mut a, &c, Key::F);
        c.data_mut(|d| d.remove::<Rect>(egui::Id::new("drum-beacon")));
        frame(&mut a, &c, 0.0, true);
        assert!(c
            .data(|d| d.get_temp::<Rect>(egui::Id::new("drum-beacon")))
            .is_none());
    }
}
