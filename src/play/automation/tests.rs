use super::*;
use crate::play::{
    playback::{self, Mode},
    transport,
};
use cat_smf_editor::{
    chain_settings::PluginState,
    edit::{
        parameter_curve::{Point, Target, ValueSpace},
        tracks,
    },
    host::PluginKey,
};

fn fixture() -> App {
    let mut music = tracks::new_song();
    parameter_curve::create(
        &mut music,
        0,
        Curve {
            version: 1,
            target: Target {
                group: 0,
                layer: 1,
                plugin_format: "CLAP".into(),
                plugin_id: "fixture".into(),
                parameter_id: "101".into(),
            },
            value_space: ValueSpace::Plain,
            minimum: 0.0,
            maximum: 1.0,
            name: "Cutoff".into(),
            path: "Filter".into(),
            points: vec![
                Point {
                    tick: 0,
                    value: 0.2,
                },
                Point {
                    tick: 960,
                    value: 0.8,
                },
            ],
            controls: vec![Point {
                tick: 400,
                value: 0.9,
            }],
        },
    )
    .unwrap();
    let mut app = crate::view::tests::app(music);
    app.loaded.as_mut().unwrap().patches[0].layers[0].instrument = Some(PluginState {
        reference: None,
        plugin: PluginKey {
            format: "CLAP".into(),
            id: "fixture".into(),
            name: "Fixture".into(),
            vendor: String::new(),
            bundle_path: "X:/fixture.clap".into(),
        },
        state: vec![],
        preset_name: None,
        bypass: false,
    });
    app
}

#[test]
fn automation_scope_uses_one_owner_and_curve_tail_without_an_extra_native_part() {
    let mut app = fixture();
    app.track = 1;
    let prepared = transport::prepare(&app, Mode::Track).unwrap();
    assert_eq!(
        prepared.parts.iter().map(|p| p.0).collect::<Vec<_>>(),
        vec![0]
    );
    assert!(prepared.active.automation.error.is_none());
    assert_eq!(prepared.active.automation.owners.len(), 1);
    assert_eq!(prepared.active.automation.owners[0].curves.len(), 1);
    assert_eq!(prepared.active.end, 1.0);
    let all = transport::prepare(&app, Mode::AllTracks).unwrap();
    assert_eq!(all.parts.iter().map(|p| p.0).collect::<Vec<_>>(), vec![0]);
    app.track = 0;
    assert_eq!(
        transport::prepare(&app, Mode::Track)
            .unwrap()
            .active
            .automation
            .owners
            .len(),
        1
    );
}

fn evaluation_points(plan: &RuntimePlan, fragment: usize, end: u64) -> Vec<(u64, u64)> {
    let mut frame = 0;
    let mut values = Vec::new();
    while frame < end {
        let callback_end = (frame + fragment as u64).min(end);
        while frame < callback_end {
            if plan.applies(frame) {
                values.push((frame, plan.tick(frame).to_bits()));
            }
            let n = plan.next(frame, (callback_end - frame) as usize);
            assert!(n > 0);
            frame += n as u64;
        }
    }
    values
}

#[test]
fn automation_sample_grid_is_independent_of_callback_fragmentation_and_keeps_fractional_ticks() {
    let app = fixture();
    let p = transport::prepare(&app, Mode::AllTracks).unwrap();
    let runtime = RuntimePlan::new(
        p.active.automation.owners[0].clone(),
        &p.parts[0].2,
        0.123,
        48000,
    );
    let full = evaluation_points(&runtime, 1024, 12000);
    for fragment in [1, 17, 63, 127, 251, 512] {
        assert_eq!(full, evaluation_points(&runtime, fragment, 12000));
    }
    assert_eq!(runtime.tick(0), runtime.owner.tempo.fractional_tick(0.123));
    assert!(full
        .iter()
        .any(|(_, tick)| f64::from_bits(*tick).fract() != 0.0));
    let curve = &runtime.owner.curves[0];
    assert_ne!(
        curve.value_at(runtime.tick(0)),
        curve.value_at(runtime.tick(12000))
    );
}

#[test]
fn automation_seek_and_repeat_restore_beginning_value_and_range() {
    let app = fixture();
    let all = transport::prepare(&app, Mode::AllTracks).unwrap();
    let seek = crate::play::selection_playback::skip(&app, all.active, 0.25).unwrap();
    let runtime = RuntimePlan::new(
        seek.active.automation.owners[0].clone(),
        &seek.parts[0].2,
        seek.active.start_seconds,
        48000,
    );
    assert_eq!(runtime.tick(0), 240.0);
    let repeated = transport::repeat::prepare(&app, (240, 720)).unwrap();
    assert_eq!(repeated.active.start_seconds, 0.25);
    assert_eq!(repeated.active.end, 0.5);
    let head = RuntimePlan::new(
        repeated.active.automation.owners[0].clone(),
        &repeated.parts[0].2,
        repeated.active.start_seconds,
        48000,
    );
    assert_eq!(head.tick(0), runtime.tick(0));
}

#[test]
fn automation_missing_target_stops_and_empty_curve_is_inert() {
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    let mut c = parameter_curve::read(&loaded.music).unwrap().curves[&1].clone();
    c.target.plugin_id = "missing".into();
    let e = loaded.music.tracks[1]
        .events
        .iter_mut()
        .find(|e| parameter_curve::owned(e))
        .unwrap();
    e.data = parameter_curve::PREFIX.to_vec();
    e.data.extend(serde_json::to_vec(&c).unwrap());
    let failed = transport::prepare(&app, Mode::AllTracks).unwrap();
    assert!(failed
        .active
        .automation
        .error
        .as_ref()
        .unwrap()
        .contains("Instrument changed"));
    app.playback.pending = Some(failed);
    transport::advance(&mut app);
    assert!(app.playback.active.is_none() && app.playback.pending.is_none());
    assert!(app.load_error.as_ref().unwrap().contains("Automation"));
    // Empty curves are inert even when their target is unresolved.
    let loaded = app.loaded.as_mut().unwrap();
    c.points.clear();
    c.controls.clear();
    parameter_curve::write(&mut loaded.music, 1, &c).unwrap();
    assert!(!transport::prepare(&app, Mode::AllTracks)
        .unwrap()
        .active
        .automation
        .active());
    assert!(transport::prepare(&app, Mode::AllTracks)
        .unwrap()
        .active
        .automation
        .error
        .is_none());
    assert!(!playback::parts(&app, Mode::AllTracks).is_empty());
}

#[test]
fn automation_unrelated_owner_is_playable_and_voicevox_or_conflict_is_rejected() {
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.tracks.push(loaded.music.tracks[1].clone());
    loaded.patches.push(Patch::default());
    loaded.refresh();
    assert!(transport::prepare(&app, Mode::AllTracks)
        .unwrap()
        .active
        .automation
        .error
        .as_ref()
        .unwrap()
        .contains("Conflicting"));
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    cat_smf_editor::voicevox::voice::Voice::write(
        &mut loaded.music.tracks[0],
        Some(&Default::default()),
    )
    .unwrap();
    assert!(transport::prepare(&app, Mode::AllTracks)
        .unwrap()
        .active
        .automation
        .error
        .as_ref()
        .unwrap()
        .contains("VOICEVOX"));
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.tracks[0]
        .events
        .retain(|e| !parameter_curve::group_event(e));
    tracks::add(&mut loaded.music).unwrap();
    loaded.patches.push(Patch::default());
    loaded.refresh();
    app.track = 2;
    let scope = transport::prepare(&app, Mode::Track).unwrap();
    assert_eq!(scope.parts.iter().map(|p| p.0).collect::<Vec<_>>(), vec![2]);
    assert!(scope.active.automation.error.is_none());
    assert!(transport::prepare(&app, Mode::AllTracks)
        .unwrap()
        .active
        .automation
        .error
        .is_some());
}

#[test]
fn automation_layer_batch_precedes_notes_and_preserves_other_layers_and_effect_sum() {
    use crate::play::layer_render::{Effect, Instrument, Render};
    use cat_smf_editor::{host::parameters::ParameterValue, instrument_layers::Layer};
    use std::{cell::RefCell, rc::Rc};
    #[derive(Debug, PartialEq)]
    enum Action {
        Parameters(Vec<f64>),
        Note(usize, u8),
        Effect,
    }
    struct InstrumentMock(usize, Rc<RefCell<Vec<Action>>>);
    impl Instrument for InstrumentMock {
        fn parameters(&mut self, values: &[ParameterValue]) -> Result<(), i32> {
            self.1.borrow_mut().push(Action::Parameters(
                values.iter().map(|v| v.value()).collect(),
            ));
            Ok(())
        }
        fn render(
            &mut self,
            v: &[u8; 128],
            _: &[u64; 128],
            _: i32,
            out: &mut [f32],
            _: usize,
        ) -> bool {
            self.1.borrow_mut().push(Action::Note(self.0, v[60]));
            out.fill(1.0);
            true
        }
    }
    struct EffectMock(Rc<RefCell<Vec<Action>>>);
    impl Effect for EffectMock {
        fn apply(&mut self, _: &mut [f32], _: usize) -> bool {
            self.0.borrow_mut().push(Action::Effect);
            true
        }
    }
    let app = fixture();
    let prepared = transport::prepare(&app, Mode::AllTracks).unwrap();
    let owner = prepared.active.automation.owners[0].clone();
    let runtime = RuntimePlan::new(owner.clone(), &prepared.parts[0].2, 0.0, 48000);
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut render = Render::new(
        vec![
            (InstrumentMock(0, actions.clone()), Layer::default()),
            (
                InstrumentMock(1, actions.clone()),
                Layer {
                    id: 2,
                    ..Default::default()
                },
            ),
        ],
        Some(EffectMock(actions.clone())),
    );
    let token = ParameterValue::default();
    render.set_automation(vec![LayerBindings {
        layer: 0,
        lanes: vec![Binding {
            windows: Default::default(),
            live: None,
            curve: owner.curves[0].clone(),
            value: token,
        }],
        values: vec![token],
    }]);
    let mut velocities = [0; 128];
    velocities[60] = 100;
    let mut output = [0.0; 128];
    assert!(render.automate(Some(&runtime), 0));
    assert!(render.process(&velocities, &[48000; 128], 1, &mut output, 2));
    assert_eq!(
        *actions.borrow(),
        vec![
            Action::Parameters(vec![0.2]),
            Action::Note(0, 100),
            Action::Note(1, 100),
            Action::Effect
        ]
    );
    assert_eq!(output[0], 2.0);
    actions.borrow_mut().clear();
    assert!(render.automate(Some(&runtime), 17));
    assert!(actions.borrow().is_empty());
    assert!(render.automate(None, 64));
    assert!(actions.borrow().is_empty());
    render.set_automation(vec![]);
    assert!(!render.automate(Some(&runtime), 64));
    assert_eq!(render.parameter_error, Some(-6));
}
