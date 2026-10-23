use super::*;

fn text(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push(t.galley.job.text.clone()),
        egui::epaint::Shape::Vec(shapes) => shapes.iter().for_each(|s| text(s, out)),
        _ => {}
    }
}
fn draw(a: &mut App, c: &egui::Context, view: View) -> Vec<String> {
    let mut state = preview::State::default();
    let mut labels = vec![];
    for _ in 0..3 {
        labels.clear();
        let output = c.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| preview::show(a, ctx, &mut state, view),
        );
        for s in &output.shapes {
            text(&s.shape, &mut labels);
        }
    }
    labels
}
#[test]
fn double_score_draws_actual_event_columns_and_preserves_the_selected_pair() {
    let mut a = fixture();
    a.note_mode = false;
    a.track_view.split = Some([1, 0]);
    a.track_view.active = 1;
    a.track_view.scroll_tick = 240;
    let c = egui::Context::default();
    let labels = draw(&mut a, &c, View::DoubleScore);
    for header in ["MBT", "Note", "Duration", "Velocity"] {
        assert_eq!(
            labels.iter().filter(|s| s.as_str() == header).count(),
            2,
            "{header}: {labels:?}"
        );
    }
    assert_eq!(a.track_view.split, Some([1, 0]));
    assert_eq!(a.track_view.active, 1);
    assert_eq!(a.track_view.scroll_tick, 240);
    assert!(!a.note_mode);
    assert_eq!(a.track, 0);
    assert!(!crate::play::transport::timeline::running(&a));
}
#[test]
fn score_mixer_and_layer_previews_include_the_normal_screen_controls() {
    let mut a = fixture();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let c = egui::Context::default();
    let labels = draw(&mut a, &c, View::Score(1));
    assert!(labels.iter().any(|s| s == "MBT"), "{labels:?}");
    let labels = draw(&mut a, &c, View::Mixer);
    for control in ["Master auto gain", "Master effects"] {
        assert!(labels.iter().any(|s| s == control), "{labels:?}");
    }
    let labels = draw(&mut a, &c, View::Instruments(1));
    for control in ["Instrument layers", "+", "M", "S", "−"] {
        assert!(labels.iter().any(|s| s == control), "{labels:?}");
    }
    let labels = draw(&mut a, &c, View::Effects(1));
    for control in ["Track effects", "+", "Mixer"] {
        assert!(labels.iter().any(|s| s == control), "{labels:?}");
    }
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!a.playback.mixer_open && !a.layer_editor.open && !a.effect_editor.is_open());
    assert_eq!(a.track, 0);
}

#[test]
fn macros_draw_saved_values_polarity_and_variations_without_starting_audio() {
    use cat_smf_editor::{
        chain_settings::PluginState,
        edit::parameter_curve::{Curve, Point, Target as ParameterTarget, ValueSpace},
        host::PluginKey,
        score::macro_values::{self, Values, Variations},
    };
    let mut a = fixture();
    let xml = br#"<patch><parameters><p><modrouting source="7" depth="1"/></p></parameters><customcontroller><entry i="0" v="-0.5" bipolar="1" label="Cutoff"/></customcontroller></patch>"#;
    let mut state = b"sub3".to_vec();
    state.extend((xml.len() as u32).to_le_bytes());
    state.extend([0; 24]);
    state.extend(xml);
    a.loaded.as_mut().unwrap().patches[1].layers[0].instrument = Some(PluginState {
        plugin: PluginKey {
            format: "CLAP".into(),
            id: "org.surge-synth-team.surge-xt".into(),
            name: "Surge XT".into(),
            vendor: "Surge".into(),
            bundle_path: "X:/surge.clap".into(),
        },
        state,
        reference: None,
        preset_name: None,
        bypass: false,
    });
    let curve = Curve {
        version: 1,
        target: ParameterTarget {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: "macro1".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Macro 1 · Cutoff".into(),
        path: "Macros".into(),
        points: vec![Point {
            tick: 0,
            value: 0.75,
        }],
        controls: vec![],
    };
    macro_values::write(
        &mut a.loaded.as_mut().unwrap().music.tracks[1],
        &Values {
            version: 1,
            parameters: vec![curve.clone()],
            variations: vec![Variations {
                layer: 1,
                selected: 1,
                snapshots: vec![vec![curve.clone()], vec![curve]],
            }],
        },
    )
    .unwrap();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let labels = draw(&mut a, &egui::Context::default(), View::Macros(1, 1));
    for control in [
        "Macro 1 · Cutoff",
        "Macro +0.500  ·  pos 0.750",
        "Variation",
        "1",
        "2",
        "複製 [D]",
        "確定・終了 [Enter]",
        "取消 [Esc]",
    ] {
        assert!(labels.iter().any(|s| s == control), "{control}: {labels:?}");
    }
    assert!(labels.iter().any(|s| s.contains("bipolar [-1, 1]")));
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!a.macro_editor.open());
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!(a.track, 0);
}

#[test]
fn score_confirmation_leaves_the_double_column_view() {
    let mut a = fixture();
    a.track_view.split = Some([0, 1]);
    let c = egui::Context::default();
    browse(&mut a, &c);
    select(&mut a, Target::Level(Level::Track(1)));
    press(&mut a, &c, Key::L);
    select(&mut a, Target::View(View::Score(1)));
    press(&mut a, &c, Key::Enter);
    assert!(!a.palette.is_open());
    assert_eq!(a.track_view.split, None);
    assert_eq!(a.track, 1);
}
