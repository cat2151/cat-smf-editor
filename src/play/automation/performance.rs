use super::*;
pub(super) fn prepare(
    app: &App,
    parts: &[(usize, Patch, Sequence)],
    collection: &parameter_curve::Collection,
    plans: &mut Vec<Arc<OwnerPlan>>,
) -> Result<(), String> {
    let music = &app.loaded.as_ref().ok_or("No SMF is loaded")?.music;
    let windows = cat_smf_editor::score::performance::read(music)?;
    for part in parts
        .iter()
        .filter(|(owner, _, _)| windows.contains_key(owner))
    {
        let owner = part.0;
        if collection
            .curves
            .keys()
            .any(|&track| collection.owner(track) == Some(owner))
        {
            return Err("Performance expression windows cannot share an owner with ordinary parameter tracks".into());
        }
        let events = BTreeMap::from([(owner, music.tracks[owner].events.clone())]);
        let plan = super::pattern::prepare(
            app,
            std::slice::from_ref(part),
            &windows,
            &events,
            &TempoMap::new(music, owner),
        )?;
        plans.retain(|p| p.owner != owner);
        plans.extend(plan.owners.iter().cloned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_smf_editor::score::{
        pattern::{self, Catalog},
        scene::{
            self,
            arrangement::{self as grid, PatternOrScene},
        },
    };
    #[test]
    fn flattened_song_reopens_with_the_same_timing_sound_controls_and_window_gaps() {
        let (mut app, source, id) = crate::view::patterns::edit_tests::fixture();
        app.patterns.editing = None;
        crate::view::patterns::refresh(&mut app);
        let m = &mut app.loaded.as_mut().unwrap().music;
        let c = Catalog::read(m).unwrap();
        let p = c.patterns[&id].clone();
        pattern::automation::write(
            m,
            id,
            Curve {
                version: 1,
                target: parameter_curve::Target {
                    group: p.container,
                    layer: 1,
                    plugin_format: "CLAP".into(),
                    plugin_id: "performance-parameter-fixture".into(),
                    parameter_id: "17".into(),
                },
                value_space: parameter_curve::ValueSpace::Plain,
                minimum: 0.,
                maximum: 1.,
                name: "Cutoff".into(),
                path: "Filter".into(),
                points: vec![parameter_curve::Point {
                    tick: 0,
                    value: 0.8,
                }],
                controls: vec![],
            },
            Some(0.2),
        )
        .unwrap();
        pattern::automation::set_midi_baseline(m, p.container, 2, Some(11), 100).unwrap();
        cat_smf_editor::edit::insert(
            &mut m.tracks[source],
            cat_smf_editor::model::Event {
                tick: p.tick,
                delta: 0,
                status: 0xb2,
                meta_type: 0,
                data: vec![11, 70],
            },
        );
        cat_smf_editor::edit::refresh_deltas(&mut m.tracks[source]).unwrap();
        let lane = grid::add_lane(m, None).unwrap();
        grid::place(m, None, lane, 1, Some(PatternOrScene::Pattern(id))).unwrap();
        grid::place(m, None, lane, 3, Some(PatternOrScene::Pattern(id))).unwrap();
        scene::sizing::song(m, 4).unwrap();
        let original = crate::play::arrangement::prepare(&app, None).unwrap();
        let flat =
            cat_smf_editor::score::performance::song(&app.loaded.as_ref().unwrap().music).unwrap();
        let flat =
            cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(&flat).unwrap()).unwrap();
        let restored = crate::view::tests::app(flat);
        let next =
            crate::play::transport::prepare(&restored, crate::play::playback::Mode::AllTracks)
                .unwrap();
        assert!(
            next.active.automation.error.is_none(),
            "{:?}",
            next.active.automation.error
        );
        let sounded = next
            .parts
            .iter()
            .filter(|p| !p.2.notes.is_empty() || next.active.automation.owner(p.0).is_some())
            .collect::<Vec<_>>();
        assert_eq!(sounded.len(), 1);
        let part = sounded[0];
        assert_eq!(original.active.end, next.active.end);
        assert_eq!(original.parts[0].1, part.1);
        assert_eq!(
            original.parts[0]
                .2
                .events
                .iter()
                .map(|e| (e.time, e.word))
                .collect::<Vec<_>>(),
            part.2
                .events
                .iter()
                .map(|e| (e.time, e.word))
                .collect::<Vec<_>>()
        );
        let plan = &next.active.automation.owners[0];
        assert!(plan.midi);
        let curve = plan.curves[0].clone();
        let binding = Binding {
            windows: plan.windows[&curve.target].clone(),
            live: None,
            curve,
            value: Default::default(),
        };
        for (tick, value) in [(0., 0.8), (1920., 0.2), (3840., 0.8), (5760., 0.2)] {
            assert_eq!(binding.at(tick), Some(value));
        }
    }
}
