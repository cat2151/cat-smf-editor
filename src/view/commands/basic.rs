use super::*;
use crate::{
    play::{mixer, playback, transport},
    view::{busy, files},
};

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if matches!(s, Screen::Global) {
        out.push(Command::plain(s, "app.quit", &[], |a, c| {
            a.request_close(c);
        }));
        out.push(Command::new(
            s,
            "file.open",
            &[(Key::O, Modifiers::CTRL)],
            |a, _| a.file_open_requested = true,
        ));
        out.push(Command::new(
            s,
            "file.new",
            &[(Key::N, Modifiers::CTRL)],
            |a, c| a.new_smf(c),
        ));
        out.push(
            Command::new(s, "recent.open", &[(Key::P, Modifiers::CTRL)], |a, _| {
                super::super::recent::commands::open(a)
            })
            .repeating(),
        );
        if a.loaded.is_some() {
            out.push(
                Command::new(s, "file.save", &[(Key::S, Modifiers::CTRL)], |a, _| {
                    a.allow_departure(files::Action::Save);
                })
                .repeating(),
            );
            out.push(Command::plain(s, "file.export-performance", &[], |a, _| {
                files::performance::dialog(a)
            }));
        }
    }
    if matches!(s, Screen::Main) && a.loaded.is_some() {
        super::guide::collect(a, s, out);
        for (id, kind) in [
            ("event.insert", None),
            (
                "event.insert-tempo",
                Some(cat_smf_editor::edit::meta_input::Kind::Tempo),
            ),
            (
                "event.insert-key-signature",
                Some(cat_smf_editor::edit::meta_input::Kind::KeySignature),
            ),
        ] {
            out.push(Command::operation(
                s,
                id,
                Operation::InsertEvent { kind, text: None },
            ));
        }
        out.push(Command::operation(
            s,
            "event.insert-same-measure",
            Operation::SameMeasure {
                batch: false,
                number: None,
            },
        ));
        out.push(Command::new(
            s,
            "track.add",
            &[(Key::N, Modifiers::CTRL | Modifiers::SHIFT)],
            |a, c| busy::request(a, c, busy::Task::AddTrack),
        ));
        out.push(Command::new(
            s,
            "layer.open",
            &[(Key::T, Modifiers::SHIFT)],
            |a, _| super::super::layers::open(a),
        ));
        out.push(Command::new(
            s,
            "drum.open",
            &[(Key::D, Modifiers::SHIFT)],
            |a, _| super::super::drums::open(a),
        ));
        out.push(Command::plain(s, "kit.add-track", &[], |a, _| {
            crate::player::kit_selection::open(a, true, false)
        }));
        out.push(Command::plain(s, "drum.add-track", &[], |a, _| {
            super::super::drums::add_track(a)
        }));
    }
    if matches!(
        s,
        Screen::Main | Screen::Score | Screen::Timeline | Screen::Overview
    ) {
        out.push(
            Command::new(
                s,
                "settings.open",
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
        out.push(Command::operation(s, "view.browse", Operation::Views));
    }
    let mut add = |id, keys: &[Key], run: fn(&mut App, &egui::Context)| {
        let mut command = Command::plain(s, id, keys, run);
        command.repeat = matches!(s, Screen::Confirm | Screen::Invalid)
            || matches!(
                id,
                "settings.open" | "edit.open" | "edit.repeat" | "timeline.open" | "play.follow"
            );
        out.push(command);
    };
    match s {
        Screen::Browser if a.player_selection.no_notes_error => {
            add("browser.close-error", &[Key::Enter, Key::Escape], |a, _| {
                a.player_selection.no_notes_error = false;
                a.player_selection.open = false;
                a.player_selection.error = None;
            })
        }
        Screen::Invalid => add("invalid.close", &[Key::Enter, Key::Escape], |a, _| {
            a.invalid_smf_name = None;
            a.load_error = None;
        }),
        Screen::Missing => {
            add("missing.new", &[Key::N], |a, _| files::create_new_song(a));
            add("missing.quit", &[Key::Q], |a, c| a.request_close(c));
        }
        Screen::Confirm => {
            add("save.confirm", &[Key::Y], |a, c| {
                a.finish_departure(true, c)
            });
            add("save.cancel", &[Key::Escape], |a, _| {
                a.config.navigation.departure = None;
                a.config.navigation.departure_error = None;
            });
            add("save.discard", &[Key::N], |a, c| {
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
                add("backup.open", &[Key::O], files::open_backup);
            }
        }
        Screen::Backup => {
            add("backup.close", &[Key::Enter, Key::Escape], |a, _| {
                files::close_backup_notice(a)
            });
            add("backup.open", &[Key::O], files::open_backup);
        }
        Screen::Help => {
            add("app.help", &[Key::F1], |a, _| a.help_open = false);
            add("help.close", &[Key::Escape], |a, _| a.help_open = false);
        }
        Screen::Analysis => add("spectrum.close", &[Key::F, Key::Escape], |a, _| {
            if a.analysis.is_open() {
                super::super::analysis::toggle(a);
            }
        }),
        Screen::Note => {
            add(
                "note.confirm",
                &[Key::Enter],
                super::super::note_edit::confirm,
            );
            add("note.cancel", &[Key::Escape], |a, _| a.note_edit = None);
        }
        Screen::Trill => add(
            "trill.cancel",
            &[Key::Escape],
            super::super::trill_edit::cancel,
        ),
        Screen::Main | Screen::Score | Screen::Timeline | Screen::Mixer => {
            if s != Screen::Mixer {
                add("app.help", &[Key::F1], |a, _| crate::view::help::summon(a));
            }
            if a.loaded.is_none() {
                return;
            }
            if s == Screen::Timeline && !a.parameter_editor.open {
                add("mml.open", &[Key::I], |a, c| {
                    busy::request(a, c, busy::Task::MmlInput)
                });
            }
            if s == Screen::Main {
                add("edit.open", &[Key::E], |a, _| a.edit_menu.open = true);
                if a.last_command.is_some() {
                    add("edit.repeat", &[Key::Period], |a, _| {
                        super::super::edit_menu::repeat(a)
                    });
                }
                add("timeline.open", &[Key::R], |a, _| {
                    super::super::piano_roll::open(a)
                });
                add("go.open", &[Key::G], |a, _| {
                    a.track_view.origin_tick = playback::location(a).0;
                    a.track_view.go_open = true;
                    a.track_view.track_list = false;
                });
                add("split.open", &[Key::W], |a, _| {
                    a.track_view.split_open = true
                });
                add("mml.open", &[Key::I], |a, c| {
                    super::super::drums::input(a, c)
                });
                add("mml.measure", &[], |a, c| {
                    use crate::view::mml_input::measure;
                    measure::begin(a, c)
                });
                add("track.patch", &[Key::T], |a, _| {
                    if super::super::parameter_editor::is_track(a) {
                        super::super::parameter_editor::open(a);
                    } else {
                        crate::player::bulk::request(a);
                    }
                });
                add("effect.open", &[Key::X], |a, _| {
                    if super::super::parameter_editor::is_track(a) {
                        super::super::parameter_editor::open(a);
                    } else {
                        mixer::open_effects(a, false);
                    }
                });
            }
            if s == Screen::Timeline {
                add("timeline.close", &[Key::R, Key::Escape], |a, _| {
                    a.piano_roll.open = false;
                    a.tempo_curve.open = false;
                    a.parameter_editor.close();
                });
            }
            if s != Screen::Mixer {
                add("play.open", &[Key::P], |a, ctx| {
                    a.playback.open = true;
                    a.playback.view = crate::view::dock::locate::opener(a, ctx);
                });
                add("play.last", &[Key::Space], |a, _| {
                    playback::start(a, a.playback.last)
                });
                add("play.follow", &[Key::F], |a, _| {
                    a.playback.follow = !a.playback.follow
                });
            }
            if s != Screen::Timeline {
                add("mixer.open", &[Key::M], |a, _| {
                    if a.playback.mixer_focused() {
                        a.playback.mixer_open = false;
                    } else if a.playback.mixer_open {
                        mixer::focus(a);
                    } else {
                        mixer::open(a);
                    }
                });
            }
            if s == Screen::Mixer {
                super::mixer::collect(a, s, out);
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
            "app.undo",
            &[(Key::Z, Modifiers::CTRL)],
            |a, _| super::super::undo::apply(a),
        ));
    }
    if matches!(
        s,
        Screen::Main
            | Screen::Score
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
            | Screen::Settings
            | Screen::Program
            | Screen::Edit
            | Screen::Go
            | Screen::Tracks
            | Screen::Split
            | Screen::Play
            | Screen::Patterns
    ) {
        out.push(
            Command::new(s, "app.stop", &[(Key::Space, Modifiers::SHIFT)], |a, _| {
                transport::stop(a)
            })
            .repeating(),
        );
    }
}
