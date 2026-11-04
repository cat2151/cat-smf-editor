use super::*;
use cat_smf_editor::score::pattern::recording::{Key, Mode, Pad, ROW_KEYS};

/// Every chord pad of a C major session, down to the right end of each row, shows a name.
#[test]
fn every_chord_pad_is_named() {
    for mode in [
        Mode::Triad,
        Mode::Seventh,
        Mode::TriadInversions,
        Mode::SeventhInversions,
    ] {
        let mut t = Settings::new(Key::from_signature(None), 500_000);
        t.mode = mode;
        for row in 0..4u8 {
            for column in 0..ROW_KEYS[usize::from(row)] {
                let (name, _) = window::caption(&t, Pad::Row { row, column });
                assert!(!name.is_empty(), "{mode:?} row {row} column {column}");
            }
        }
    }
}
