//! A frame hands the backend only the windows it shows: any other viewport in the output opens
//! a native window that the next frame closes again.
use super::*;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

#[test]
fn the_song_in_a_dock_window_beside_the_help_window_opens_no_other_window() {
    let (mut a, c, ..) = verification_song();
    a.help_open = true;
    a.layout.spec = crate::store::layout::parse(
        r#"
[[window]]
title = "Dock 1"
rect = [1920, 0, 1600, 900]
column = [
  { view = "song", height = 2 },
  { view = "mixer" },
]
"#,
    )
    .unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    harness::unshown(&c);
    for i in 0..10 {
        frame(&mut a, &c, ROOT, vec![]);
        let unshown = harness::unshown(&c);
        assert!(
            unshown.is_empty(),
            "frame {i}: {unshown:?}, Song window {:?}",
            Pane::Song.viewport()
        );
    }
}
