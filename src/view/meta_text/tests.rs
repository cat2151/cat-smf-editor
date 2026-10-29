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
fn guide_chords_keep_spelling_and_tick_coordinates_with_no_notes() {
    use cat_smf_editor::{edit, score::chord_guide};
    let mut music = edit::tracks::new_song();
    music.tracks[0].events.clear();
    chord_guide::add(&mut music).unwrap();
    for (tick, text) in [(0, "Isus4"), (1920, "I")] {
        edit::insert(
            &mut music.tracks[1],
            chord_guide::event(tick, text).unwrap(),
        );
    }
    let a = app(music.clone());
    let sources = crate::play::playback::performance(&a);
    let layout =
        crate::view::piano_roll::layout_from(a.loaded.as_ref().unwrap(), &sources, &[0], 0, 0);
    assert!(layout.size.x >= 1920.0 * layout.scale + crate::view::piano_roll::PX_PER_QUARTER);
    let c = egui::Context::default();
    let draw = |scale: f32, left: f32| {
        c.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 300.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show_music(
                        ui,
                        &music,
                        &Store::default(),
                        egui::Rect::from_min_max(egui::pos2(left, 20.0), egui::pos2(990.0, 280.0)),
                        scale,
                    );
                });
            },
        )
    };
    let first = draw(0.1, 20.0);
    assert_eq!(positions(&first, "Isus4").len(), 1);
    assert_eq!(positions(&first, "I").len(), 1);
    assert!((positions(&first, "I")[0].x - positions(&first, "Isus4")[0].x - 192.0).abs() < 0.01);
    let zoomed = draw(0.2, 20.0);
    let scrolled = draw(0.2, -80.0);
    assert!((positions(&zoomed, "I")[0].x - positions(&first, "I")[0].x - 192.0).abs() < 0.01);
    assert!((positions(&zoomed, "I")[0].x - positions(&scrolled, "I")[0].x - 100.0).abs() < 0.01);
}

#[test]
fn guide_chords_do_not_appear_on_raw_sources_or_local_projections() {
    use cat_smf_editor::{
        edit,
        score::{chord_guide, pattern},
    };
    let mut music = edit::tracks::new_song();
    chord_guide::add(&mut music).unwrap();
    edit::insert(
        &mut music.tracks[1],
        chord_guide::event(0, "Isus4").unwrap(),
    );
    let source = pattern::operations::create_source(&mut music).unwrap();
    let mut a = app(music);
    let c = egui::Context::default();
    let draw = |loaded: &crate::app::Loaded, track| {
        c.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, loaded, track, &Store::default(), ui.max_rect(), 0.1);
            });
        })
    };
    assert_eq!(
        positions(&draw(a.loaded.as_ref().unwrap(), 0), "Isus4").len(),
        1
    );
    assert!(positions(&draw(a.loaded.as_ref().unwrap(), source), "Isus4").is_empty());
    a.loaded.as_mut().unwrap().display_music = Some(a.loaded.as_ref().unwrap().music.clone());
    assert!(positions(&draw(a.loaded.as_ref().unwrap(), 0), "Isus4").is_empty());
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
        app.config.meta_text_visible = false;
        let output = render(&mut app, &ctx, vec![]);
        assert!(positions(&output, "Fermata").is_empty());
        assert!(positions(&output, "Ritardando").is_empty());
    }
}

#[test]
fn annotations_follow_tick_scale_scroll_and_visibility() {
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.tracks[0].events[0].tick = 480;
    loaded.music.tracks[1].events.clear();
    loaded.music.tracks[2].events.clear();
    let ctx = egui::Context::default();
    let draw = |scale, left, visible| {
        let config = Store {
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
                        0,
                        &config,
                        egui::Rect::from_min_max(egui::pos2(left, 20.0), egui::pos2(590.0, 280.0)),
                        scale,
                    );
                });
            },
        )
    };
    let first = positions(&draw(0.2, 20.0, true), "Fermata")[0];
    let zoomed = positions(&draw(0.4, 20.0, true), "Fermata")[0];
    let scrolled = positions(&draw(0.4, -30.0, true), "Fermata")[0];
    assert!((zoomed.x - first.x - 96.0).abs() < 0.01);
    assert!((zoomed.x - scrolled.x - 50.0).abs() < 0.01);
    assert!(positions(&draw(0.2, 20.0, false), "Fermata").is_empty());
    assert!(positions(&draw(0.2, -200.0, true), "Fermata").is_empty());
}
