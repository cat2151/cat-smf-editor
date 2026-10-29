use super::*;
use crate::{
    cursor::{moves, split},
    play::playback,
};

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if a.loaded.is_none() {
        return;
    }
    if matches!(s, Screen::Main | Screen::Timeline | Screen::Tracks) {
        out.push(Command::plain(
            s,
            "rename-track",
            "トラック名変更",
            &[Key::F2],
            |a, _| {
                super::super::track_rename::begin(a, a.track);
            },
        ));
    }
    if matches!(s, Screen::Main | Screen::Timeline | Screen::Drums) {
        for (id, name, key, forward) in [
            ("track-next", "次のtrack", Key::ArrowRight, true),
            ("track-prev", "前のtrack", Key::ArrowLeft, false),
        ] {
            out.push(Command::new(
                s,
                id,
                name,
                &[(key, Modifiers::CTRL)],
                move |a, _| {
                    split::step_track(a, forward, playback::location(a).0);
                    if s == Screen::Timeline {
                        a.audition.stop_key();
                        a.piano_roll.open = false;
                        a.tempo_curve.open = false;
                        a.parameter_editor.close();
                        super::super::piano_roll::open(a);
                    }
                },
            ));
        }
    }
    match s {
        Screen::Main | Screen::Selection => {
            if split::row_count(a, a.track) > 0 {
                for (id, name, keys, step) in [
                    ("row-down", "次の行", &[Key::J, Key::ArrowDown][..], 1),
                    ("row-up", "前の行", &[Key::K, Key::ArrowUp][..], -1),
                    ("page-down", "10行下", &[Key::PageDown][..], 10),
                    ("page-up", "10行上", &[Key::PageUp][..], -10),
                    ("first-row", "先頭の行", &[Key::Home][..], isize::MIN),
                    ("last-row", "末尾の行", &[Key::End][..], isize::MAX),
                ] {
                    out.push(
                        Command::plain(s, id, name, keys, move |a, _| moves::move_rows(a, step))
                            .repeating(),
                    );
                }
                for (id, name, keys, forward, measure) in [
                    (
                        "measure-next",
                        "次の小節",
                        vec![
                            (Key::L, Modifiers::NONE),
                            (Key::ArrowRight, Modifiers::NONE),
                            (Key::ArrowDown, Modifiers::CTRL),
                        ],
                        true,
                        true,
                    ),
                    (
                        "measure-prev",
                        "前の小節",
                        vec![
                            (Key::H, Modifiers::NONE),
                            (Key::ArrowLeft, Modifiers::NONE),
                            (Key::ArrowUp, Modifiers::CTRL),
                        ],
                        false,
                        true,
                    ),
                    (
                        "beat-next",
                        "次の拍",
                        vec![(Key::J, Modifiers::SHIFT)],
                        true,
                        false,
                    ),
                    (
                        "beat-prev",
                        "前の拍",
                        vec![(Key::K, Modifiers::SHIFT)],
                        false,
                        false,
                    ),
                ] {
                    out.push(
                        Command::new(s, id, name, &keys, move |a, _| {
                            moves::boundary(a, forward, measure)
                        })
                        .repeating(),
                    );
                }
            }
        }
        Screen::Go => {
            out.push(Command::plain(
                s,
                "go-close",
                "閉じる",
                &[Key::Escape],
                |a, _| a.track_view.go_open = false,
            ));
            out.push(Command::plain(
                s,
                "go-tracks",
                "Track list",
                &[Key::T],
                |a, _| a.track_view.track_list = true,
            ));
            out.push(Command::plain(
                s,
                "go-start",
                "Track start",
                &[Key::H],
                |a, _| split::go_edge(a, false),
            ));
            out.push(Command::plain(
                s,
                "go-end",
                "Track end",
                &[Key::E],
                |a, _| split::go_edge(a, true),
            ));
        }
        Screen::Tracks => {
            out.push(Command::plain(
                s,
                "tracks-close",
                "閉じる",
                &[Key::Escape],
                |a, _| a.track_view.go_open = false,
            ));
            for (id, name, key, forward) in [
                ("go-next", "Next track", Key::N, true),
                ("go-prev", "Previous track", Key::P, false),
            ] {
                out.push(Command::plain(s, id, name, &[key], move |a, _| {
                    split::step_track(a, forward, a.track_view.origin_tick)
                }));
            }
            for (i, title) in a.loaded.as_ref().unwrap().track_titles.iter().enumerate() {
                let keys = split::key_label(i)
                    .and_then(|c| {
                        Key::from_name(&c.to_ascii_uppercase().to_string()).map(|k| {
                            (
                                k,
                                if c.is_uppercase() {
                                    Modifiers::SHIFT
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
                    format!("track-{i}"),
                    title,
                    &keys,
                    move |a, _| split::go(a, i, a.track_view.origin_tick),
                ));
            }
        }
        Screen::Split => {
            for (id, name, key, run) in [
                (
                    "split-close",
                    "閉じる",
                    Key::Escape,
                    (|a: &mut App, _: &egui::Context| a.track_view.split_open = false)
                        as fn(&mut App, &egui::Context),
                ),
                ("split-off", "1列", Key::W, |a, _| split::split_off(a)),
                ("split-left", "左の列", Key::H, |a, _| split::focus(a, 0)),
                ("split-right", "右の列", Key::L, |a, _| {
                    split::focus(a, 1)
                }),
                ("split-next", "次のtrack", Key::J, |a, _| {
                    split::step_column(a, true)
                }),
                ("split-prev", "前のtrack", Key::K, |a, _| {
                    split::step_column(a, false)
                }),
            ] {
                out.push(Command::plain(s, id, name, &[key], run));
            }
        }
        _ => {}
    }
}
