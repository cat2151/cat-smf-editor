use super::*;
use phrase::Looper;

fn press(a: &mut App, ctx: &egui::Context, k: Key, m: Modifiers) {
    frame(a, ctx, vec![key(k, true, m), key(k, false, m)], true);
}
fn pad(a: &mut App, ctx: &egui::Context, k: Key, pressed: bool) {
    frame(a, ctx, vec![key(k, pressed, Modifiers::NONE)], true);
}

/// Notes of pattern `id` as (start, end, pitch) from its start.
fn notes(a: &App, id: u64) -> Vec<(u64, u64, u8)> {
    use cat_smf_editor::score::{
        note_view::{note_off, note_on},
        pattern::{body, Catalog},
    };
    let m = &a.loaded.as_ref().unwrap().music;
    let p = Catalog::read(m).unwrap().patterns[&id].clone();
    let events = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    events
        .iter()
        .filter(|e| note_on(e))
        .map(|e| {
            let off = events
                .iter()
                .find(|o| note_off(o) && o.data[0] == e.data[0] && o.tick > e.tick)
                .unwrap();
            (e.tick - p.tick, off.tick - p.tick, e.data[0])
        })
        .collect()
}

#[test]
fn ctrl_q_quantizes_the_phrase_loop_it_repeats() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(&mut a, &ctx, Key::Q, Modifiers::CTRL);
    assert!(recorder(&a).settings().quantize);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    let sixteenth = Duration::from_micros(u64::from(recorder(&a).settings().tempo) / 4);
    // The clock runs between frames, so each key lies well before its rounding boundary.
    pad(&mut a, &ctx, Key::Z, true);
    rewind(&mut a, sixteenth * 3 / 10);
    pad(&mut a, &ctx, Key::Z, false);
    rewind(&mut a, sixteenth * 6 / 10);
    pad(&mut a, &ctx, Key::X, true);
    rewind(&mut a, sixteenth * 7 / 10);
    pad(&mut a, &ctx, Key::X, false);
    rewind(&mut a, sixteenth * 16);
    frame(&mut a, &ctx, vec![], true);
    let Looper::Looping { pattern, .. } = session(&a).sound().looper else {
        panic!("{:?}", session(&a).sound().looper)
    };
    let grid = u64::from(a.loaded.as_ref().unwrap().music.division) / 4;
    assert_eq!(
        notes(&a, pattern),
        vec![(0, grid, 60), (grid, grid * 2, 62)]
    );
}
