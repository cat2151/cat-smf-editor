use super::*;
use crate::{
    play::{
        playback::{self, Mode},
        transport,
    },
    store::config::ProgramTarget,
    view::{comma_menu, edit_menu, note_edit::Field},
};

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    match s {
        Screen::Settings => {
            for (id, key, run) in [
                (
                    "settings.close",
                    Key::Escape,
                    (|a: &mut App, _: &egui::Context| a.comma_menu.open = false)
                        as fn(&mut App, &egui::Context),
                ),
                ("tempo.max", Key::M, |a, _| {
                    a.config
                        .select_tempo_curve_maximized(!a.config.tempo_curve_maximized)
                }),
                ("parameter.max", Key::C, |a, _| {
                    a.config
                        .select_parameter_curve_maximized(!a.config.parameter_curve_maximized)
                }),
                ("macro.variation-max", Key::U, |a, _| {
                    a.config.select_macro_variation_curve_maximized(
                        !a.config.macro_variation_curve_maximized,
                    )
                }),
                ("piano-roll.max", Key::R, |a, _| {
                    a.config
                        .select_piano_roll_maximized(!a.config.piano_roll_maximized)
                }),
                ("meta.all", Key::A, |a, _| {
                    a.config
                        .select_meta_text_visible(!a.config.meta_text_visible)
                }),
                ("program.target", Key::T, |a, _| a.comma_menu.program = true),
                ("mascot.playback", Key::B, |a, _| {
                    crate::play::mascot::toggle(a)
                }),
                ("spectrum.toggle", Key::F, |a, _| {
                    super::super::analysis::toggle(a)
                }),
                ("spectrum.reset", Key::P, |a, _| {
                    super::super::analysis::reset_window(a)
                }),
            ] {
                out.push(Command::plain(s, id, &[key], run));
            }
            out.push(Command::plain(
                s,
                "pattern.reset-windows",
                &[],
                super::super::patterns::windows::reset,
            ));
            out.push(Command::plain(
                s,
                "keyboard.reset-windows",
                &[],
                super::super::recording::reset_windows,
            ));
            if a.loaded.is_some() {
                out.push(Command::plain(s, "file.auto-save", &[Key::S], |a, _| {
                    a.toggle_auto_save()
                }));
            }
        }
        Screen::Program => {
            out.push(Command::plain(s, "program.auto", &[Key::A], |a, _| {
                comma_menu::choose(a, ProgramTarget::Auto)
            }));
            out.push(Command::plain(s, "program.current", &[Key::C], |a, _| {
                comma_menu::choose(a, ProgramTarget::Current)
            }));
            out.push(Command::plain(
                s,
                "program.close",
                &[Key::Escape],
                |a, _| a.comma_menu.open = false,
            ));
        }
        Screen::Edit => {
            out.push(Command::plain(s, "edit.close", &[Key::Escape], |a, _| {
                a.edit_menu.open = false
            }));
            if a.undo.available() {
                out.push(Command::plain(s, "edit.undo", &[Key::U], |a, _| {
                    a.edit_menu.open = false;
                    super::super::undo::apply(a);
                }));
            }
            if let Some(l) = &a.loaded
                && let Some(e) = a.selected_event
                && l.visible().tracks[a.track]
                    .events
                    .get(e)
                    .is_some_and(cat_smf_editor::score::note_view::note_on)
                && cat_smf_editor::edit::trill::ensure_note_editable(l.visible(), a.track, e)
                    .is_ok()
            {
                out.push(Command::plain(s, "edit.pitch", &[Key::P], |a, _| {
                    edit_menu::choose(a, Field::Pitch)
                }));
                out.push(Command::plain(s, "edit.velocity", &[Key::V], |a, _| {
                    edit_menu::choose(a, Field::Velocity)
                }));
                out.push(Command::plain(s, "edit.mbt", &[Key::M], |a, _| {
                    edit_menu::choose(a, Field::Mbt)
                }));
                out.push(Command::plain(s, "edit.duration", &[Key::D], |a, _| {
                    edit_menu::choose(a, Field::Duration)
                }));
                out.push(Command::plain(s, "edit.lyrics", &[Key::L], |a, _| {
                    edit_menu::choose(a, Field::Lyrics)
                }));
            }
        }
        Screen::Play => {
            out.push(Command::plain(s, "play.close", &[Key::Escape], |a, _| {
                a.playback.open = false
            }));
            out.push(Command::plain(s, "play.stop", &[Key::D], |a, _| {
                transport::stop(a);
                a.playback.open = false;
            }));
            if a.loaded.is_none() {
                return;
            }
            if crate::view::track_overview::is_song(a) {
                crate::view::track_overview::song_playback::collect_menu(out);
                return;
            }
            for (id, key, mode) in [
                ("play.all", Key::A, Mode::AllTracks),
                ("play.all-measure", Key::C, Mode::AllTracksFromMeasure),
                ("play.measure-start", Key::S, Mode::MeasureStart),
                ("play.measure", Key::M, Mode::Measure),
                ("play.track", Key::T, Mode::Track),
                ("play.position", Key::P, Mode::Position),
                ("play.beat", Key::B, Mode::Beat),
                ("play.beat-only", Key::I, Mode::BeatOnly),
                ("play.note", Key::E, Mode::Event),
                ("play.chord", Key::N, Mode::Notes),
                ("play.last", Key::Space, a.playback.last),
            ] {
                out.push(Command::plain(s, id, &[key], move |a, _| {
                    playback::start(a, mode)
                }));
            }
        }
        _ => {}
    }
}
