use super::*;
use cat_smf_editor::{
    host::PluginKey,
    model::{Music, Track},
};

fn effect(id: &str) -> PluginState {
    crate::test_support::referenced(PluginState {
        reference: None,
        plugin: PluginKey {
            format: "CLAP".into(),
            id: id.into(),
            name: id.into(),
            vendor: "fixture".into(),
            bundle_path: "X:/plugins/fixture.clap".into(),
        },
        state: vec![1],
        preset_name: Some(id.into()),
        bypass: false,
    })
}

fn names(effects: &[PluginState]) -> Vec<&str> {
    effects.iter().map(|e| e.plugin.id.as_str()).collect()
}

/// Two tracks; track 1 chains delay → reverb → chorus, the master hall → echo.
fn fixture() -> App {
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }; 2],
    });
    let mut patch = app.loaded.as_ref().unwrap().patches[1].clone();
    patch.effects = vec![effect("delay"), effect("reverb"), effect("chorus")];
    crate::player::selection::store_track(&mut app, 1, &patch).unwrap();
    crate::player::selection::store_master(
        &mut app,
        cat_smf_editor::master_effects::Settings {
            effects: vec![effect("hall"), effect("echo")],
            ..Default::default()
        },
    )
    .unwrap();
    app.select_track(1);
    app
}

fn track_effects(app: &App) -> Vec<PluginState> {
    app.loaded.as_ref().unwrap().patches[1].effects.clone()
}

fn saved_track(app: &App) -> Vec<(String, bool)> {
    cat_smf_editor::instrument_layers::read(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap()
    .effects
    .into_iter()
    .map(|e| (e.patch.plugin.id, e.bypass))
    .collect()
}

pub(crate) fn frame(
    app: &mut App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

pub(crate) fn press(
    app: &mut App,
    ctx: &egui::Context,
    key: egui::Key,
    modifiers: egui::Modifiers,
) {
    let event = |pressed| egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    };
    frame(app, ctx, vec![event(true), event(false)]);
}

fn plain(app: &mut App, ctx: &egui::Context, key: egui::Key) {
    press(app, ctx, key, egui::Modifiers::NONE);
}

#[test]
fn x_opens_the_track_chain_on_its_last_effect_and_x_again_browses_it() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    plain(&mut app, &ctx, egui::Key::X);
    assert_eq!(app.effect_editor.target, Some(Target::Track(1)));
    assert_eq!(app.effect_editor.selected, 2);
    assert!(app.player_selection.error.is_none());
    plain(&mut app, &ctx, egui::Key::X);
    // The fixture has no player, so the browser request reports why.
    assert!(app.player_selection.error.is_some());
    assert!(app.effect_editor.is_open());
}

#[test]
fn x_on_a_track_without_effects_browses_at_once() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    app.select_track(0);
    plain(&mut app, &ctx, egui::Key::X);
    assert_eq!(app.effect_editor.target, Some(Target::Track(0)));
    assert!(app.player_selection.error.is_none());
    plain(&mut app, &ctx, egui::Key::X);
    assert!(app.player_selection.error.is_some());
}

#[test]
fn keys_reorder_bypass_remove_and_close_with_saved_results() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    plain(&mut app, &ctx, egui::Key::X);
    plain(&mut app, &ctx, egui::Key::K);
    plain(&mut app, &ctx, egui::Key::K);
    assert_eq!(app.effect_editor.selected, 0);
    press(&mut app, &ctx, egui::Key::J, egui::Modifiers::SHIFT);
    assert_eq!(names(&track_effects(&app)), ["reverb", "delay", "chorus"]);
    assert_eq!(app.effect_editor.selected, 1);
    plain(&mut app, &ctx, egui::Key::B);
    assert_eq!(
        saved_track(&app),
        [
            ("reverb".into(), false),
            ("delay".into(), true),
            ("chorus".into(), false)
        ]
    );
    press(&mut app, &ctx, egui::Key::K, egui::Modifiers::SHIFT);
    assert_eq!(names(&track_effects(&app)), ["delay", "reverb", "chorus"]);
    assert!(track_effects(&app)[0].bypass);
    plain(&mut app, &ctx, egui::Key::Delete);
    assert_eq!(names(&track_effects(&app)), ["reverb", "chorus"]);
    assert_eq!(app.effect_editor.selected, 0);
    plain(&mut app, &ctx, egui::Key::Enter);
    assert!(!app.effect_editor.is_open());
    plain(&mut app, &ctx, egui::Key::X);
    plain(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.effect_editor.is_open());
}

#[test]
fn mixer_opens_the_master_chain_whose_edits_reach_conductor_metadata() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    crate::play::mixer::open(&mut app);
    crate::play::mixer::handle_key(&mut app, egui::Key::X);
    assert_eq!(app.effect_editor.target, Some(Target::Master));
    assert_eq!(app.effect_editor.selected, 1);
    press(&mut app, &ctx, egui::Key::K, egui::Modifiers::SHIFT);
    plain(&mut app, &ctx, egui::Key::B);
    assert_eq!(names(&app.audition.master().effects), ["echo", "hall"]);
    let saved = cat_smf_editor::master_references::read(
        &app.loaded.as_ref().unwrap().music,
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    assert_eq!(
        saved
            .effects
            .iter()
            .map(|e| (e.patch.plugin.id.as_str(), e.bypass))
            .collect::<Vec<_>>(),
        [("echo", true), ("hall", false)]
    );
    plain(&mut app, &ctx, egui::Key::Delete);
    plain(&mut app, &ctx, egui::Key::Delete);
    assert!(app.audition.master().effects.is_empty());
    assert!(!app.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .any(|e| e.status == 0xff && e.meta_type == 0x7f));
}

#[test]
fn mixer_track_cursor_opens_that_tracks_chain() {
    let mut app = fixture();
    app.select_track(0);
    crate::play::mixer::open(&mut app);
    crate::play::mixer::handle_key(&mut app, egui::Key::L);
    crate::play::mixer::handle_key(&mut app, egui::Key::L);
    crate::play::mixer::handle_key(&mut app, egui::Key::Enter);
    assert_eq!(app.effect_editor.target, Some(Target::Track(1)));
    assert_eq!(app.track, 1);
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
        .unwrap_or_else(|| panic!("{text} is not drawn"))
}

#[test]
fn click_selects_and_drag_moves_an_effect_below_another() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    open(&mut app, Target::Track(1), 2);
    // The modal sizes itself invisibly first.
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![]);
    let first = text_pos(&output, "1 delay");
    let second = text_pos(&output, "2 reverb");
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(second), button(second, true)],
    );
    frame(&mut app, &ctx, vec![button(second, false)]);
    assert_eq!(app.effect_editor.selected, 1);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(first), button(first, true)],
    );
    let below = second + egui::vec2(0.0, 10.0);
    for step in 1..=4 {
        let pos = first + (below - first) * (step as f32 / 4.0);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(pos)]);
    }
    frame(&mut app, &ctx, vec![button(below, false)]);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(names(&track_effects(&app)), ["reverb", "delay", "chorus"]);
    assert_eq!(app.effect_editor.selected, 1);
}

#[test]
fn chain_draws_its_source_effects_and_sinks_in_signal_order() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    open(&mut app, Target::Master, 0);
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![]);
    let order =
        ["Tracks", "1 hall", "2 echo", "Auto gain", "Output"].map(|t| text_pos(&output, t).y);
    assert!(order.is_sorted(), "{order:?}");
    open(&mut app, Target::Track(1), 0);
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![]);
    let order = ["1 delay", "3 chorus", "Mixer"].map(|t| text_pos(&output, t).y);
    assert!(order.is_sorted(), "{order:?}");
}
