use super::*;
use crate::view::{note_edit::tests::render, piano_roll::tests::press, tests::app};
use cat_smf_editor::model::{Event, Music, Track};

fn fixture() -> crate::app::App {
    let event = |tick, meta_type, text: &str| Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type,
        data: text.as_bytes().to_vec(),
    };
    app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![event(0, 1, "Fermata"), event(3840, 0x2f, "")],
            },
            Track {
                events: vec![
                    event(0, 1, "Breath"),
                    event(480, 3, "Track title"),
                    event(3840, 0x2f, ""),
                ],
            },
            Track {
                events: vec![event(0, 1, "Ritardando"), event(3840, 0x2f, "")],
            },
        ],
    })
}

fn positions(output: &egui::FullOutput, label: &str) -> Vec<egui::Pos2> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(t.pos),
            _ => None,
        })
        .collect()
}

#[test]
fn both_overlays_show_all_tracks_at_ticks_and_respect_display_settings() {
    for track in [0, 1] {
        let mut app = fixture();
        app.select_track(track);
        let ctx = egui::Context::default();
        press(&mut app, &ctx, egui::Key::R);
        let output = render(&mut app, &ctx, vec![]);
        let count = if track == 0 { 2 } else { 1 };
        let fermata = positions(&output, "Fermata");
        let breath = positions(&output, "Breath");
        let rit = positions(&output, "Ritardando");
        assert_eq!(fermata.len(), count);
        assert_eq!(breath.len(), count);
        assert_eq!(rit.len(), count);
        for i in 0..count {
            assert_eq!(fermata[i].x, breath[i].x);
            assert!(breath[i].y > fermata[i].y);
            assert!(rit[i].y > breath[i].y);
        }
        app.config.meta_text_icons = true;
        let output = render(&mut app, &ctx, vec![]);
        assert!(positions(&output, "Fermata").is_empty());
        app.config.meta_text_icons = false;
        app.config.meta_text_visible = false;
        let output = render(&mut app, &ctx, vec![]);
        assert!(positions(&output, "Fermata").is_empty());
        assert!(positions(&output, "Ritardando").is_empty());
    }
}

#[test]
fn annotations_follow_tick_scale_scroll_and_icons_remain_visible() {
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.tracks[0].events[0].tick = 480;
    loaded.music.tracks[1].events.clear();
    loaded.music.tracks[2].events.clear();
    let ctx = egui::Context::default();
    let draw = |scale, left, icons, visible| {
        let config = Store {
            meta_text_icons: icons,
            meta_text_visible: visible,
            ..Store::default()
        };
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 300.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _ = show(
                        ui,
                        loaded,
                        &config,
                        egui::Rect::from_min_max(egui::pos2(left, 20.0), egui::pos2(590.0, 280.0)),
                        scale,
                    );
                });
            },
        )
    };
    let first = positions(&draw(0.2, 20.0, false, true), "Fermata")[0];
    let zoomed = positions(&draw(0.4, 20.0, false, true), "Fermata")[0];
    let scrolled = positions(&draw(0.4, -30.0, false, true), "Fermata")[0];
    assert!((zoomed.x - first.x - 96.0).abs() < 0.01);
    assert!((zoomed.x - scrolled.x - 50.0).abs() < 0.01);
    let icon = draw(0.2, 20.0, true, true);
    assert!(positions(&icon, "Fermata").is_empty());
    let bubbles = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter(|s| {
                matches!(
                    &s.shape, egui::epaint::Shape::Rect(r) if r.stroke.color == COLOR
                )
            })
            .count()
    };
    assert_eq!(bubbles(&icon), 1);
    assert_eq!(bubbles(&draw(0.2, 20.0, true, false)), 0);
    assert_eq!(bubbles(&draw(0.2, -200.0, true, true)), 0);
}
