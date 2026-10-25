use super::*;
use crate::view::commands::{self, Screen};

pub(super) fn fixture() -> App {
    let mut app = crate::view::tests::app(cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track { events: vec![] }; 2],
    });
    let mut patch = Patch {
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
    crate::player::selection::store_track(&mut app, 1, &patch).unwrap();
    app.track = 1;
    open(&mut app);
    app
}
fn press(app: &mut App, ctx: &egui::Context, key: Key) {
    let _ = ctx.run(
        egui::RawInput {
            events: [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect(),
            ..Default::default()
        },
        |c| commands::route(app, c),
    );
}
#[test]
fn keyboard_grid_page_numeric_cancel_confirm_undo_and_close() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    assert_eq!(commands::owner(&app, &ctx), Screen::Drums);
    press(&mut app, &ctx, Key::Enter);
    assert!(drums::active(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        480,
        0,
        0
    ));
    press(&mut app, &ctx, Key::PageDown);
    press(&mut app, &ctx, Key::ArrowUp);
    press(&mut app, &ctx, Key::Enter);
    assert!(drums::active(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        480,
        1,
        16
    ));
    press(&mut app, &ctx, Key::ArrowLeft);
    press(&mut app, &ctx, Key::Enter);
    assert_eq!(commands::owner(&app, &ctx), Screen::DrumInput);
    app.drum_editor.input = Some(Input::Note {
        text: "73".into(),
        focus: false,
    });
    press(&mut app, &ctx, Key::Escape);
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[1]
            .output,
        60
    );
    press(&mut app, &ctx, Key::Enter);
    app.drum_editor.input = Some(Input::Note {
        text: "73".into(),
        focus: false,
    });
    press(&mut app, &ctx, Key::Enter);
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[1]
            .output,
        73
    );
    app.undo_music().unwrap();
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[1]
            .output,
        60
    );
    press(&mut app, &ctx, Key::Escape);
    assert!(!app.drum_editor.open);
    assert!(!app.piano_roll.open);
}

#[test]
fn drums_input_blocks_background_commands_and_stale_palette_targets() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let old = commands::definitions(&app, Screen::Drums)
        .into_iter()
        .find(|c| c.id == "drum-activate")
        .unwrap();
    press(&mut app, &ctx, Key::ArrowUp);
    assert!(!commands::execute(&mut app, &ctx, &old));
    app.drum_editor.field = 1;
    press(&mut app, &ctx, Key::Enter);
    let page = app.drum_editor.page;
    press(&mut app, &ctx, Key::PageDown);
    press(&mut app, &ctx, Key::J);
    assert_eq!(app.drum_editor.page, page);
    assert_eq!(app.drum_editor.row, 1);
    assert!(!app.playback.open);
    assert!(commands::definitions(&app, Screen::DrumInput)
        .iter()
        .all(|c| !matches!(c.id.as_str(), "drum-play" | "undo")));
}

#[test]
fn nonempty_track_prompts_immediately_and_escape_preserves_the_song() {
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let ctx = egui::Context::default();
    open(&mut app);
    assert_eq!(commands::owner(&app, &ctx), Screen::DrumCreate);
    assert!(app.drum_editor.error.is_none());
    let defs = commands::definitions(&app, Screen::DrumCreate);
    assert!(defs
        .iter()
        .any(|c| c.id == "drum-create" && c.keys.contains(&(Key::Enter, egui::Modifiers::NONE))));
    assert!(!defs
        .iter()
        .any(|c| matches!(c.id.as_str(), "drum-activate" | "drum-play" | "undo")));
    press(&mut app, &ctx, Key::J);
    press(&mut app, &ctx, Key::PageDown);
    assert_eq!(app.drum_editor.row, 0);
    assert_eq!(app.drum_editor.page, 0);
    press(&mut app, &ctx, Key::Escape);
    assert_eq!(commands::owner(&app, &ctx), Screen::Main);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!app.undo.available());
}

#[test]
fn confirmed_creation_adds_one_track_and_undo_preserves_the_original() {
    let mut app = fixture();
    let patch = app.loaded.as_ref().unwrap().patches[1].clone();
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    create_track_with(&mut app, |_| Ok((patch, Default::default()))).unwrap();
    assert_eq!(app.track, 2);
    assert!(app.drum_editor.open);
    assert!(!app.drum_editor.confirm_create);
    assert!(app.loaded.as_ref().unwrap().patches[2].drums.is_some());
    app.undo_music().unwrap();
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}

#[test]
fn confirmation_draws_simple_english_choices() {
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let ctx = egui::Context::default();
    open(&mut app);
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |c| show(&mut app, c));
    let output = ctx.run(input(), |c| show(&mut app, c));
    for label in ["Create a new drum track?", "Yes · Enter", "No · Esc"] {
        assert!(
            output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label)),
            "Missing {label}"
        );
    }
}

#[test]
fn fast_toggle_visual_navigation_and_scoped_silent_loops_survive_edits() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    assert_eq!(app.playback.repeat, Some((0, 1920)));
    assert_eq!(
        app.playback
            .pending
            .as_ref()
            .unwrap()
            .parts
            .iter()
            .map(|p| p.0)
            .collect::<Vec<_>>(),
        vec![1]
    );
    press(&mut app, &ctx, Key::Space);
    assert!(playback::measure(&app).unwrap().active(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        0,
        0
    ));
    press(&mut app, &ctx, Key::F);
    assert!(!playback::measure(&app).unwrap().active(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        0,
        0
    ));
    press(&mut app, &ctx, Key::K);
    assert_eq!(app.drum_editor.row, 1);
    press(&mut app, &ctx, Key::S);
    assert_eq!(app.playback.pending.as_ref().unwrap().parts.len(), 2);
    press(&mut app, &ctx, Key::PageDown);
    assert_eq!(app.playback.repeat, Some((1920, 3840)));
    assert_eq!(
        app.playback.pending.as_ref().unwrap().active.start_seconds,
        2.0
    );
    assert_eq!(app.playback.pending.as_ref().unwrap().active.end, 2.0);
    super::commands::apply(&mut app, super::commands::Op::Boundary(true, 1));
    assert_eq!(app.playback.repeat, Some((1920, 5760)));
    press(&mut app, &ctx, Key::F);
    assert_eq!(app.playback.repeat, Some((1920, 5760)));
    crate::play::transport::advance(&mut app);
    let end = app.playback.active.as_ref().unwrap().end;
    app.audition.set_test_seconds(end + 0.01);
    crate::play::transport::advance(&mut app);
    assert_eq!(
        app.playback.pending.as_ref().unwrap().active.start_seconds,
        2.0
    );
    press(&mut app, &ctx, Key::Escape);
    assert!(app.playback.pending.is_none());
    assert!(app.playback.repeat.is_none());
}

#[test]
fn grid_renders_percussion_above_hat_snare_and_kick() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1500.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |c| show(&mut app, c));
    let output = ctx.run(input(), |c| show(&mut app, c));
    let y = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos.y),
                _ => None,
            })
            .unwrap()
    };
    assert!(y("Percussion") < y("Hi-Hat"));
    assert!(y("Hi-Hat") < y("Snare"));
    assert!(y("Snare") < y("Kick"));
    assert!(output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="meas. 1")));
}

#[test]
fn twelve_step_measures_bound_keyboard_and_wheel_changes_repeat_end() {
    let mut app = fixture();
    app.edit_music(|m| {
        cat_smf_editor::edit::insert(
            &mut m.tracks[0],
            cat_smf_editor::model::Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x58,
                data: vec![6, 3, 24, 8],
            },
        );
        Ok(())
    })
    .unwrap();
    open(&mut app);
    assert_eq!(playback::measure(&app).unwrap().steps.len(), 12);
    assert_eq!(app.playback.repeat, Some((0, 1440)));
    let ctx = egui::Context::default();
    for _ in 0..20 {
        press(&mut app, &ctx, Key::L);
    }
    assert_eq!(app.drum_editor.field, 13);
    press(&mut app, &ctx, Key::Space);
    assert!(playback::measure(&app).unwrap().active(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        0,
        11
    ));
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1500.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |c| show(&mut app, c));
    let output = ctx.run(input(), |c| show(&mut app, c));
    let pos = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "meas. 1" => {
                Some(t.pos + t.galley.size() * 0.5)
            }
            _ => None,
        })
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, 1.0),
                    modifiers: Default::default(),
                },
            ],
            ..input()
        },
        |c| show(&mut app, c),
    );
    assert_eq!(app.drum_editor.last, 2);
    assert_eq!(app.playback.repeat, Some((0, 2880)));
}
