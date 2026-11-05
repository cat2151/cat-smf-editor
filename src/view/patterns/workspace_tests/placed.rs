use super::*;
use cat_smf_editor::model::{Event, Music, Track};
use cat_smf_editor::score::pattern::{Definition, Kind, Meter};

const MEASURE: u64 = 1920;
const COLOR: egui::Color32 = egui::Color32::from_rgb(0xa6, 0xe2, 0x2e);

fn note(events: &mut Vec<Event>, pitch: u8, start: u64, end: u64) {
    for (tick, status, velocity) in [(start, 0x90, 100), (end, 0x80, 0)] {
        events.push(Event {
            tick,
            delta: 0,
            status,
            meta_type: 0,
            data: vec![pitch, velocity],
        });
    }
}

/// Two measures: a low and a high note, then a middle note held across the bar line.
fn two_measures() -> (Music, Definition) {
    let mut events = vec![];
    note(&mut events, 40, 0, 480);
    note(&mut events, 80, 480, 960);
    note(&mut events, 60, 960, 2880);
    events.sort_by_key(|e| e.tick);
    let mut music = cat_smf_editor::edit::tracks::new_song();
    music.tracks.push(Track { events });
    let meter = Meter {
        numerator: 4,
        exponent: 2,
        ticks: MEASURE,
    };
    let definition = Definition {
        id: 1,
        kind: Kind::Normal,
        name: "Pattern 1".into(),
        container: 1,
        track: music.tracks.len() - 1,
        tick: 0,
        meters: vec![meter.clone(), meter],
    };
    (music, definition)
}

fn painted(area: egui::Rect, measure: usize) -> Vec<(egui::Rect, egui::Color32)> {
    let (music, p) = two_measures();
    let c = egui::Context::default();
    c.run(Default::default(), |ctx| {
        let painter = ctx.layer_painter(egui::LayerId::background());
        super::super::cards::paint_container_notes(
            &painter,
            area,
            &music,
            &p,
            Some((measure, measure + 1)),
            COLOR,
        );
    })
    .shapes
    .iter()
    .filter_map(|s| match &s.shape {
        egui::Shape::Rect(r) => Some((r.rect, r.fill)),
        _ => None,
    })
    .collect()
}

#[test]
fn the_lowest_and_highest_notes_reach_the_area_edges() {
    let area = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(90.0, 27.0));
    let notes = painted(area, 0);
    let top = notes.iter().map(|(r, _)| r.top()).fold(f32::MAX, f32::min);
    let bottom = notes.iter().map(|(r, _)| r.bottom()).fold(0.0, f32::max);
    assert_eq!((top, bottom), (area.top(), area.bottom()));
}

#[test]
fn a_held_note_keeps_its_height_and_marks_only_its_note_on() {
    let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(90.0, 27.0));
    let held = COLOR.gamma_multiply(0.45);
    let first = painted(area, 0);
    assert_eq!(
        first.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
        [held, COLOR, held, COLOR, held, COLOR]
    );
    let (onset, _) = first[5];
    assert_eq!(onset.width(), 2.0);
    let second = painted(area, 1);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].1, held);
    assert_eq!(second[0].0.top(), onset.top());
}

#[test]
fn a_placed_pattern_is_named_by_its_short_container_and_serial() {
    let (mut a, c, _, _, _) = verification_song();
    let p = Pane::Song;
    frame(&mut a, &c, p.viewport(), vec![]);
    let rect = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((p.title(), "cell", 0usize, 3u64))))
        .unwrap();
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                "workspace-shapes",
                p.viewport(),
            )))
        })
        .unwrap();
    let texts = shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if rect.contains(t.pos) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, ["Int Sw 2"]);
}

#[test]
fn a_candidate_shows_the_short_name_and_is_found_by_it() {
    let (mut a, c, first, second, _) = verification_song();
    let p = Pane::Song;
    frame(&mut a, &c, p.viewport(), vec![]);
    let part = PatternOrScene::Pattern(second);
    let rect = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((p.title(), "part", format!("{part:?}")))))
        .unwrap();
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                "workspace-shapes",
                p.viewport(),
            )))
        })
        .unwrap();
    assert!(shapes.iter().any(|s| matches!(
        &s.shape,
        egui::Shape::Text(t) if rect.contains(t.pos) && t.galley.text() == "Int Sw 2"
    )));
    let s = a.patterns.workspace.selection_mut(p);
    s.query = "sw 2".into();
    s.choice = Some(PatternOrScene::Pattern(first));
    frame(&mut a, &c, p.viewport(), vec![]);
    assert_eq!(a.patterns.workspace.selection(p).choice, Some(part));
}

#[test]
fn a_loop_stays_loop_and_takes_no_serial() {
    let (mut a, c, _, second, _) = verification_song();
    let p = Pane::Song;
    let track = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns[&second]
        .track;
    let mut id = 0;
    a.edit_tracks(|m| {
        id = cat_smf_editor::score::pattern::operations::create_as(m, track, Kind::Loop)?;
        Ok(())
    })
    .unwrap();
    frame(&mut a, &c, p.viewport(), vec![]);
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                "workspace-shapes",
                p.viewport(),
            )))
        })
        .unwrap();
    let text = |part: PatternOrScene| {
        let rect = c
            .data(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new((p.title(), "part", format!("{part:?}"))))
            })
            .unwrap();
        shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if rect.contains(t.pos) => Some(t.galley.text().to_string()),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(text(PatternOrScene::Pattern(id)), "Loop");
    assert_eq!(text(PatternOrScene::Pattern(second)), "Int Sw 2");
}
