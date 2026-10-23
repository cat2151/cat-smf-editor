use super::*;
use cat_smf_editor::{
    edit::parameter_curve::{Curve, Point, Target, ValueSpace},
    score::{macro_curve, macro_values},
};

fn with_curves() -> App {
    let mut music = two_tracks().loaded.unwrap().music;
    let lane = |layer, value| Curve {
        version: 1,
        target: Target {
            group: 1,
            layer,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: "macro-fixture".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Macro 1".into(),
        path: "Macros".into(),
        points: vec![Point { tick: 0, value }],
        controls: vec![],
    };
    macro_values::write(
        &mut music.tracks[0],
        &macro_values::Values {
            version: 1,
            parameters: vec![lane(1, 0.2), lane(2, 0.2)],
            variations: (1..=2)
                .map(|layer| macro_values::Variations {
                    layer,
                    selected: 0,
                    snapshots: vec![vec![lane(layer, 0.2)], vec![lane(layer, 0.8)]],
                })
                .collect(),
        },
    )
    .unwrap();
    for layer in 1..=2 {
        let mut c = lane(layer, 0.0);
        c.target.parameter_id = "macro-variation".into();
        c.target.plugin_id = "CLAP:org.surge-synth-team.surge-xt".into();
        c.points = vec![
            Point {
                tick: 960,
                value: 0.0,
            },
            Point {
                tick: 20000,
                value: 1.0,
            },
        ];
        c.controls = vec![Point {
            tick: 4000,
            value: 3.0,
        }];
        macro_curve::write(&mut music.tracks[0], &c).unwrap();
    }
    app(music)
}

#[test]
fn maps_show_only_their_tracks_layers_in_notes_and_raw_modes_without_writing() {
    for note_mode in [true, false] {
        let mut app = with_curves();
        app.note_mode = note_mode;
        let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
        for track in [0, 1] {
            let ctx = egui::Context::default();
            let mut rect = egui::Rect::NOTHING;
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    rect = ui.available_rect_before_wrap();
                    crate::view::track_map::show(&app, ui, track);
                });
            });
            let curves: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::QuadraticBezier(b)
                        if b.stroke.color
                            == egui::epaint::ColorMode::Solid(super::super::variations::COLOR) =>
                    {
                        Some(b)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(curves.len(), if track == 0 { 2 } else { 0 });
            for c in curves {
                // The shared value axis includes the outlying control; time includes
                // curve points beyond the last note (13920) rather than clipping them.
                assert!((c.points[1].x - (rect.right() - 2.0)).abs() < 0.01);
                assert!((c.points[2].y - rect.bottom()).abs() < 0.01);
                assert!(
                    (c.points[0].y - rect.top() - rect.height() * 960.0 / 20000.0).abs() < 0.01
                );
            }
        }
        assert_eq!(
            cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
            before
        );
    }
}
