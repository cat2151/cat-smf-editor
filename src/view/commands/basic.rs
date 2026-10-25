use super::*;
use crate::{
    play::{mixer, playback, transport},
    view::{busy, files},
};

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if matches!(s, Screen::Main) {
        out.push(Command::plain(s, "quit", "終了", &[], |a, c| {
            a.request_close(c);
        }));
        out.push(Command::new(
            s,
            "open",
            "開く",
            &[(Key::O, Modifiers::CTRL)],
            |a, _| a.file_open_requested = true,
        ));
        out.push(Command::new(
            s,
            "new",
            "新規SMF",
            &[(Key::N, Modifiers::CTRL)],
            |a, c| a.new_smf(c),
        ));
        if a.loaded.is_some() {
            out.push(Command::plain(
                s,
                "insert-event",
                "Insert event…",
                &[],
                |_, _| {},
            ));
            out.push(
                Command::new(s, "save", "保存", &[(Key::S, Modifiers::CTRL)], |a, _| {
                    a.allow_departure(files::Action::Save);
                })
                .repeating(),
            );
            out.push(Command::new(
                s,
                "add-track",
                "トラック追加",
                &[(Key::N, Modifiers::CTRL | Modifiers::SHIFT)],
                |a, c| busy::request(a, c, busy::Task::AddTrack),
            ));
            out.push(Command::new(
                s,
                "layers",
                "レイヤー",
                &[(Key::T, Modifiers::SHIFT)],
                |a, _| super::super::layers::open(a),
            ));
            out.push(Command::new(
                s,
                "drums",
                "Drum sequencer",
                &[(Key::D, Modifiers::SHIFT)],
                |a, _| super::super::drums::open(a),
            ));
            out.push(Command::plain(
                s,
                "add-drum-track",
                "ドラムトラック追加",
                &[],
                |a, _| super::super::drums::add_track(a),
            ));
        }
        out.push(
            Command::new(
                s,
                "recent",
                "最近開いたファイル",
                &[(Key::P, Modifiers::CTRL)],
                |a, _| super::super::recent::commands::open(a),
            )
            .repeating(),
        );
    }
    if matches!(s, Screen::Main | Screen::Timeline | Screen::Overview) {
        out.push(
            Command::new(
                s,
                "settings",
                "設定menu",
                &[(Key::Comma, Modifiers::CTRL)],
                |a, _| super::super::comma_menu::open(a),
            )
            .repeating(),
        );
    }
    if a.loaded.is_some()
        && matches!(
            s,
            Screen::Main | Screen::Timeline | Screen::Mixer | Screen::Layers | Screen::Effects
        )
    {
        out.push(Command::plain(s, "browse-views", "Views…", &[], |_, _| {}));
    }
    let mut add = |id, name, keys: &[Key], run: fn(&mut App, &egui::Context)| {
        let mut command = Command::plain(s, id, name, keys, run);
        command.repeat = matches!(s, Screen::Confirm | Screen::Invalid)
            || matches!(
                id,
                "settings" | "edit" | "repeat-edit" | "timeline" | "follow"
            );
        out.push(command);
    };
    match s {
        Screen::Browser if a.player_selection.no_notes_error => add(
            "browser-error-close",
            "閉じる",
            &[Key::Enter, Key::Escape],
            |a, _| {
                a.player_selection.no_notes_error = false;
                a.player_selection.open = false;
                a.player_selection.error = None;
            },
        ),
        Screen::Invalid => add(
            "invalid-close",
            "閉じる",
            &[Key::Enter, Key::Escape],
            |a, _| {
                a.invalid_smf_name = None;
                a.load_error = None;
            },
        ),
        Screen::Missing => {
            add("missing-new", "新規SMF", &[Key::N], |a, _| {
                files::create_new_song(a)
            });
            add("missing-quit", "終了", &[Key::Q], |a, c| {
                a.request_close(c)
            });
        }
        Screen::Confirm => {
            add("save-confirm", "保存", &[Key::Y], |a, c| {
                a.finish_departure(true, c)
            });
            add("save-cancel", "取消", &[Key::Escape], |a, _| {
                a.config.navigation.departure = None;
                a.config.navigation.departure_error = None;
            });
            add("save-discard", "破棄 / 続行", &[Key::N], |a, c| {
                if matches!(
                    a.config.navigation.departure,
                    Some(files::Action::Save | files::Action::EnableAutoSave)
                ) {
                    a.config.navigation.departure = None;
                    a.config.navigation.departure_error = None;
                } else {
                    a.finish_departure(false, c);
                }
            });
            if a.config.navigation.backup_tip_visible && a.saved.directory.is_some() {
                add("backup", "Backup directory", &[Key::O], files::open_backup);
            }
        }
        Screen::Backup => {
            add(
                "backup-close",
                "閉じる",
                &[Key::Enter, Key::Escape],
                |a, _| files::close_backup_notice(a),
            );
            add("backup", "Backup directory", &[Key::O], files::open_backup);
        }
        Screen::Help => {
            add("help", "ヘルプ", &[Key::F1], |a, _| a.help_open = false);
            add("help-close", "閉じる", &[Key::Escape], |a, _| {
                a.help_open = false
            });
        }
        Screen::Analysis => add(
            "analysis-close",
            "閉じる",
            &[Key::F, Key::Escape],
            |a, _| {
                if a.analysis.is_open() {
                    super::super::analysis::toggle(a);
                }
            },
        ),
        Screen::Note => {
            add(
                "note-confirm",
                "確定",
                &[Key::Enter],
                super::super::note_edit::confirm,
            );
            add("note-cancel", "取消", &[Key::Escape], |a, _| {
                a.note_edit = None
            });
        }
        Screen::Trill => add(
            "trill-cancel",
            "取消",
            &[Key::Escape],
            super::super::trill_edit::cancel,
        ),
        Screen::Main | Screen::Timeline | Screen::Mixer => {
            if s != Screen::Mixer {
                add("help", "ヘルプ", &[Key::F1], |a, _| {
                    a.help_open = !a.help_open
                });
            }
            if a.loaded.is_none() {
                return;
            }
            if s == Screen::Timeline && !a.parameter_editor.open {
                add("mml-open", "MML / コード進行", &[Key::I], |a, c| {
                    busy::request(a, c, busy::Task::MmlInput)
                });
            }
            if s == Screen::Main {
                add("view-mode", "Notes / Raw", &[Key::V], |a, _| {
                    a.note_mode = !a.note_mode;
                    crate::cursor::moves::sync_mode(a);
                    a.reset_scroll = true;
                });
                add("edit", "編集menu", &[Key::E], |a, _| {
                    a.edit_menu.open = true
                });
                if a.last_command.is_some() {
                    add("repeat-edit", "編集反復", &[Key::Period], |a, _| {
                        super::super::edit_menu::repeat(a)
                    });
                }
                add("timeline", "Piano roll / Tempo curve", &[Key::R], |a, _| {
                    super::super::piano_roll::open(a)
                });
                add("go", "移動menu", &[Key::G], |a, _| {
                    a.track_view.origin_tick = playback::location(a).0;
                    a.track_view.go_open = true;
                    a.track_view.track_list = false;
                });
                add("columns", "2列menu", &[Key::W], |a, _| {
                    a.track_view.split_open = true
                });
                add("mml-open", "MML / コード進行", &[Key::I], |a, c| {
                    busy::request(a, c, busy::Task::MmlInput)
                });
                add(
                    "mml-measure",
                    "MML / コード進行: 1小節を上書き",
                    &[],
                    |a, c| {
                        use crate::view::mml_input::measure;
                        measure::begin(a, c)
                    },
                );
                add("patch", "音色選択", &[Key::T], |a, _| {
                    if super::super::parameter_editor::is_track(a) {
                        super::super::parameter_editor::open(a);
                    } else {
                        crate::player::bulk::request(a);
                    }
                });
                add("effects", "エフェクト", &[Key::X], |a, _| {
                    if super::super::parameter_editor::is_track(a) {
                        super::super::parameter_editor::open(a);
                    } else {
                        mixer::open_effects(a, false);
                    }
                });
            }
            if s == Screen::Timeline {
                add(
                    "timeline-close",
                    "閉じる",
                    &[Key::R, Key::Escape],
                    |a, _| {
                        a.piano_roll.open = false;
                        a.tempo_curve.open = false;
                        a.parameter_editor.close();
                    },
                );
            }
            if s != Screen::Mixer {
                add("play-menu", "再生menu", &[Key::P], |a, _| {
                    a.playback.open = true
                });
                add(
                    "play-last",
                    "前回の方法で再生",
                    &[Key::Space],
                    |a, _| playback::start(a, a.playback.last),
                );
                add("follow", "再生位置追従", &[Key::F], |a, _| {
                    a.playback.follow = !a.playback.follow
                });
            }
            if s != Screen::Timeline {
                add("mixer", "Mixer", &[Key::M], |a, _| {
                    if a.playback.mixer_focused() {
                        a.playback.mixer_open = false;
                    } else if a.playback.mixer_open {
                        a.playback.mixer_focus = true;
                    } else {
                        mixer::open(a);
                    }
                });
            }
            if s == Screen::Mixer {
                mixer_commands(a, s, out);
            }
        }
        _ => {}
    }
    if matches!(
        s,
        Screen::Main
            | Screen::Timeline
            | Screen::Overview
            | Screen::Mixer
            | Screen::Layers
            | Screen::Effects
    ) && a.undo.available()
    {
        out.push(Command::new(
            s,
            "undo",
            "元に戻す",
            &[(Key::Z, Modifiers::CTRL)],
            |a, _| super::super::undo::apply(a),
        ));
    }
    if matches!(
        s,
        Screen::Main
            | Screen::TrackRename
            | Screen::Timeline
            | Screen::Overview
            | Screen::Mixer
            | Screen::Layers
            | Screen::Effects
            | Screen::Mml
            | Screen::Selection
            | Screen::Note
            | Screen::Trill
            | Screen::LayerInput
            | Screen::ParameterInput
            | Screen::ParameterPicker
            | Screen::LayerHelp
            | Screen::EffectHelp
            | Screen::Help
            | Screen::Settings
            | Screen::Program
            | Screen::Edit
            | Screen::Go
            | Screen::Tracks
            | Screen::Split
            | Screen::Play
    ) {
        out.push(
            Command::new(
                s,
                "stop",
                "停止",
                &[(Key::Space, Modifiers::SHIFT)],
                |a, _| transport::stop(a),
            )
            .repeating(),
        );
    }
}

fn mixer_commands(a: &App, s: Screen, out: &mut Vec<Command>) {
    out.push(Command::plain(
        s,
        "mixer-unfocus",
        "Event listへ戻る",
        &[Key::Escape],
        mixer::unfocus,
    ));
    out.push(Command::plain(
        s,
        "spectrum",
        "Spectrum / Lissajous",
        &[Key::F],
        |a, _| super::super::analysis::toggle(a),
    ));
    for (id, name, keys, key) in [
        (
            "mixer-left",
            "前のtrack / master",
            &[Key::H, Key::ArrowLeft][..],
            mixer::Operation::Previous,
        ),
        (
            "mixer-right",
            "次のtrack",
            &[Key::L, Key::ArrowRight][..],
            mixer::Operation::Next,
        ),
        (
            "mixer-effects",
            "選択対象のエフェクト",
            &[Key::X, Key::Space, Key::Enter][..],
            mixer::Operation::Effects,
        ),
    ] {
        out.push(Command::plain(s, id, name, keys, move |a, _| {
            mixer::apply(a, key);
        }));
    }
    // When reached from Event list, enter() explicitly selects this track, not master.
    if !a.playback.mixer_focused() || !a.playback.mixer_master {
        for (id, name, keys, key) in [
            (
                "volume-down",
                "現在のtrackを減音 −3 dB",
                &[Key::J, Key::ArrowDown][..],
                mixer::Operation::Quieter,
            ),
            (
                "volume-up",
                "現在のtrackを増音 +3 dB",
                &[Key::K, Key::ArrowUp][..],
                mixer::Operation::Louder,
            ),
        ] {
            out.push(Command::plain(s, id, name, keys, move |a, _| {
                mixer::apply(a, key);
            }));
        }
    }
}
