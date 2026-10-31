use super::*;
use eframe::egui::{self, Modifiers};
use std::collections::HashSet;

/// `key` / `physical_key` measured on JIS (kbd106) through eframe 0.31.1, row by row.
fn measured() -> Vec<(&'static str, Key, Option<Key>)> {
    let same = |name, k| (name, k, Some(k));
    vec![
        same("z", Key::Z),
        same("x", Key::X),
        same("c", Key::C),
        same("v", Key::V),
        same("b", Key::B),
        same("n", Key::N),
        same("m", Key::M),
        same(",", Key::Comma),
        same(".", Key::Period),
        same("/", Key::Slash),
        ("\\ lower", Key::Backslash, None),
        same("a", Key::A),
        same("s", Key::S),
        same("d", Key::D),
        same("f", Key::F),
        same("g", Key::G),
        same("h", Key::H),
        same("j", Key::J),
        same("k", Key::K),
        same("l", Key::L),
        same(";", Key::Semicolon),
        (":", Key::Colon, Some(Key::Quote)),
        ("]", Key::CloseBracket, Some(Key::Backslash)),
        same("q", Key::Q),
        same("w", Key::W),
        same("e", Key::E),
        same("r", Key::R),
        same("t", Key::T),
        same("y", Key::Y),
        same("u", Key::U),
        same("i", Key::I),
        same("o", Key::O),
        same("p", Key::P),
        ("@", Key::OpenBracket, Some(Key::OpenBracket)),
        ("[", Key::OpenBracket, Some(Key::CloseBracket)),
        same("1", Key::Num1),
        same("2", Key::Num2),
        same("3", Key::Num3),
        same("4", Key::Num4),
        same("5", Key::Num5),
        same("6", Key::Num6),
        same("7", Key::Num7),
        same("8", Key::Num8),
        same("9", Key::Num9),
        same("0", Key::Num0),
        same("-", Key::Minus),
        ("^", Key::Equals, Some(Key::Equals)),
        ("\\ upper", Key::Backslash, None),
    ]
}

fn event(key: Key, physical_key: Option<Key>) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn convert(e: &egui::Event, layout: Layout, held: &HashSet<Pad>, down: &[u8]) -> Option<Pad> {
    let egui::Event::Key {
        key,
        physical_key,
        pressed,
        ..
    } = *e
    else {
        unreachable!()
    };
    pad(
        layout,
        key,
        physical_key,
        pressed,
        |p| held.contains(&p),
        |vk| down.contains(&vk),
    )
}

#[test]
fn all_48_measured_keys_are_distinct_pads_in_row_order() {
    let mut expected = vec![];
    for (row, &n) in cat_smf_editor::score::pattern::recording::ROW_KEYS
        .iter()
        .enumerate()
    {
        for column in 0..n {
            expected.push(Pad::Row {
                row: row as u8,
                column,
            });
        }
    }
    let got = measured()
        .into_iter()
        .map(|(name, key, physical)| {
            let down: &[u8] = match name {
                "\\ lower" => &[0xE2],
                "\\ upper" => &[0xDC],
                _ => &[],
            };
            convert(&event(key, physical), Layout::Rows, &HashSet::new(), down)
                .unwrap_or_else(|| panic!("{name} has no pad"))
        })
        .collect::<Vec<_>>();
    assert_eq!(got, expected);
    assert_eq!(got.iter().collect::<HashSet<_>>().len(), 48);
}

#[test]
fn backslash_side_comes_from_win32_and_is_ignored_when_neither_is_down() {
    let none = HashSet::new();
    let press = event(Key::Backslash, None);
    assert_eq!(convert(&press, Layout::Rows, &none, &[]), None);
    assert_eq!(
        convert(&press, Layout::Rows, &none, &[0xE2]),
        Some(LOWER_BACKSLASH.1)
    );
    assert_eq!(
        convert(&press, Layout::Rows, &none, &[0xDC]),
        Some(UPPER_BACKSLASH.1)
    );
    // Lower already sounding: a second press while both are down is the upper key.
    let lower = HashSet::from([LOWER_BACKSLASH.1]);
    assert_eq!(
        convert(&press, Layout::Rows, &lower, &[0xE2, 0xDC]),
        Some(UPPER_BACKSLASH.1)
    );
    assert_eq!(convert(&press, Layout::Rows, &lower, &[0xE2]), None);
}

#[test]
fn backslash_release_picks_the_held_side_or_the_one_win32_reports_up() {
    let release = egui::Event::Key {
        key: Key::Backslash,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    let rows = Layout::Rows;
    assert_eq!(convert(&release, rows, &HashSet::new(), &[]), None);
    let upper = HashSet::from([UPPER_BACKSLASH.1]);
    assert_eq!(
        convert(&release, rows, &upper, &[]),
        Some(UPPER_BACKSLASH.1)
    );
    let both = HashSet::from([LOWER_BACKSLASH.1, UPPER_BACKSLASH.1]);
    assert_eq!(
        convert(&release, rows, &both, &[0xE2]),
        Some(UPPER_BACKSLASH.1)
    );
    assert_eq!(
        convert(&release, rows, &both, &[0xDC]),
        Some(LOWER_BACKSLASH.1)
    );
}

#[test]
fn letters_layout_maps_only_c_to_b() {
    let none = HashSet::new();
    let letters = [Key::C, Key::D, Key::E, Key::F, Key::G, Key::A, Key::B]
        .map(|k| convert(&event(k, Some(k)), Layout::Letters, &none, &[]));
    assert_eq!(letters, std::array::from_fn(|i| Some(Pad::Letter(i as u8))));
    assert_eq!(
        convert(&event(Key::Z, Some(Key::Z)), Layout::Letters, &none, &[]),
        None
    );
    assert_eq!(
        convert(&event(Key::H, Some(Key::H)), Layout::Rows, &none, &[]),
        Some(Pad::Row { row: 1, column: 5 })
    );
}

#[test]
fn win32_key_state_links_and_reads() {
    // Only the call is checked; the result depends on the physical keyboard.
    let _ = [key_down(LOWER_BACKSLASH.0), key_down(UPPER_BACKSLASH.0)];
}
