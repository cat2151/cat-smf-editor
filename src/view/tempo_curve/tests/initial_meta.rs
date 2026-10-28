//! The first tempo curve edit also writes SMF's implicit 4/4 and C major.
use super::*;
use cat_smf_editor::edit::initial_meta::{C_MAJOR, FOUR_FOUR};

/// (track, tick, meta type, data) of every Time Signature and Key Signature.
fn signatures(app: &crate::app::App) -> Vec<(usize, u64, u8, Vec<u8>)> {
    app.loaded
        .as_ref()
        .unwrap()
        .music
        .tracks
        .iter()
        .enumerate()
        .flat_map(|(t, track)| track.events.iter().map(move |e| (t, e)))
        .filter(|(_, e)| e.status == 0xff && matches!(e.meta_type, 0x58 | 0x59))
        .map(|(t, e)| (t, e.tick, e.meta_type, e.data.clone()))
        .collect()
}

fn songs(app: &crate::app::App) -> String {
    format!("{:?}", app.loaded.as_ref().unwrap().music.tracks)
}

/// Adds one curve point a quarter right of the first tempo, as a user's first edit.
fn edit_once(app: &mut crate::app::App, ctx: &egui::Context, output: &egui::FullOutput) {
    let first = circles(output, TEMPO)[0].0;
    super::edit::click(
        app,
        ctx,
        11.0,
        first + egui::vec2(96.0, 0.0),
        egui::PointerButton::Primary,
    );
    assert!(
        app.tempo_curve.error.is_none(),
        "{:?}",
        app.tempo_curve.error
    );
}

fn ctrl_z(app: &mut crate::app::App, ctx: &egui::Context) {
    render(
        app,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            })
            .collect(),
    );
}

#[test]
fn opening_writes_nothing_and_the_first_edit_writes_both_and_one_undo_removes_them() {
    let mut app = app(music());
    let original = songs(&app);
    let generation = app.generation;
    let (ctx, output) = opened_in(&mut app, 0);
    at(&mut app, &ctx, 10.5, vec![]);
    assert_eq!(app.generation, generation);
    assert_eq!(songs(&app), original);

    edit_once(&mut app, &ctx, &output);
    assert_eq!(app.generation, generation + 1);
    assert!(tempo_curve::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .is_some());
    assert_eq!(
        signatures(&app),
        [
            (0, 0, 0x58, FOUR_FOUR.to_vec()),
            (0, 0, 0x59, C_MAJOR.to_vec())
        ]
    );
    ctrl_z(&mut app, &ctx);
    assert_eq!(songs(&app), original);
    assert!(signatures(&app).is_empty());
}

#[test]
fn a_song_with_both_gets_no_more_signatures_from_an_edit() {
    let mut song = music();
    for (meta_type, data) in [(0x58, &[3u8, 2, 24, 8][..]), (0x59, &[2, 0][..])] {
        cat_smf_editor::edit::insert(&mut song.tracks[1], event(0, 0xff, meta_type, data));
    }
    cat_smf_editor::edit::refresh_deltas(&mut song.tracks[1]).unwrap();
    let mut app = app(song);
    let before = signatures(&app);
    let generation = app.generation;
    let (ctx, output) = opened_in(&mut app, 0);
    edit_once(&mut app, &ctx, &output);
    assert_eq!(app.generation, generation + 1);
    assert_eq!(signatures(&app), before);
}
