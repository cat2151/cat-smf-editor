use super::*;
use crate::view::palette::tests::{frame, open, press, query};
use cat_smf_editor::edit::tracks;
mod layout;
mod rendering;

fn fixture() -> App {
    let mut music = tracks::new_song();
    tracks::add(&mut music).unwrap();
    music.tracks[0]
        .events
        .retain(|e| !matches!(e.status & 0xf0, 0x80 | 0x90));
    crate::view::tests::app(music)
}
fn browse(a: &mut App, ctx: &egui::Context) {
    open(a, ctx);
    query(a, ctx, "views");
    assert_eq!(crate::view::palette::tests::listed(a).len(), 1);
    press(a, ctx, Key::Enter);
    assert!(a.command_input.views.is_some());
}
fn select(a: &mut App, target: Target) {
    let s = a.command_input.views.as_mut().unwrap();
    s.selected = s.entries.iter().position(|e| e.target == target).unwrap();
}

#[test]
fn browsing_is_read_only_and_enter_opens_the_leaf_without_key_leaks() {
    let mut a = fixture();
    let c = egui::Context::default();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let generation = a.generation;
    browse(&mut a, &c);
    select(&mut a, Target::Level(Level::Track(1)));
    frame(
        &mut a,
        &c,
        vec![
            egui::Event::Key {
                key: Key::L,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
            egui::Event::Text("l".into()),
        ],
    );
    assert_eq!(a.track, 0);
    assert_eq!(
        a.command_input.views.as_ref().unwrap().level,
        Level::Track(1)
    );
    select(&mut a, Target::View(View::Piano(1)));
    press(&mut a, &c, Key::ArrowRight);
    assert!(
        !a.piano_roll.open,
        "horizontal movement never executes a leaf"
    );
    assert!(crate::view::modal::is_open(&a));
    press(&mut a, &c, Key::Enter);
    assert!(a.piano_roll.open);
    assert_eq!(a.track, 1);
    assert!(!crate::view::modal::is_open(&a));
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!(a.generation, generation);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(a.config.piano_roll_maximized);
    press(&mut a, &c, Key::Escape);
    assert!(!a.piano_roll.open);
}

#[test]
fn hjkl_arrows_and_escape_preserve_the_original_track_and_selection() {
    let mut a = fixture();
    let c = egui::Context::default();
    browse(&mut a, &c);
    let first = a.command_input.views.as_ref().unwrap().selected;
    press(&mut a, &c, Key::J);
    assert_eq!(a.command_input.views.as_ref().unwrap().selected, first + 1);
    press(&mut a, &c, Key::ArrowUp);
    assert_eq!(a.command_input.views.as_ref().unwrap().selected, first);
    select(&mut a, Target::Level(Level::Track(1)));
    let selected = a.command_input.views.as_ref().unwrap().selected;
    press(&mut a, &c, Key::L);
    press(&mut a, &c, Key::H);
    assert_eq!(a.command_input.views.as_ref().unwrap().level, Level::Tracks);
    assert_eq!(a.command_input.views.as_ref().unwrap().selected, selected);
    press(&mut a, &c, Key::ArrowRight);
    press(&mut a, &c, Key::Escape);
    assert_eq!(a.command_input.views.as_ref().unwrap().level, Level::Tracks);
    press(&mut a, &c, Key::Escape);
    assert!(!crate::view::modal::is_open(&a));
    assert_eq!(a.track, 0);
    assert!(!a.piano_roll.open && !a.tempo_curve.open);
}

#[test]
fn candidates_require_existing_content_and_refresh_after_edits() {
    let mut a = fixture();
    let targets = |a: &App, level| {
        catalog::entries(a, level)
            .into_iter()
            .map(|e| e.target)
            .collect::<Vec<_>>()
    };
    assert!(!targets(&a, Level::Track(0)).contains(&Target::View(View::Piano(0))));
    assert!(targets(&a, Level::Track(1)).contains(&Target::View(View::Piano(1))));
    assert!(!targets(&a, Level::Track(1)).contains(&Target::View(View::Effects(1))));
    assert!(targets(&a, Level::Layer(1, 1)).is_empty());
    assert!(targets(&a, Level::Tracks).contains(&Target::View(View::Tempo)));
    a.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .retain(|e| !(e.status == 0xff && e.meta_type == 0x51));
    assert!(!targets(&a, Level::Tracks).contains(&Target::View(View::Tempo)));
}

#[test]
fn generation_change_invalidates_the_browsed_targets() {
    let mut a = fixture();
    let c = egui::Context::default();
    browse(&mut a, &c);
    select(&mut a, Target::Level(Level::Track(1)));
    press(&mut a, &c, Key::Enter);
    select(&mut a, Target::View(View::Piano(1)));
    a.generation += 1;
    press(&mut a, &c, Key::Enter);
    assert!(!a.piano_roll.open);
    assert!(a.command_input.views.is_none());
    assert_eq!(a.track, 0);
}

#[test]
fn layer_candidates_follow_saved_parameter_targets_and_ignore_empty_or_changed_targets() {
    use cat_smf_editor::{
        chain_settings::PluginState,
        edit::parameter_curve::{self, Curve, Point, Target as ParameterTarget, ValueSpace},
        host::PluginKey,
    };
    let mut a = fixture();
    let slot = PluginState {
        reference: None,
        plugin: PluginKey {
            format: "CLAP".into(),
            id: "fixture".into(),
            name: "Fixture".into(),
            vendor: "Fixture".into(),
            bundle_path: "X:/plugins/fixture.clap".into(),
        },
        state: vec![],
        preset_name: None,
        bypass: false,
    };
    a.loaded.as_mut().unwrap().patches[1].layers[0].instrument = Some(slot);
    let curve = Curve {
        version: 1,
        target: ParameterTarget {
            group: 0,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "fixture".into(),
            parameter_id: "cutoff".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Cutoff".into(),
        path: "Filter".into(),
        points: vec![Point {
            tick: 0,
            value: 0.5,
        }],
        controls: vec![],
    };
    let music = &mut a.loaded.as_mut().unwrap().music;
    let lane = parameter_curve::create(music, 1, curve).unwrap();
    let entries = catalog::entries(&a, Level::Layer(1, 1));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].target, Target::View(View::Parameter(lane)));
    let patches = a.loaded.as_ref().unwrap().patches.clone();
    a = crate::view::tests::app(a.loaded.as_ref().unwrap().music.clone());
    a.loaded.as_mut().unwrap().patches[..patches.len()].clone_from_slice(&patches);
    let c = egui::Context::default();
    browse(&mut a, &c);
    select(&mut a, Target::Level(Level::Track(1)));
    press(&mut a, &c, Key::ArrowRight);
    select(&mut a, Target::Level(Level::Layer(1, 1)));
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    assert_eq!(
        a.command_input.views.as_ref().unwrap().level,
        Level::Track(1)
    );
    let child = c.memory(|m| m.area_rect(egui::Id::new(("views-list", 2))).unwrap());
    assert!(child.height() > 40.0 && child.height() < 100.0);
    assert!(!a.parameter_editor.open);
    assert!(!crate::play::transport::timeline::running(&a));
    press(&mut a, &c, Key::Escape);
    press(&mut a, &c, Key::Escape);
    let mut curve = parameter_curve::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .curves[&lane]
        .clone();
    curve.points.clear();
    parameter_curve::write(&mut a.loaded.as_mut().unwrap().music, lane, &curve).unwrap();
    assert!(catalog::entries(&a, Level::Layer(1, 1)).is_empty());
    curve.points.push(Point {
        tick: 0,
        value: 0.5,
    });
    parameter_curve::write(&mut a.loaded.as_mut().unwrap().music, lane, &curve).unwrap();
    a.loaded.as_mut().unwrap().patches[1].layers[0].settings.id = 99;
    assert!(catalog::entries(&a, Level::Layer(1, 1)).is_empty());
    assert!(catalog::entries(&a, Level::Layer(1, 99)).is_empty());
}

#[test]
fn help_escape_and_double_score_keep_the_score_context() {
    let mut a = fixture();
    a.note_mode = false;
    let c = egui::Context::default();
    browse(&mut a, &c);
    press(&mut a, &c, Key::F1);
    assert!(a.help_open);
    press(&mut a, &c, Key::F1);
    // Pressing again keeps help open and lifts it above the other windows.
    assert!(a.help_open && a.help.raise);
    assert!(crate::view::modal::is_open(&a));
    select(&mut a, Target::View(View::DoubleScore));
    press(&mut a, &c, Key::Enter);
    assert_eq!(a.track_view.split, Some([0, 1]));
    assert!(a.note_mode);
    assert!(!crate::view::modal::is_open(&a));
    assert_eq!(a.track, 0);
}

#[test]
fn track_overview_is_listed_previewed_and_opened_from_views() {
    let mut a = fixture();
    let c = egui::Context::default();
    browse(&mut a, &c);
    select(&mut a, Target::View(View::Overview));
    frame(&mut a, &c, vec![]);
    assert!(
        !a.track_overview.open,
        "the preview never opens the real overview"
    );
    press(&mut a, &c, Key::Enter);
    assert!(a.track_overview.open);
    assert!(!crate::view::modal::is_open(&a));
    assert_eq!(
        crate::view::commands::owner(&a, &c),
        crate::view::commands::Screen::Overview
    );
}

#[test]
fn public_views_is_read_only_without_palette_search() {
    let mut a = fixture();
    let c = egui::Context::default();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let request = crate::view::commands::Request::new(&a, crate::view::commands::Operation::Views);
    crate::view::commands::submit(&mut a, &c, request).unwrap();
    assert!(!a.palette.is_open());
    frame(&mut a, &c, vec![]);
    select(&mut a, Target::Level(Level::Track(1)));
    press(&mut a, &c, Key::L);
    select(&mut a, Target::View(View::Piano(1)));
    frame(&mut a, &c, vec![]);
    assert_eq!(a.track, 0);
    assert!(!a.piano_roll.open);
    assert!(!a.undo.available());
    press(&mut a, &c, Key::Enter);
    assert_eq!(a.track, 1);
    assert!(a.piano_roll.open);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!crate::play::transport::timeline::running(&a));
}
