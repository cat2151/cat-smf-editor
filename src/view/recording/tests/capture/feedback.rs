use super::*;
use crate::view::recording::capture;

fn two_phrases(a: &mut App, ctx: &egui::Context) {
    ctx.style_mut(|style| style.animation_time = 0.0);
    open_key(a, ctx);
    let ms = Duration::from_millis;
    tap(a, ctx, Key::Z, ms(500), ms(3000));
    frame(a, ctx, vec![], true);
    tap(a, ctx, Key::C, ms(500), ms(0));
    ctrl_j(a, ctx);
    frame(a, ctx, vec![], true);
}

fn render(a: &mut App, ctx: &egui::Context) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 700.0),
            )),
            ..Default::default()
        },
        |ctx| crate::view::recording::show(a, ctx),
    )
}

fn painted_row(a: &mut App, ctx: &egui::Context, index: usize) -> f32 {
    let output = render(a, ctx);
    let area = row(ctx, index).0;
    let mut green = 0;
    let mut x = None;
    for shape in &output.shapes {
        match &shape.shape {
            egui::Shape::Rect(r)
                if r.fill == egui::Color32::from_rgb(0xa6, 0xe2, 0x2e)
                    && area.contains(r.rect.center()) =>
            {
                green += 1
            }
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                    && area.contains(points[0]) =>
            {
                x = Some(points[0].x)
            }
            _ => {}
        }
    }
    assert!(green > 0, "playing notes must be green");
    x.expect("orange playhead on playing row")
}

#[test]
fn capture_feedback_follows_transport_across_phrases_gaps_and_stop() {
    let mut a = app();
    let ctx = egui::Context::default();
    two_phrases(&mut a, &ctx);
    click_at(&mut a, &ctx, row(&ctx, 1).0.center());
    assert_eq!(capture::position(&a, session(&a)), Some((1, 0)));
    assert_eq!(
        capture::title(&a, session(&a)),
        format!("{} · #1", capture::TITLE)
    );
    let bar = u64::from(a.loaded.as_ref().unwrap().music.division) * 4;
    sounding_at(&mut a, 0.1);
    let (_, tick) = capture::position(&a, session(&a)).unwrap();
    assert!(tick > 0 && tick < bar);
    let before = painted_row(&mut a, &ctx, 1);
    sounding_at(&mut a, 0.2);
    assert!(painted_row(&mut a, &ctx, 1) > before);
    // One empty bar separates the two phrases.
    sounding_at(&mut a, 2.1);
    assert_eq!(capture::position(&a, session(&a)), None);
    assert!(capture::title(&a, session(&a)).ends_with("#1 to #2 wait 2"));
    sounding_at(&mut a, 2.9);
    assert!(capture::title(&a, session(&a)).ends_with("#1 to #2 wait 2"));
    sounding_at(&mut a, 3.1);
    assert!(capture::title(&a, session(&a)).ends_with("#1 to #2 wait 1"));
    render(&mut a, &ctx);
    assert!(capture::native_title(session(&a)).ends_with("#1 to #2 wait 1"));
    sounding_at(&mut a, 4.1);
    crate::play::transport::advance(&mut a);
    assert_eq!(capture::position(&a, session(&a)).unwrap().0, 0);
    assert_eq!(
        capture::title(&a, session(&a)),
        format!("{} · #2", capture::TITLE)
    );
    painted_row(&mut a, &ctx, 0);
    // Listening progresses without changing the browsed phrase or scrolling to it.
    assert_eq!(capture::phrases(&a, session(&a)).unwrap().1, 1);
    crate::play::transport::stop(&mut a);
    assert_eq!(capture::position(&a, session(&a)), None);
    assert_eq!(capture::title(&a, session(&a)), capture::TITLE);
}

#[test]
fn capture_scroll_stays_where_the_user_put_it_and_clear_button_works() {
    let mut a = app();
    let ctx = egui::Context::default();
    two_phrases(&mut a, &ctx);
    a.append_tracks(|music| {
        use cat_smf_editor::score::pattern::recording::capture::{append, Note, Phrase};
        let source = Catalog::read(music)?
            .patterns
            .values()
            .find(|p| p.kind == Kind::Capture)
            .unwrap()
            .track;
        for _ in 0..25 {
            append(
                music,
                Some(source),
                500_000,
                &Phrase {
                    start: 0,
                    container: None,
                    notes: vec![Note {
                        start: 0,
                        length: 100,
                        pitch: 60,
                        velocity: 80,
                    }],
                },
                &[],
            )?;
        }
        Ok(())
    })
    .unwrap();
    for _ in 0..3 {
        render(&mut a, &ctx);
    }
    let id = ctx
        .data(|d| d.get_temp(egui::Id::new("capture-scroll")))
        .unwrap();
    let mut state = egui::scroll_area::State::load(&ctx, id).unwrap();
    state.offset.y = 200.0;
    state.store(&ctx, id);
    for _ in 0..20 {
        render(&mut a, &ctx);
    }
    assert_eq!(
        egui::scroll_area::State::load(&ctx, id).unwrap().offset.y,
        200.0
    );
    let clear: egui::Rect = ctx
        .data(|d| d.get_temp(egui::Id::new("capture-clear")))
        .unwrap();
    click_at(&mut a, &ctx, clear.center());
    assert!(listed(&a).is_empty(), "{:?}", session(&a).error);
}

#[test]
fn capture_clear_preserves_taken_patterns_sounds_and_does_not_return_on_undo() {
    let mut a = app();
    let ctx = egui::Context::default();
    two_phrases(&mut a, &ctx);
    capture_keys(&mut a, &[plain(Key::Enter)]);
    let music = &a.loaded.as_ref().unwrap().music;
    let catalog = Catalog::read(music).unwrap();
    let taken = normal(&catalog)[0].clone();
    let taken_body =
        body::extract(&music.tracks[taken.track].events, taken.range().unwrap()).unwrap();
    let patches = a.loaded.as_ref().unwrap().patches.clone();
    // An undo snapshot predating the clear must not resurrect the old captures.
    a.edit_tracks(|m| edit::tracks::rename(&mut m.tracks[0], "Edited"))
        .unwrap();
    // Include an unflushed phrase too.
    let ms = Duration::from_millis;
    tap(&mut a, &ctx, Key::X, ms(100), ms(0));
    capture_keys(&mut a, &[(Key::Delete, Modifiers::CTRL | Modifiers::SHIFT)]);
    assert!(listed(&a).is_empty(), "{:?}", session(&a).error);
    frame(&mut a, &ctx, vec![], true);
    assert!(listed(&a).is_empty());
    let music = &a.loaded.as_ref().unwrap().music;
    let bytes = |events: Vec<cat_smf_editor::model::Event>| {
        events
            .into_iter()
            .map(|e| (e.tick, e.status, e.meta_type, e.data))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        bytes(body::extract(&music.tracks[taken.track].events, taken.range().unwrap()).unwrap()),
        bytes(taken_body)
    );
    assert_eq!(a.loaded.as_ref().unwrap().patches, patches);
    let encoded = cat_smf_editor::smf_io::encode(music).unwrap();
    let decoded = cat_smf_editor::ffi::parse(&encoded).unwrap();
    assert!(
        cat_smf_editor::score::pattern::recording::capture::captured(&decoded)
            .unwrap()
            .is_empty()
    );
    a.undo_music().unwrap();
    assert!(
        cat_smf_editor::score::pattern::recording::capture::captured(
            &a.loaded.as_ref().unwrap().music
        )
        .unwrap()
        .is_empty()
    );
    tap(&mut a, &ctx, Key::Z, ms(100), ms(3000));
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(listed(&a).len(), 1);
    assert_eq!(listed(&a)[0][0].pitch, 60);
}
