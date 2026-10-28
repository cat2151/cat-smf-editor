use super::*;
use std::ops::{Deref, DerefMut};

// Keep fingerprint inputs and thread-local roots alive through edits and Undo.
pub(super) struct Fixture {
    app: App,
    _instrument: crate::test_support::instrument::Instrument,
}
impl Deref for Fixture {
    type Target = App;
    fn deref(&self) -> &App {
        &self.app
    }
}
impl DerefMut for Fixture {
    fn deref_mut(&mut self) -> &mut App {
        &mut self.app
    }
}

pub(super) fn fixture() -> Fixture {
    let mut music = tracks::new_song();
    let lane = |value| Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: "macro-fixture".into(),
        },
        minimum: 0.0,
        maximum: 1.0,
        value_space: ValueSpace::Plain,
        name: "Macro 1".into(),
        path: "Macros".into(),
        points: vec![Point { tick: 0, value }],
        controls: vec![],
    };
    macro_values::write(
        &mut music.tracks[0],
        &Values {
            version: 1,
            parameters: vec![lane(0.2)],
            variations: vec![Variations {
                layer: 1,
                selected: 0,
                snapshots: vec![vec![lane(0.2)], vec![lane(0.8)], vec![lane(0.4)]],
            }],
        },
    )
    .unwrap();
    let mut app = crate::view::tests::app(music);
    let instrument = crate::test_support::instrument::install(&mut app, 0);
    let mut c = lane(0.0);
    c.target.parameter_id = "macro-variation".into();
    c.target.plugin_id = "CLAP:org.surge-synth-team.surge-xt".into();
    c.maximum = 2.0;
    c.points.clear();
    macro_curve::write(&mut app.loaded.as_mut().unwrap().music.tracks[0], &c).unwrap();
    app.parameter_editor = State {
        open: true,
        variation_layer: Some(1),
        owner: Some(0),
        ..Default::default()
    };
    Fixture {
        app,
        _instrument: instrument,
    }
}

pub(super) fn index_curve() -> Curve {
    Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "CLAP:org.surge-synth-team.surge-xt".into(),
            parameter_id: "macro-variation".into(),
        },
        minimum: 0.0,
        maximum: 2.0,
        value_space: ValueSpace::Plain,
        name: "Macro variation curve".into(),
        path: "Macros".into(),
        points: vec![
            Point {
                tick: 0,
                value: 0.0,
            },
            Point {
                tick: 480,
                value: 1.0,
            },
            Point {
                tick: 960,
                value: 2.0,
            },
        ],
        controls: vec![
            Point {
                tick: 240,
                value: 0.0,
            },
            Point {
                tick: 720,
                value: 1.5,
            },
        ],
    }
}
