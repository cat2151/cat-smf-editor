use super::*;
use cat_smf_editor::edit::{reference_tempo, trill};

fn trill_song() -> Music {
    let mut song = music();
    song.tracks[1] = notes(
        &(0..8)
            .map(|i| (100 + i * 24, 24, 60 + (i % 2) as u8))
            .collect::<Vec<_>>(),
    );
    song
}

#[test]
fn seconds_control_changes_saved_metadata_and_note_count() {
    let mut app = app(trill_song());
    let (ctx, output) = opened_in(&mut app, 0);
    let text_position = |output: &egui::FullOutput, needle: &str| {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Text(t) if t.galley.text().contains(needle) => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing label {needle}"))
    };
    let header = text_position(&output, "Trill Start");
    super::edit::click(&mut app, &ctx, 11.0, header, egui::PointerButton::Primary);
    let output = at(&mut app, &ctx, 12.0, vec![]);
    let input = text_position(&output, "0.025");
    let to = input + egui::vec2(30.0, 0.0);
    super::edit::hold(&mut app, &ctx, 13.0, input, to);
    super::edit::release(&mut app, &ctx, 13.1, to);
    assert!(app.trill_edit.is_some());
    let output = at(&mut app, &ctx, 14.0, vec![]);
    let apply = text_position(&output, "Apply");
    super::edit::click(&mut app, &ctx, 15.0, apply, egui::PointerButton::Primary);
    let music = &app.loaded.as_ref().unwrap().music;
    let region = trill::read(music).unwrap().remove(0);
    assert!(region.metadata.first_note_seconds.unwrap() > 0.025);
    let count = music.tracks[1]
        .events
        .iter()
        .filter(|e| cat_smf_editor::score::note_view::note_on(e))
        .count();
    assert!(count < 8);
    let bytes = cat_smf_editor::smf_io::encode(music).unwrap();
    assert_eq!(
        trill::read(&cat_smf_editor::ffi::parse(&bytes).unwrap()).unwrap(),
        [region]
    );
}

#[test]
fn curve_changes_reflow_notes_and_invalid_interval_rolls_back_the_app_edit() {
    let mut app = app(trill_song());
    opened_in(&mut app, 0);
    let prepared = super::super::trills::prepare(&mut app).unwrap();
    let mut expected = (*prepared).clone();
    cat_smf_editor::edit::regenerate(&mut expected).unwrap();
    let before = trill::read(&expected).unwrap();
    app.edit_music(|music| {
        *music = (*prepared).clone();
        tempo_curve::write(
            music,
            Some(&Curve {
                version: 1,
                points: vec![Point { tick: 0, bpm: 60.0 }],
                controls: vec![],
            }),
        )
    })
    .unwrap();
    let music = &app.loaded.as_ref().unwrap().music;
    let ons: Vec<_> = music.tracks[1]
        .events
        .iter()
        .filter(|e| cat_smf_editor::score::note_view::note_on(e))
        .map(|e| e.tick)
        .collect();
    assert!(ons.len() > 8);
    let map = cat_smf_editor::score::audition_timing::TempoMap::new(music, 1);
    for pair in ons.windows(2) {
        assert!((map.duration(pair[0], pair[1] - pair[0]) - 0.025).abs() < 1e-10);
    }
    assert_eq!(trill::read(music).unwrap(), before);
    let bytes = cat_smf_editor::smf_io::encode(music).unwrap();
    let generation = app.generation;
    assert!(app
        .edit_music(|music| trill::set_interval(music, 1, 1, 1e-12))
        .is_err());
    assert_eq!(app.generation, generation);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        bytes
    );
}

#[test]
fn opening_curve_detects_trills_even_with_existing_references_and_reopening_keeps_speed() {
    let mut song = music();
    song.tracks[1] = notes(
        &(0..8)
            .map(|i| (100 + i * 24, 24, 60 + (i % 2) as u8))
            .collect::<Vec<_>>(),
    );
    reference_tempo::initialize(&mut song).unwrap();
    let mut app = app(song);
    assert!(trill::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .is_empty());
    // A normal piano roll doesn't initialize trills.
    let (ctx, _) = opened_in(&mut app, 1);
    assert!(trill::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .is_empty());
    press(&mut app, &ctx, egui::Key::R);
    app.select_track(0);
    press(&mut app, &ctx, egui::Key::R);
    let prepared = super::super::trills::prepare(&mut app).unwrap();
    let regions = trill::read(&prepared).unwrap();
    assert!(trill::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .is_empty());
    assert!(!app.undo.available());
    assert_eq!(regions.len(), 1);
    assert!((regions[0].metadata.interval_seconds - 0.025).abs() < 1e-12);
    app.edit_music(|music| {
        *music = (*prepared).clone();
        music.tracks[0]
            .events
            .iter_mut()
            .find(|e| e.meta_type == 0x51)
            .unwrap()
            .data = 1_000_000u32.to_be_bytes()[1..].to_vec();
        Ok(())
    })
    .unwrap();
    press(&mut app, &ctx, egui::Key::R);
    press(&mut app, &ctx, egui::Key::R);
    let mut expected = (*prepared).clone();
    cat_smf_editor::edit::regenerate(&mut expected).unwrap();
    assert_eq!(
        trill::read(&app.loaded.as_ref().unwrap().music).unwrap(),
        trill::read(&expected).unwrap()
    );
    let generation = app.generation;
    at(&mut app, &ctx, 20.0, vec![]);
    assert_eq!(app.generation, generation);
}

#[test]
fn both_curve_panels_start_and_end_labels_open_editor_without_editing_curve() {
    for label in ["Trill Start", "Trill End"] {
        for panel in 0..2 {
            let mut app = app(trill_song());
            let (ctx, output) = opened_in(&mut app, 0);
            let positions: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(positions.len(), 2);
            let generation = app.generation;
            super::edit::click(
                &mut app,
                &ctx,
                11.0,
                positions[panel],
                egui::PointerButton::Primary,
            );
            assert!(app.trill_edit.is_some(), "{label}, panel {panel}");
            assert_eq!(app.generation, generation);
            assert!(tempo_curve::read(&app.loaded.as_ref().unwrap().music)
                .unwrap()
                .is_none());
        }
    }
}
