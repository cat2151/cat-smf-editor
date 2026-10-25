use super::*;
use crate::view::{commands, tests::app};
use cat_smf_editor::model::{Event, Music, Track};

fn event(tick: u64, status: u8, meta_type: u8, data: Vec<u8>) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type,
        data,
    }
}

/// Conductor, a melody of one note per measure for four measures, and an empty track.
fn fixture() -> App {
    let end = || event(7680, 0xff, 0x2f, vec![]);
    let mut melody = Vec::new();
    for measure in 0..4 {
        melody.push(event(measure * 1920, 0x90, 0, vec![60 + measure as u8, 90]));
        melody.push(event(
            measure * 1920 + 960,
            0x80,
            0,
            vec![60 + measure as u8, 0],
        ));
    }
    melody.push(end());
    app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![event(0, 0xff, 0x51, vec![0x07, 0xa1, 0x20]), end()],
            },
            Track { events: melody },
            Track {
                events: vec![end()],
            },
        ],
    })
}

fn key(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

fn render(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

fn press(app: &mut App, ctx: &egui::Context, keys: &[egui::Key]) {
    for &k in keys {
        render(app, ctx, vec![key(k, true), key(k, false)]);
    }
}

fn click(app: &mut App, ctx: &egui::Context, pos: egui::Pos2, button: egui::PointerButton) {
    render(app, ctx, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        render(
            app,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

fn pane(ctx: &egui::Context, slot: usize) -> egui::Rect {
    ctx.read_response(egui::Id::new(("track-overview-pane", slot)))
        .unwrap()
        .rect
}

fn text_pos(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == text => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing text {text}"))
}

fn playing_tracks(app: &App) -> Vec<usize> {
    app.playback
        .pending
        .as_ref()
        .unwrap()
        .parts
        .iter()
        .map(|p| p.0)
        .collect()
}

fn opened() -> (App, egui::Context) {
    opened_from(fixture())
}

fn opened_from(mut app: App) -> (App, egui::Context) {
    let ctx = egui::Context::default();
    app.select_track(1);
    render(&mut app, &ctx, vec![]);
    let command = commands::candidates(&app, &ctx)
        .into_iter()
        .find(|c| c.id == "track-overview")
        .unwrap();
    assert!(commands::execute(&mut app, &ctx, &command));
    render(&mut app, &ctx, vec![]);
    (app, ctx)
}

#[test]
fn palette_opens_the_overview_and_escape_closes_it() {
    let (mut app, ctx) = opened();
    assert!(app.track_overview.open);
    assert_eq!(commands::owner(&app, &ctx), Screen::Overview);
    assert_eq!(app.track_overview.cell, (1, 0));
    press(&mut app, &ctx, &[egui::Key::Escape]);
    assert!(!app.track_overview.open);
    assert_eq!(commands::owner(&app, &ctx), Screen::Main);
}

#[test]
fn the_go_menu_has_no_overview_entry() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::Escape, egui::Key::G]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Go);
    assert!(!commands::definitions(&app, Screen::Go)
        .iter()
        .any(|c| c.name == "Track Overview"));
    press(&mut app, &ctx, &[egui::Key::M]);
    assert!(!app.track_overview.open);
}

#[test]
fn keys_move_the_cell_by_measure_and_track_within_the_song() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::H, egui::Key::K, egui::Key::K]);
    assert_eq!(
        app.track_overview.cell,
        (0, 0),
        "stops at the first measure and track"
    );
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::J]);
    assert_eq!(app.track_overview.cell, (1, 1920));
    press(&mut app, &ctx, &[egui::Key::L; 6]);
    assert_eq!(
        app.track_overview.cell,
        (1, 7680),
        "stops at the last event's measure"
    );
    press(&mut app, &ctx, &[egui::Key::J, egui::Key::J]);
    assert_eq!(app.track_overview.cell, (2, 7680));
}

#[test]
fn enter_plays_every_track_and_t_plays_the_cell_track_from_its_measure() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::Enter]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 2.0).abs() < 1e-9);
    assert!(
        (prepared.active.end - 6.0).abs() < 1e-9,
        "plays to the song end"
    );
    assert!(playing_tracks(&app).contains(&1));
    assert!(app.track_overview.open);
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::T]);
    assert_eq!(playing_tracks(&app), vec![1]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 4.0).abs() < 1e-9);
}

#[test]
fn g_closes_the_overview_and_puts_the_cursor_on_the_cell() {
    let (mut app, ctx) = opened();
    press(
        &mut app,
        &ctx,
        &[egui::Key::L, egui::Key::L, egui::Key::K, egui::Key::G],
    );
    assert!(!app.track_overview.open);
    assert_eq!(app.track, 0);
    assert_eq!(app.track_view.cursor_tick, Some(3840));
    assert_eq!(commands::owner(&app, &ctx), Screen::Main);
}

#[test]
fn play_menu_and_space_play_from_the_cell_and_keep_the_overview_open() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::P]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Play);
    press(&mut app, &ctx, &[egui::Key::S]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 2.0).abs() < 1e-9);
    assert_eq!(playing_tracks(&app), vec![1]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Overview);
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::Space]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!(
        (prepared.active.start_seconds - 4.0).abs() < 1e-9,
        "repeats the last mode at the moved cell"
    );
    assert!(app.track_overview.open);
}

#[test]
fn f_toggles_follow_and_ctrl_comma_opens_settings() {
    let (mut app, ctx) = opened();
    let follow = app.playback.follow;
    press(&mut app, &ctx, &[egui::Key::F]);
    assert_eq!(app.playback.follow, !follow);
    let ctrl_comma = egui::Event::Key {
        key: egui::Key::Comma,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::CTRL,
    };
    render(&mut app, &ctx, vec![ctrl_comma]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Settings);
}

#[test]
fn left_click_plays_all_tracks_from_the_clicked_measure() {
    let (mut app, ctx) = opened();
    let rect = pane(&ctx, 1);
    // 0.2 px per tick: x 400 is tick 2000 inside the second measure.
    click(
        &mut app,
        &ctx,
        egui::pos2(rect.left() + 400.0, rect.center().y),
        egui::PointerButton::Primary,
    );
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 2.0).abs() < 1e-9);
    assert_eq!(app.track_overview.cell, (1, 1920));
}

#[test]
fn right_click_menu_offers_track_play_and_jump() {
    let (mut app, ctx) = opened();
    let rect = pane(&ctx, 1);
    let at = egui::pos2(rect.left() + 800.0, rect.center().y);
    click(&mut app, &ctx, at, egui::PointerButton::Secondary);
    let output = render(&mut app, &ctx, vec![]);
    let button = text_pos(&output, "t — track play");
    click(&mut app, &ctx, button, egui::PointerButton::Primary);
    assert_eq!(playing_tracks(&app), vec![1]);
    assert!(
        (app.playback.pending.as_ref().unwrap().active.start_seconds - 4.0).abs() < 1e-9,
        "x 800 is the third measure"
    );
    click(&mut app, &ctx, at, egui::PointerButton::Secondary);
    let output = render(&mut app, &ctx, vec![]);
    let button = text_pos(&output, "g — jump");
    click(&mut app, &ctx, button, egui::PointerButton::Primary);
    assert!(!app.track_overview.open);
    assert_eq!(app.track, 1);
    assert_eq!(app.track_view.cursor_tick, Some(3840));
}

#[test]
fn panes_tell_notes_events_and_empty_tracks_apart() {
    let app = fixture();
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(
        (0..3).map(|t| kind(loaded, t)).collect::<Vec<_>>(),
        vec![Kind::Events, Kind::Notes, Kind::Empty]
    );
}

#[test]
fn reveal_scrolls_only_as_far_as_the_cell_needs() {
    let shown = egui::vec2(100.0, 50.0);
    let cell = |x: f32, y: f32| egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(40.0, 20.0));
    assert_eq!(
        window::reveal_offset(egui::vec2(0.0, 0.0), shown, cell(10.0, 10.0)),
        egui::vec2(0.0, 0.0)
    );
    assert_eq!(
        window::reveal_offset(egui::vec2(0.0, 0.0), shown, cell(200.0, 100.0)),
        egui::vec2(140.0, 70.0)
    );
    assert_eq!(
        window::reveal_offset(egui::vec2(300.0, 300.0), shown, cell(200.0, 100.0)),
        egui::vec2(200.0, 100.0)
    );
}

/// `color` as painted while the opening window still fades in.
fn faded(painted: egui::Color32, color: egui::Color32) -> bool {
    let opacity = f32::from(painted.a()) / 255.0;
    painted.a() > 0
        && (0..3).all(|i| (f32::from(painted[i]) - f32::from(color[i]) * opacity).abs() <= 2.0)
}

/// Lines in `color` that lie within the pane's height and start inside it.
fn lines_in(output: &egui::FullOutput, pane: egui::Rect, color: egui::Color32) -> usize {
    output
        .shapes
        .iter()
        .filter(|s| match &s.shape {
            egui::epaint::Shape::Path(path) => {
                matches!(path.stroke.color, egui::epaint::ColorMode::Solid(c) if faded(c, color))
                    && path.points.len() > 1
                    && pane.contains(path.points[0])
                    && path.points.iter().all(|p| pane.y_range().contains(p.y))
            }
            _ => false,
        })
        .count()
}

#[test]
fn conductor_shows_tempo_and_tracks_with_curves_show_them() {
    use cat_smf_editor::edit::parameter_curve::{self, Curve, Point, Target, ValueSpace};
    use cat_smf_editor::score::{
        macro_curve,
        macro_values::{self, Values, Variations},
    };
    let mut music = fixture().loaded.unwrap().music;
    let lane = |id: &str, value| Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: id.into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Cutoff".into(),
        path: "Filter".into(),
        points: vec![
            Point { tick: 0, value },
            Point {
                tick: 3840,
                value: 1.0 - value,
            },
        ],
        controls: vec![Point {
            tick: 1920,
            value: 0.5,
        }],
    };
    let control = parameter_curve::create(&mut music, 1, lane("cutoff", 0.2)).unwrap();
    let snapshot = |value| {
        let mut c = lane("macro", value);
        c.points.truncate(1);
        c.controls.clear();
        vec![c]
    };
    macro_values::write(
        &mut music.tracks[1],
        &Values {
            version: 1,
            parameters: snapshot(0.2),
            variations: vec![Variations {
                layer: 1,
                selected: 0,
                snapshots: vec![snapshot(0.2), snapshot(0.8)],
            }],
        },
    )
    .unwrap();
    let mut variation = lane("macro-variation", 0.0);
    variation.target.plugin_id = "CLAP:org.surge-synth-team.surge-xt".into();
    variation.points[1].value = 1.0;
    macro_curve::write(&mut music.tracks[1], &variation).unwrap();
    let (mut app, ctx) = opened_from(crate::view::tests::app(music));
    let output = render(&mut app, &ctx, vec![]);
    let tempo = crate::view::tempo_curve::TEMPO;
    let curve = crate::view::tempo_curve::CURVE;
    assert_eq!(lines_in(&output, pane(&ctx, 0), tempo), 1);
    assert_eq!(lines_in(&output, pane(&ctx, 1), curve), 1);
    assert_eq!(lines_in(&output, pane(&ctx, 2), curve), 0);
    assert_eq!(lines_in(&output, pane(&ctx, control), curve), 1);
}

#[test]
fn panes_draw_notes_in_the_track_color_with_measure_lines_only() {
    let (mut app, ctx) = opened();
    let output = render(&mut app, &ctx, vec![]);
    let pane = pane(&ctx, 1);
    let color = crate::view::track_color(1);
    let mut notes: Vec<_> = output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if pane.contains(r.rect.min)
                    && r.rect.height() < pane.height() / 2.0
                    && faded(r.fill, color) =>
            {
                Some(r)
            }
            _ => None,
        })
        .collect();
    assert_eq!(notes.len(), 4);
    notes.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
    let measure = notes[1].rect.left() - notes[0].rect.left();
    let lines: Vec<_> = output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::LineSegment { points, .. } if pane.contains(points[0]) => {
                Some(*points)
            }
            _ => None,
        })
        .collect();
    let edge = |y: f32| (y - pane.top()).abs() < 3.0 || (pane.bottom() - y).abs() < 3.0;
    assert!(
        lines.iter().all(|p| p[0].x == p[1].x || edge(p[0].y)),
        "no staff lines"
    );
    // Inside the first four measures, past the lane's left frame.
    let first = notes[0].rect.left();
    let within = lines
        .iter()
        .filter(|p| p[0].x == p[1].x)
        .filter(|p| (first + 1.0..first + 4.0 * measure - 1.0).contains(&p[0].x))
        .count();
    assert_eq!(within, 3, "one line per measure, none per beat");
}

/// Shapes far longer than the view lose precision and flicker while scrolling.
#[test]
fn pane_shapes_stay_within_the_visible_area() {
    let (mut app, ctx) = opened();
    let output = render(&mut app, &ctx, vec![]);
    let pane = pane(&ctx, 1);
    let view = pane.expand(2.0);
    for s in output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
    {
        let bounds = s.shape.visual_bounding_rect();
        if !bounds.is_finite() {
            continue;
        }
        assert!(
            view.contains_rect(bounds),
            "{bounds:?} reaches past {pane:?}"
        );
    }
}

#[test]
fn drum_notes_fade_after_their_first_thirty_second() {
    let mut app = fixture();
    app.loaded.as_mut().unwrap().patches[1].drums = Some(Default::default());
    let (mut app, ctx) = opened_from(app);
    let output = render(&mut app, &ctx, vec![]);
    let pane = pane(&ctx, 1);
    let color = crate::view::track_color(1);
    // Head and tail share the hue; the tail is fainter.
    let mut notes: Vec<_> = output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if r.rect.height() < pane.height() / 2.0 && faded(r.fill, color) =>
            {
                Some((r.rect, r.fill.a()))
            }
            _ => None,
        })
        .collect();
    notes.sort_by(|a, b| a.0.left().total_cmp(&b.0.left()));
    let head = notes.iter().map(|n| n.1).max().unwrap();
    let (heads, tails): (Vec<&(egui::Rect, u8)>, Vec<_>) = notes.iter().partition(|n| n.1 == head);
    let heads: Vec<_> = heads.iter().map(|n| n.0).collect();
    let tails: Vec<_> = tails.iter().map(|n| n.0).collect();
    assert_eq!((heads.len(), tails.len()), (4, 4));
    for (head, tail) in heads.iter().zip(&tails) {
        assert_eq!(head.right(), tail.left());
        // A 32nd of the 960-tick note.
        let ratio = head.width() / (head.width() + tail.width());
        assert!((ratio - 0.0625).abs() < 0.01, "{ratio}");
    }
}
