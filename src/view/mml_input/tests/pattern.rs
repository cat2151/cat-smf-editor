use super::*;
use cat_smf_editor::score::pattern::{body, operations, Catalog};

fn pattern_fixture() -> (App, u64, u64) {
    let mut music = fixture().loaded.unwrap().music;
    let track = operations::create_source(&mut music).unwrap();
    let id = operations::create(&mut music, track, None).unwrap();
    let neighbor = operations::create(&mut music, track, None).unwrap();
    let p = Catalog::read(&music).unwrap().patterns[&neighbor].clone();
    cat_smf_editor::edit::mml::insert(&mut music, track, p.tick, 0, "e1").unwrap();
    let mut app = crate::view::tests::app(music);
    crate::view::patterns::library::edit(&mut app, id).unwrap();
    (app, id, neighbor)
}

fn definition(app: &App, id: u64) -> cat_smf_editor::score::pattern::Definition {
    Catalog::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns[&id]
        .clone()
}

fn contents(events: &[cat_smf_editor::model::Event]) -> Vec<(u64, u8, u8, Vec<u8>)> {
    events
        .iter()
        .map(|e| (e.tick, e.status, e.meta_type, e.data.clone()))
        .collect()
}

fn placed_pattern_fixture() -> (App, u64, u64) {
    use cat_smf_editor::score::scene::arrangement::{self, PatternOrScene};
    let (mut app, id, neighbor) = pattern_fixture();
    let mut music = app.loaded.as_ref().unwrap().music.clone();
    let lane = arrangement::add_lane(&mut music, None).unwrap();
    arrangement::place(&mut music, None, lane, 1, Some(PatternOrScene::Pattern(id))).unwrap();
    app.apply_music(music).unwrap();
    (app, id, neighbor)
}

#[test]
fn two_measure_mml_previews_confirms_reopens_and_undoes_pattern_growth() {
    let (mut app, id, neighbor) = placed_pattern_fixture();
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let other = definition(&app, neighbor);
    let other_events = body::extract(
        &app.loaded.as_ref().unwrap().music.tracks[other.track].events,
        other.range().unwrap(),
    )
    .unwrap();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    assert!(app.mml_input.open);
    frame(&mut app, &ctx, vec![egui::Event::Text("c1d1".into())]);
    assert_eq!(definition(&app, id).length().unwrap(), 3840);
    assert_eq!(
        cat_smf_editor::score::scene::read(&app.loaded.as_ref().unwrap().music)
            .unwrap()
            .song_length,
        3840
    );
    assert!(
        definition(&app, id).tick > other.tick,
        "growth relocates storage"
    );
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![]);
    assert!(!app.mml_input.open, "{:?}", app.mml_input.error);
    let music = &app.loaded.as_ref().unwrap().music;
    assert_eq!(
        contents(
            &body::extract(&music.tracks[other.track].events, other.range().unwrap()).unwrap()
        ),
        contents(&other_events)
    );
    let reopened = cat_smf_editor::ffi::parse(&smf_io::encode(music).unwrap()).unwrap();
    let p = Catalog::read(&reopened).unwrap().patterns[&id].clone();
    assert_eq!(p.meters.len(), 2);
    assert_eq!(
        cat_smf_editor::score::scene::read(&reopened)
            .unwrap()
            .song_length,
        3840
    );
    assert_eq!(
        body::extract(&reopened.tracks[p.track].events, p.range().unwrap())
            .unwrap()
            .iter()
            .filter(|e| cat_smf_editor::score::note_view::note_on(e))
            .count(),
        2
    );
    app.undo_music().unwrap();
    assert_eq!(definition(&app, id).meters.len(), 1);
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}

#[test]
fn preview_growth_can_shrink_back_and_cancel_without_an_undo_entry() {
    let (mut app, id, _) = placed_pattern_fixture();
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c1 r1".into())]);
    assert_eq!(definition(&app, id).meters.len(), 2); // Trailing rest counts.
    let mut state = std::mem::take(&mut app.mml_input);
    state.buffer = "c4".into();
    preview(&mut app, &mut state);
    assert_eq!(definition(&app, id).meters.len(), 1);
    assert_eq!(
        cat_smf_editor::score::scene::read(&app.loaded.as_ref().unwrap().music)
            .unwrap()
            .song_length,
        1920
    );
    state.buffer = "c1 d1".into();
    preview(&mut app, &mut state);
    app.mml_input = state;
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!app.undo.available());
}

#[test]
fn growth_rounds_up_using_the_last_meter_and_the_insertion_offset() {
    let (mut app, id, _) = pattern_fixture();
    let mut music = app.loaded.as_ref().unwrap().music.clone();
    operations::resize(&mut music, id, body::parse_meters("3/4", 480).unwrap()).unwrap();
    app.apply_music(music).unwrap();
    app.track_view.cursor_tick = Some(960);
    let ctx = egui::Context::default();
    open(&mut app, &ctx, false);
    app.mml_input.buffer = "c1".into();
    let (music, end) = inserted(&app.mml_input).unwrap();
    assert_eq!(end, 2880);
    let p = Catalog::read(&music).unwrap().patterns[&id].clone();
    assert_eq!(p.meters, body::parse_meters("3/4 3/4", 480).unwrap());
}
