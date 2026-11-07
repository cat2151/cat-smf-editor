use super::*;
use crate::play::{playback::Mode, transport};
use cat_smf_editor::edit::parameter_curve::{Curve, Point, Target, ValueSpace};

#[test]
fn an_unrelated_three_four_pattern_cannot_change_the_song_axis() {
    let (mut a, _, _, _) = fixture();
    let mut unrelated = 0;
    a.edit_tracks(|m| {
        unrelated = operations::create(m, 1, None)?;
        operations::resize(
            m,
            unrelated,
            vec![scene::Meter {
                numerator: 3,
                exponent: 2,
                ticks: 1440,
            }],
        )
    })
    .unwrap();
    crate::view::patterns::library::edit(&mut a, unrelated).unwrap();
    assert_eq!(
        a.loaded.as_ref().unwrap().time.map(0).unwrap().signature(0),
        (3, 2)
    );
    let (mut a, ctx) = opened_from(a);
    press(&mut a, &ctx, &[egui::Key::L]);
    assert_eq!(a.track_overview.cell.1, 1920);
    let p = Panes::read(&a, 0).unwrap();
    let starts: Vec<_> = p
        .overview
        .beats
        .iter()
        .filter(|(_, beat)| *beat == 1)
        .map(|(x, _)| (x / p.overview.scale).round() as u64)
        .collect();
    assert!(starts.contains(&1920));
    assert!(!starts.contains(&1440));
    press(&mut a, &ctx, &[egui::Key::L; 20]);
    assert_eq!(a.track_overview.cell.1, 7680);
}

#[test]
fn curve_instances_have_song_ticks_and_separate_clips_after_silence() {
    let (mut a, first, _, _) = fixture();
    let _instrument = crate::test_support::instrument::install(&mut a, 1);
    let curve = Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: "cutoff".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Cutoff".into(),
        path: String::new(),
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
            tick: 480,
            value: 0.5,
        }],
    };
    // Build saved curve data directly: drawing does not need a native parameter catalog.
    pattern::automation::write(
        &mut a.loaded.as_mut().unwrap().music,
        first,
        curve,
        Some(0.1),
    )
    .unwrap();
    a.loaded.as_mut().unwrap().refresh();
    a.generation += 1;
    let song = Song::read(&a).unwrap();
    let mut ranges: Vec<_> = song
        .rows
        .iter()
        .flat_map(|r| &r.curves)
        .map(|c| {
            (
                c.start,
                c.end,
                c.curve.points[0].tick,
                c.curve.points[1].tick,
            )
        })
        .collect();
    ranges.sort();
    assert_eq!(
        ranges,
        vec![
            (0, 1920, 0, 960),
            (3840, 5760, 3840, 4800),
            (7680, 9600, 7680, 8640)
        ]
    );
    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    open(&mut a);
    for _ in 0..3 {
        render(&mut a, &ctx, vec![]);
    }
    let output = render(&mut a, &ctx, vec![]);
    let paths: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Path(p) if p.stroke.width == 1.5 => Some(s.clip_rect),
            _ => None,
        })
        .collect();
    assert!(!paths.is_empty());
    // Each curve's clipping is bounded by one pattern; it never spans a silent measure.
    assert!(paths.iter().all(|r| r.width() <= 384.1));
}

#[test]
fn repeated_scene_jump_uses_that_occurrences_conductor_tempo() {
    let (mut a, first, _, s) = fixture();
    a.edit_tracks(|m| {
        let lane = scene::read(m)?.song_lanes[0];
        grid::place(m, None, lane, 7, Some(Scene(s)))?;
        cat_smf_editor::edit::insert(&mut m.tracks[0], event(11520, 0xff, 0x51, vec![15, 66, 64]));
        Ok(())
    })
    .unwrap();
    let (mut a, ctx) = opened_from(a);
    press(&mut a, &ctx, &[egui::Key::L; 7]);
    assert_eq!(a.track_overview.cell.1, 13440);
    press(&mut a, &ctx, &[egui::Key::G]);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, first);
    assert_eq!(
        crate::view::patterns::timing::pattern_tick(&a, first).unwrap(),
        13440
    );
    assert_eq!(
        crate::view::patterns::timing::pattern(&a, first)
            .unwrap()
            .duration(0, 480),
        1.0
    );
}

#[test]
fn a_multi_measure_body_jump_keeps_its_internal_measure() {
    let (mut a, first) = crate::view::tests::unplaced::fixture();
    a.edit_tracks(|m| {
        let meter = pattern::Catalog::read(m)?.patterns[&first].meters[0].clone();
        operations::resize(m, first, vec![meter.clone(), meter])?;
        let lane = grid::add_lane(m, None)?;
        grid::place(m, None, lane, 3, Some(Pattern(first)))
    })
    .unwrap();
    let (mut a, ctx) = opened_from(a);
    press(&mut a, &ctx, &[egui::Key::L; 3]);
    press(&mut a, &ctx, &[egui::Key::G]);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, first);
    assert_eq!(crate::play::playback::location(&a).0, 1920);
    assert_eq!(
        crate::view::patterns::timing::pattern_tick(&a, first).unwrap(),
        3840
    );
}

#[test]
fn empty_declared_song_and_empty_containers_never_reveal_body_storage() {
    let (mut a, _) = crate::view::tests::unplaced::fixture();
    a.edit_tracks(|m| scene::sizing::song(m, 6)).unwrap();
    let song = Song::read(&a).unwrap();
    assert_eq!(song.end, 11520);
    assert!(song
        .rows
        .iter()
        .all(|r| r.events.is_empty() && r.occurrences.is_empty()));
    let (mut a, ctx) = opened_from(a);
    press(&mut a, &ctx, &[egui::Key::L; 20]);
    assert_eq!(a.track_overview.cell.1, 9600);
    press(&mut a, &ctx, &[egui::Key::Enter]);
    let active = &a.playback.pending.as_ref().unwrap().active;
    assert_eq!(active.start_seconds, 10.0);
    assert_eq!(active.end, 2.0);
}

#[test]
fn candidate_audition_and_scene_playback_do_not_seek_or_draw_a_song_playhead() {
    let (mut a, first, _, s) = fixture();
    crate::view::patterns::preview::start(&mut a, first).unwrap();
    transport::advance(&mut a);
    a.audition.set_test_seconds(0.5);
    assert_eq!(
        crate::view::track_overview::song_playback::playing(&a),
        None
    );
    crate::view::track_overview::song_playback::seek(&mut a, 3840);
    assert_eq!(
        a.playback.active.as_ref().unwrap().mode,
        Mode::Pattern(first)
    );
    crate::play::arrangement::start(&mut a, Some(s));
    transport::advance(&mut a);
    a.audition.set_test_seconds(0.5);
    assert_eq!(
        crate::view::track_overview::song_playback::playing(&a),
        None
    );
}

#[test]
fn music_undo_and_reload_rebuild_the_same_song_projection() {
    let (mut a, first, _, _) = fixture();
    let expected = notes(&Panes::read(&a, 0).unwrap().overview);
    let bytes = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    a.edit_tracks(|m| {
        let lane = scene::read(m)?.song_lanes[0];
        grid::place(m, None, lane, 1, None)
    })
    .unwrap();
    assert_ne!(notes(&Panes::read(&a, 0).unwrap().overview), expected);
    a.undo_music().unwrap();
    assert_eq!(notes(&Panes::read(&a, 0).unwrap().overview), expected);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        bytes
    );
    let reloaded = crate::view::tests::app(cat_smf_editor::ffi::parse(&bytes).unwrap());
    assert_eq!(
        notes(&Panes::read(&reloaded, 0).unwrap().overview),
        expected
    );
    assert!(Song::read(&reloaded)
        .unwrap()
        .rows
        .iter()
        .any(|r| r.occurrences.iter().any(|o| o.pattern == first)));
}

#[test]
fn scoped_song_survives_transport_refresh_without_adding_other_containers() {
    let (mut a, _, _, _) = fixture();
    open(&mut a);
    play_track(&mut a, 1, 0);
    let saved = transport::timeline::capture(&a).unwrap();
    transport::timeline::resume(&mut a, saved);
    assert_eq!(playing_tracks(&a), vec![1]);
    assert!(a
        .playback
        .pending
        .as_ref()
        .unwrap()
        .active
        .containers
        .is_some());
}
