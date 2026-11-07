use super::*;
use cat_smf_editor::score::step_notes;

/// A new song with an ordinary (neither Drum nor Drum kit) pattern container being edited,
/// every pitch shown as a row.
pub(super) fn ordinary_pattern() -> App {
    let mut a = ordinary_pattern_in(cat_smf_editor::edit::tracks::new_song());
    set_rows(&mut a, cat_smf_editor::score::harmony::Set::Chromatic);
    a
}
/// The row set of the edited container's step grid.
pub(super) fn set_rows(a: &mut App, set: cat_smf_editor::score::harmony::Set) {
    let container = a.patterns.container.unwrap();
    a.config.melody_row_sets.insert(container, set);
}
pub(super) fn ordinary_pattern_in(music: cat_smf_editor::model::Music) -> App {
    let mut a = crate::view::tests::app(music);
    crate::view::patterns::library::add_container_with(&mut a, None).unwrap();
    assert!(
        crate::view::patterns::mode::editing(&a, a.track).is_some(),
        "{:?}",
        a.load_error
    );
    assert!(!a.loaded.as_ref().unwrap().patches[a.track].sequencer());
    a
}
fn sequencer(a: &App) -> bool {
    a.loaded.as_ref().unwrap().patches[a.track].sequencer()
}
/// One frame as the app runs it: the grid routes its own keys while it is open.
pub(super) fn frame(
    a: &mut App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            if is_open(a) {
                show(a, ctx);
            } else {
                crate::view::commands::route(a, ctx);
            }
        },
    )
}
fn key(a: &mut App, ctx: &egui::Context, key: Key, modifiers: egui::Modifiers) {
    let events = [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        })
        .collect();
    frame(a, ctx, events);
}
pub(super) fn plain(a: &mut App, ctx: &egui::Context, k: Key) {
    key(a, ctx, k, egui::Modifiers::NONE);
}
pub(super) fn shift(a: &mut App, ctx: &egui::Context, k: Key) {
    key(a, ctx, k, egui::Modifiers::SHIFT);
}
/// (pitch, Note On tick, Note Off tick, velocity) of the edited pattern body.
fn notes(a: &App) -> Vec<(u8, u64, u64, u8)> {
    step_notes::notes(&a.loaded.as_ref().unwrap().visible().tracks[a.track])
        .into_iter()
        .map(|n| (n.pitch, n.start, n.end, n.velocity))
        .collect()
}
/// An ordinary pattern with its step grid open.
pub(super) fn fixture() -> App {
    let mut a = ordinary_pattern();
    open(&mut a);
    assert!(is_open(&a) && melody::active(&a), "{:?}", a.load_error);
    a
}
fn opened() -> (App, egui::Context) {
    let mut a = ordinary_pattern();
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift(&mut a, &ctx, Key::D);
    assert_eq!(a.load_error, None);
    assert!(is_open(&a) && melody::active(&a));
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}

#[test]
fn shift_d_on_an_ordinary_pattern_opens_the_step_grid_without_changing_the_sound() {
    // Empty, with notes, and not yet being edited: every way in opens the melody grid.
    for case in 0..3 {
        let mut a = ordinary_pattern();
        let track = a.track;
        if case == 1 {
            a.edit_music(|m| cat_smf_editor::edit::mml::insert(m, track, 0, 0, "c").map(|_| ()))
                .unwrap();
        }
        if case == 2 {
            a.patterns.editing = None;
        }
        let patch = a.loaded.as_ref().unwrap().patches[track].clone();
        let ctx = egui::Context::default();
        frame(&mut a, &ctx, vec![]);
        shift(&mut a, &ctx, Key::D);
        assert_eq!(a.load_error, None, "case {case}");
        assert!(!a.drum_editor.confirm_create, "case {case}");
        assert!(is_open(&a) && melody::active(&a), "case {case}");
        assert!(!sequencer(&a));
        assert_eq!(a.loaded.as_ref().unwrap().patches[track], patch);
        let mut output = frame(&mut a, &ctx, vec![]);
        for _ in 0..2 {
            output = frame(&mut a, &ctx, vec![]);
        }
        let title = format!(
            "Step sequencer · {}",
            a.loaded.as_ref().unwrap().track_titles[track]
        );
        assert!(
            output.shapes.iter().any(|s| matches!(&s.shape,
                egui::epaint::Shape::Text(t) if t.galley.text() == title)),
            "case {case}"
        );
        assert_eq!(
            crate::view::commands::owner(&a, &ctx),
            crate::view::commands::Screen::Drums
        );
    }
}

#[test]
fn a_drum_container_still_opens_the_drum_grid() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let mut patch = Patch {
        kit: None,
        drums: Some(drums::Settings::default()),
        layers: vec![],
        effects: vec![],
    };
    for id in 1..=4 {
        patch.layers.push(ResolvedLayer {
            settings: cat_smf_editor::instrument_layers::Layer {
                id,
                ..Default::default()
            },
            instrument: None,
        });
    }
    crate::view::patterns::library::add_container_with(&mut a, Some(patch)).unwrap();
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift(&mut a, &ctx, Key::D);
    assert!(is_open(&a) && sequencer(&a) && !melody::active(&a));
    let mut commands = Vec::new();
    super::commands::collect(&a, crate::view::commands::Screen::Drums, &mut commands);
    assert!(commands.iter().any(|c| c.id == "drum.row-next"));
    assert!(!commands.iter().any(|c| c.id.starts_with("melody.")));
}

#[test]
fn keyboard_only_open_move_place_extend_shorten_page_undo_and_close() {
    let (mut a, ctx) = opened();
    // An empty pattern starts on C4, step 1.
    assert_eq!((melody::pitch(&a), melody::step(&a)), (60, 0));
    plain(&mut a, &ctx, Key::L);
    plain(&mut a, &ctx, Key::K);
    plain(&mut a, &ctx, Key::Space);
    // 480 ticks per quarter: a 16th step is 120 ticks.
    assert_eq!(notes(&a), [(61, 120, 240, 100)]);
    assert_eq!(a.audition.auditioned, [61]);
    shift(&mut a, &ctx, Key::L);
    shift(&mut a, &ctx, Key::L);
    assert_eq!(notes(&a), [(61, 120, 480, 100)]);
    shift(&mut a, &ctx, Key::H);
    assert_eq!(notes(&a), [(61, 120, 360, 100)]);
    // Shortening never goes below one step.
    shift(&mut a, &ctx, Key::H);
    shift(&mut a, &ctx, Key::H);
    assert_eq!(notes(&a), [(61, 120, 240, 100)]);
    // f on the following step places a separate note right after it.
    plain(&mut a, &ctx, Key::ArrowRight);
    plain(&mut a, &ctx, Key::F);
    assert_eq!(notes(&a), [(61, 120, 240, 100), (61, 240, 360, 100)]);
    // Extending the first note stops at the head of the second.
    plain(&mut a, &ctx, Key::H);
    shift(&mut a, &ctx, Key::L);
    assert_eq!(notes(&a), [(61, 120, 240, 100), (61, 240, 360, 100)]);
    // Undo keeps the grid open on the same pitch and step.
    key(&mut a, &ctx, Key::Z, egui::Modifiers::CTRL);
    assert_eq!(notes(&a), [(61, 120, 240, 100)]);
    assert!(is_open(&a) && melody::active(&a));
    assert_eq!((melody::pitch(&a), melody::step(&a)), (61, 1));
    // Removing covers the whole note.
    plain(&mut a, &ctx, Key::Space);
    assert_eq!(notes(&a), []);
    // Octaves and rows.
    shift(&mut a, &ctx, Key::K);
    assert_eq!(melody::pitch(&a), 73);
    shift(&mut a, &ctx, Key::J);
    shift(&mut a, &ctx, Key::J);
    plain(&mut a, &ctx, Key::J);
    plain(&mut a, &ctx, Key::ArrowDown);
    assert_eq!(melody::pitch(&a), 47);
    // The pattern is one measure: PageDown stays, and the end of the pattern stops a note.
    plain(&mut a, &ctx, Key::PageDown);
    assert_eq!(a.drum_editor.page, 0);
    for _ in 0..20 {
        plain(&mut a, &ctx, Key::L);
    }
    assert_eq!(melody::step(&a), 15);
    plain(&mut a, &ctx, Key::Space);
    shift(&mut a, &ctx, Key::L);
    assert_eq!(notes(&a), [(47, 1800, 1920, 100)]);
    plain(&mut a, &ctx, Key::Escape);
    assert!(!is_open(&a));
}

#[test]
fn notes_continue_into_the_next_measure_of_a_longer_pattern() {
    let (mut a, ctx) = opened();
    let track = a.track;
    // Lengthen the pattern body to two measures with a note in the second.
    let id = crate::view::patterns::mode::editing(&a, track)
        .unwrap()
        .pattern;
    a.edit_tracks(|m| {
        use cat_smf_editor::score::pattern;
        let meter = pattern::Meter {
            numerator: 4,
            exponent: 2,
            ticks: 1920,
        };
        pattern::operations::resize(m, id, vec![meter; 2])
    })
    .unwrap();
    frame(&mut a, &ctx, vec![]);
    assert!(is_open(&a), "{:?}", a.load_error);
    for _ in 0..15 {
        plain(&mut a, &ctx, Key::L);
    }
    plain(&mut a, &ctx, Key::F);
    shift(&mut a, &ctx, Key::L);
    assert_eq!(notes(&a), [(60, 1800, 2040, 100)]);
    plain(&mut a, &ctx, Key::PageDown);
    assert_eq!(a.drum_editor.page, 1);
    for _ in 0..15 {
        plain(&mut a, &ctx, Key::H);
    }
    // The continuation in measure 2 is the same note: Shift+H shortens it.
    shift(&mut a, &ctx, Key::H);
    assert_eq!(notes(&a), [(60, 1800, 1920, 100)]);
}

#[test]
fn a_note_is_one_band_with_a_head_line_and_neighbours_stay_apart() {
    let (mut a, ctx) = opened();
    plain(&mut a, &ctx, Key::F);
    shift(&mut a, &ctx, Key::L);
    plain(&mut a, &ctx, Key::L);
    plain(&mut a, &ctx, Key::L);
    plain(&mut a, &ctx, Key::F);
    let mut output = frame(&mut a, &ctx, vec![]);
    for _ in 0..2 {
        output = frame(&mut a, &ctx, vec![]);
    }
    let rect = |step: usize| {
        ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("melody-cell", 60u8, step))))
            .unwrap()
    };
    // The continuation reaches back to the head's cell: no gap inside one note.
    assert_eq!(rect(1).left(), rect(0).right());
    let heads: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::LineSegment { points, stroke }
                if stroke.width == 2.0 && stroke.color == egui::Color32::from_gray(16) =>
            {
                Some(points[0].x)
            }
            _ => None,
        })
        .collect();
    // Two notes, two head lines: at step 1 and at step 3.
    assert_eq!(heads, [rect(0).left(), rect(2).left()]);
    let clip = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new("melody-clip"))
            .unwrap()
    });
    assert!(
        rect(0).intersects(clip),
        "C4 {:?} outside {clip:?}",
        rect(0)
    );
}

#[test]
fn opening_scrolls_to_the_middle_of_the_pattern_notes() {
    let mut a = ordinary_pattern();
    let track = a.track;
    a.edit_music(|m| cat_smf_editor::edit::mml::insert(m, track, 0, 0, "o2c o7c").map(|_| ()))
        .unwrap();
    let pitches: Vec<_> = notes(&a).iter().map(|n| n.0).collect();
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift(&mut a, &ctx, Key::D);
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![]);
    }
    let (low, high) = (pitches[0].min(pitches[1]), pitches[0].max(pitches[1]));
    assert_eq!(melody::pitch(&a), low + (high - low) / 2);
    let cell = ctx
        .data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new(("melody-cell", melody::pitch(&a), 0usize)))
        })
        .unwrap();
    let clip = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new("melody-clip"))
            .unwrap()
    });
    assert!(cell.intersects(clip), "{cell:?} outside {clip:?}");
}
#[test]
fn opening_and_every_row_set_center_the_span_of_the_notes() {
    let mut a = ordinary_pattern();
    set_rows(&mut a, cat_smf_editor::score::harmony::Set::Chord);
    let track = a.track;
    a.edit_music(|m| cat_smf_editor::edit::mml::insert(m, track, 0, 0, "o4c o5c").map(|_| ()))
        .unwrap();
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift(&mut a, &ctx, Key::D);
    let check = |a: &mut App, ctx: &egui::Context| {
        // Cells are recorded only while drawn; drop the previous frames' ones.
        let placed: Vec<(u8, usize)> = notes(a)
            .iter()
            .enumerate()
            .map(|(i, n)| (n.0, i * 4))
            .collect();
        for &(p, s) in &placed {
            ctx.data_mut(|d| d.remove::<egui::Rect>(egui::Id::new(("melody-cell", p, s))));
        }
        for _ in 0..3 {
            frame(a, ctx, vec![]);
        }
        let clip = ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new("melody-clip"))
                .unwrap()
        });
        let cells: Vec<egui::Rect> = placed
            .iter()
            .map(|&(p, s)| {
                ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("melody-cell", p, s))))
                    .unwrap_or_else(|| {
                        panic!(
                            "{:?}: pitch {p} not drawn",
                            a.config.melody_rows(a.patterns.container)
                        )
                    })
            })
            .collect();
        let span = cells[0].union(cells[1]);
        assert!(
            clip.contains_rect(span),
            "{:?}: {span:?} in {clip:?}",
            a.config.melody_rows(a.patterns.container)
        );
        assert!(
            (span.center().y - clip.center().y).abs() < 18.0,
            "{:?}: {span:?} not centered in {clip:?}",
            a.config.melody_rows(a.patterns.container)
        );
    };
    check(&mut a, &ctx);
    for _ in 0..4 {
        plain(&mut a, &ctx, Key::Slash);
        check(&mut a, &ctx);
    }
}
#[test]
fn a_line_under_each_tonic_row_marks_the_octaves() {
    let mut a = ordinary_pattern();
    set_rows(&mut a, cat_smf_editor::score::harmony::Set::Penta);
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift(&mut a, &ctx, Key::D);
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![]);
    }
    let line = |p: u8| ctx.data(|d| d.get_temp::<f32>(egui::Id::new(("melody-octave", p))));
    let cell = |p: u8| {
        ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("melody-cell", p, 0usize))))
            .unwrap()
    };
    // A new song is in C major: C4 is the floor of its octave, A3 the row below it.
    let y = line(60).expect("C4 line");
    assert!(
        cell(60).bottom() <= y && y <= cell(57).top(),
        "{y} between C4 and A3"
    );
    assert_eq!(line(62), None);
}
