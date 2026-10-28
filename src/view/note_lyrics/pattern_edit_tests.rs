//! During pattern edit, the staff and the Piano roll window label the body's notes with lyrics.
use crate::app::App;
use crate::play::playback::performance;
use crate::view::patterns::library;
use crate::view::piano_roll::{roll, Layout, ROW_HEIGHT};
use crate::view::tests::unplaced::{singing, CONTAINER};

/// The lyrics the staff writes under the container.
fn staff(a: &App) -> Vec<(u64, String)> {
    let loaded = a.loaded.as_ref().unwrap();
    super::read(&loaded.music, CONTAINER, &performance(a)[CONTAINER])
        .into_iter()
        .collect()
}

/// `(tick, pitch)` of a rectangle on `layout`.
fn at(layout: &Layout, rect: &eframe::egui::Rect) -> (u64, u8) {
    let row = (rect.top() / ROW_HEIGHT).round() as u8;
    (
        (rect.left() / layout.scale).round() as u64,
        layout.high - row,
    )
}

/// `(tick, pitch, lyric)` of every labeled note on the Piano roll window's roll of the container.
fn roll_lyrics(a: &App) -> Vec<(u64, u8, String)> {
    let loaded = a.loaded.as_ref().unwrap();
    let layout = roll(loaded, &performance(a), &[CONTAINER], CONTAINER, 0);
    let mut labeled: Vec<_> = layout
        .labels(|r| r, |i| layout.notes[i].1)
        .map(|(rect, text)| {
            let (tick, pitch) = at(&layout, &rect);
            (tick, pitch, text.to_owned())
        })
        .collect();
    labeled.sort();
    labeled
}

#[test]
fn pattern_edit_lyrics_label_the_body_notes() {
    let (mut a, p1) = singing();
    library::edit(&mut a, p1).unwrap();
    assert_eq!(
        (staff(&a), roll_lyrics(&a)),
        (vec![(0, "ど".into())], vec![(0, 60, "ど".into())])
    );
}
