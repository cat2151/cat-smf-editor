//! Keyboard events to recording pads, by physical key position on a Japanese keyboard.
use cat_smf_editor::score::pattern::recording::{Layout, Pad};
use eframe::egui::Key;

/// Both yen/ro keys arrive as `Backslash` without a physical key; only Win32 tells them apart.
const LOWER_BACKSLASH: (u8, Pad) = (0xE2, Pad::Row { row: 0, column: 10 });
const UPPER_BACKSLASH: (u8, Pad) = (0xDC, Pad::Row { row: 3, column: 12 });

const ROWS: [&[Key]; 4] = [
    &[
        Key::Z,
        Key::X,
        Key::C,
        Key::V,
        Key::B,
        Key::N,
        Key::M,
        Key::Comma,
        Key::Period,
        Key::Slash,
    ],
    &[
        Key::A,
        Key::S,
        Key::D,
        Key::F,
        Key::G,
        Key::H,
        Key::J,
        Key::K,
        Key::L,
        Key::Semicolon,
        Key::Quote,
        Key::Backslash,
    ],
    &[
        Key::Q,
        Key::W,
        Key::E,
        Key::R,
        Key::T,
        Key::Y,
        Key::U,
        Key::I,
        Key::O,
        Key::P,
        Key::OpenBracket,
        Key::CloseBracket,
    ],
    &[
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
        Key::Num0,
        Key::Minus,
        Key::Equals,
    ],
];

const LETTERS: [Key; 7] = [Key::C, Key::D, Key::E, Key::F, Key::G, Key::A, Key::B];

/// Key caps of a Japanese keyboard, in `Pad::Row` order.
pub(super) const ROW_LABELS: [&[&str]; 4] = [
    &["z", "x", "c", "v", "b", "n", "m", ",", ".", "/", "\\"],
    &["a", "s", "d", "f", "g", "h", "j", "k", "l", ";", ":", "]"],
    &["q", "w", "e", "r", "t", "y", "u", "i", "o", "p", "@", "["],
    &[
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "^", "\\",
    ],
];
pub(super) const LETTER_LABELS: [&str; 7] = ["c", "d", "e", "f", "g", "a", "b"];

/// `held` tells which pads are down; `down` reads a Win32 virtual key at this moment.
pub(super) fn pad(
    layout: Layout,
    key: Key,
    physical: Option<Key>,
    pressed: bool,
    held: impl Fn(Pad) -> bool,
    down: impl Fn(u8) -> bool,
) -> Option<Pad> {
    match layout {
        Layout::Letters => LETTERS
            .iter()
            .position(|&k| k == key)
            .map(|i| Pad::Letter(i as u8)),
        Layout::Rows => match physical {
            Some(physical) => ROWS.iter().enumerate().find_map(|(row, keys)| {
                keys.iter()
                    .position(|&k| k == physical)
                    .map(|column| Pad::Row {
                        row: row as u8,
                        column: column as u8,
                    })
            }),
            None if key == Key::Backslash => backslash(pressed, held, down),
            None => None,
        },
    }
}

fn backslash(pressed: bool, held: impl Fn(Pad) -> bool, down: impl Fn(u8) -> bool) -> Option<Pad> {
    let sides = [LOWER_BACKSLASH, UPPER_BACKSLASH];
    if pressed {
        return sides
            .into_iter()
            .find(|&(vk, pad)| down(vk) && !held(pad))
            .map(|(_, pad)| pad);
    }
    // With both held, the side Win32 no longer reports down is the one released.
    let holding = sides.into_iter().filter(|&(_, pad)| held(pad));
    holding
        .clone()
        .find(|&(vk, _)| !down(vk))
        .or(holding.clone().next())
        .map(|(_, pad)| pad)
}

#[cfg(windows)]
pub(super) fn key_down(vk: u8) -> bool {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetAsyncKeyState(key: i32) -> i16;
    }
    // SAFETY: GetAsyncKeyState only reads global keyboard state for the given virtual key.
    unsafe { GetAsyncKeyState(i32::from(vk)) < 0 }
}

#[cfg(not(windows))]
pub(super) fn key_down(_: u8) -> bool {
    false
}

#[cfg(test)]
mod tests;
