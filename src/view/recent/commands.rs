use super::*;
use crate::view::commands::{Command, Screen};

pub(crate) fn open(a: &mut App) {
    crate::play::transport::stop(a);
    a.recent.open = true;
    a.recent.cursor = 0;
    a.recent.reveal = true;
    a.recent.error = None;
}
fn choose(a: &mut App, index: usize) {
    let path = a.recent.history.entries.get(index).map(|e| e.path.clone());
    a.recent.open = false;
    silence(a);
    if let Some(path) = path {
        a.open_mid_and_play(path);
    }
}
pub(crate) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if s != Screen::Recent {
        return;
    }
    out.push(Command::plain(
        s,
        "recent-close",
        "取消",
        &[Key::Escape],
        |a, _| {
            a.recent.open = false;
            silence(a);
        },
    ));
    out.push(Command::new(
        s,
        "recent-stop",
        "プレビュー停止",
        &[(Key::Space, Modifiers::SHIFT)],
        |a, _| silence(a),
    ));
    let count = a.recent.history.entries.len();
    if count == 0 {
        return;
    }
    for (id, name, keys, delta) in [
        (
            "recent-next",
            "次のファイル",
            &[Key::J, Key::ArrowDown][..],
            1,
        ),
        (
            "recent-prev",
            "前のファイル",
            &[Key::K, Key::ArrowUp][..],
            -1,
        ),
        ("recent-page-next", "10ファイル下", &[Key::PageDown][..], 10),
        ("recent-page-prev", "10ファイル上", &[Key::PageUp][..], -10),
    ] {
        out.push(
            Command::plain(s, id, name, keys, move |a, c| {
                a.recent.cursor = a.recent.cursor.saturating_add_signed(delta).min(count - 1);
                a.recent.reveal = true;
                preview(a);
                c.request_repaint();
            })
            .repeating(),
        );
    }
    out.push(Command::plain(
        s,
        "recent-choose",
        "開く",
        &[Key::Enter],
        |a, _| choose(a, a.recent.cursor),
    ));
    for (index, entry) in a.recent.history.entries.iter().enumerate() {
        let path = entry.path.clone();
        let label = shortcut(index);
        let keys = label
            .chars()
            .next()
            .and_then(|ch| {
                Key::from_name(&ch.to_ascii_uppercase().to_string()).map(|k| {
                    (
                        k,
                        if ch.is_uppercase() {
                            Modifiers::SHIFT
                        } else if matches!(ch, 'j' | 'k') {
                            Modifiers::ALT
                        } else {
                            Modifiers::NONE
                        },
                    )
                })
            })
            .into_iter()
            .collect::<Vec<_>>();
        out.push(Command::new(
            s,
            format!("recent-{}", entry.path.display()),
            entry.path.display().to_string(),
            &keys,
            move |a, _| {
                if let Some(i) = a.recent.history.entries.iter().position(|e| e.path == path) {
                    choose(a, i);
                }
            },
        ));
    }
}
