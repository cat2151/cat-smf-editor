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
        out.push(Command::plain(s, "track.rename", &[Key::F2], |a, _| {
            super::super::track_rename::begin(a, a.track);
        }));
    }
    if matches!(s, Screen::Main | Screen::Timeline | Screen::Drums) {
        for (id, key, forward) in [
            ("track.next", Key::ArrowRight, true),
            ("track.prev", Key::ArrowLeft, false),
        ] {
            out.push(Command::new(
                s,
                id,
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
                for (id, keys, step) in [
                    ("cursor.row-down", &[Key::J, Key::ArrowDown][..], 1),
                    ("cursor.row-up", &[Key::K, Key::ArrowUp][..], -1),
                    ("cursor.page-down", &[Key::PageDown][..], 10),
                    ("cursor.page-up", &[Key::PageUp][..], -10),
                    ("cursor.first-row", &[Key::Home][..], isize::MIN),
                    ("cursor.last-row", &[Key::End][..], isize::MAX),
                ] {
                    out.push(
                        Command::plain(s, id, keys, move |a, _| moves::move_rows(a, step))
                            .repeating(),
                    );
                }
                for (id, keys, forward, measure) in [
                    (
                        "cursor.measure-next",
                        vec![
                            (Key::L, Modifiers::NONE),
                            (Key::ArrowRight, Modifiers::NONE),
                            (Key::ArrowDown, Modifiers::CTRL),
                        ],
                        true,
                        true,
                    ),
                    (
                        "cursor.measure-prev",
                        vec![
                            (Key::H, Modifiers::NONE),
                            (Key::ArrowLeft, Modifiers::NONE),
                            (Key::ArrowUp, Modifiers::CTRL),
                        ],
                        false,
                        true,
                    ),
                    (
                        "cursor.beat-next",
                        vec![(Key::J, Modifiers::SHIFT)],
                        true,
                        false,
                    ),
                    (
                        "cursor.beat-prev",
                        vec![(Key::K, Modifiers::SHIFT)],
                        false,
                        false,
                    ),
                ] {
                    out.push(
                        Command::new(s, id, &keys, move |a, _| {
                            moves::boundary(a, forward, measure)
                        })
                        .repeating(),
                    );
                }
            }
        }
        Screen::Go => {
            out.push(Command::plain(s, "go.close", &[Key::Escape], |a, _| {
                a.track_view.go_open = false
            }));
            out.push(Command::plain(s, "go.tracks", &[Key::T], |a, _| {
                a.track_view.track_list = true
            }));
            out.push(Command::plain(s, "go.start", &[Key::H], |a, _| {
                split::go_edge(a, false)
            }));
            out.push(Command::plain(s, "go.end", &[Key::E], |a, _| {
                split::go_edge(a, true)
            }));
        }
        Screen::Tracks => {
            out.push(Command::plain(s, "tracks.close", &[Key::Escape], |a, _| {
                a.track_view.go_open = false
            }));
            for (id, key, forward) in [("go.next", Key::N, true), ("go.prev", Key::P, false)] {
                out.push(Command::plain(s, id, &[key], move |a, _| {
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
                out.push(
                    Command::new(s, format!("track.entry-{i}"), &keys, move |a, _| {
                        split::go(a, i, a.track_view.origin_tick)
                    })
                    .named(title)
                    .text_id("track.entry"),
                );
            }
        }
        Screen::Split => {
            for (id, key, run) in [
                (
                    "split.close",
                    Key::Escape,
                    (|a: &mut App, _: &egui::Context| a.track_view.split_open = false)
                        as fn(&mut App, &egui::Context),
                ),
                ("split.off", Key::W, |a, _| split::split_off(a)),
                ("split.left", Key::H, |a, _| split::focus(a, 0)),
                ("split.right", Key::L, |a, _| split::focus(a, 1)),
                ("split.next", Key::J, |a, _| split::step_column(a, true)),
                ("split.prev", Key::K, |a, _| split::step_column(a, false)),
            ] {
                out.push(Command::plain(s, id, &[key], run));
            }
        }
        _ => {}
    }
}
