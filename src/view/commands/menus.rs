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
            for (id, name, key, run) in [
                (
                    "settings-close",
                    "閉じる",
                    Key::Escape,
                    (|a: &mut App, _: &egui::Context| a.comma_menu.open = false)
                        as fn(&mut App, &egui::Context),
                ),
                (
                    "tempo-max",
                    "Tempo curve always maximized",
                    Key::M,
                    |a, _| {
                        a.config
                            .select_tempo_curve_maximized(!a.config.tempo_curve_maximized)
                    },
                ),
                (
                    "parameter-max",
                    "Parameter curve always maximized",
                    Key::C,
                    |a, _| {
                        a.config
                            .select_parameter_curve_maximized(!a.config.parameter_curve_maximized)
                    },
                ),
                (
                    "macro-variation-max",
                    "Macro variation curve always maximized",
                    Key::U,
                    |a, _| {
                        a.config.select_macro_variation_curve_maximized(
                            !a.config.macro_variation_curve_maximized,
                        )
                    },
                ),
                ("roll-max", "Piano roll always maximized", Key::R, |a, _| {
                    a.config
                        .select_piano_roll_maximized(!a.config.piano_roll_maximized)
                }),
                ("meta-all", "All tracks meta text", Key::A, |a, _| {
                    a.config
                        .select_meta_text_visible(!a.config.meta_text_visible)
                }),
                ("program-target", "Program change target", Key::T, |a, _| {
                    a.comma_menu.program = true
                }),
                (
                    "mascot-playback",
                    "ずんだもん演奏連動",
                    Key::B,
                    |a, _| crate::play::mascot::toggle(a),
                ),
                ("spectrum", "Spectrum / Lissajous", Key::F, |a, _| {
                    super::super::analysis::toggle(a)
                }),
                (
                    "spectrum-reset",
                    "Reset spectrum window position",
                    Key::P,
                    |a, _| super::super::analysis::reset_window(a),
                ),
            ] {
                out.push(Command::plain(s, id, name, &[key], run));
            }
            out.push(Command::plain(
                s,
                "pattern-windows-reset",
                "Reset pattern windows position",
                &[],
                super::super::patterns::windows::reset,
            ));
            out.push(Command::plain(
                s,
                "keyboard-windows-reset",
                "Reset virtual MIDI keyboard windows position",
                &[],
                super::super::recording::reset_windows,
            ));
            if a.loaded.is_some() {
                out.push(Command::plain(
                    s,
                    "auto-save",
                    "Auto save",
                    &[Key::S],
                    |a, _| a.toggle_auto_save(),
                ));
            }
        }
        Screen::Program => {
            out.push(Command::plain(
                s,
                "program-auto",
                "Auto",
                &[Key::A],
                |a, _| comma_menu::choose(a, ProgramTarget::Auto),
            ));
            out.push(Command::plain(
                s,
                "program-current",
                "Current track",
                &[Key::C],
                |a, _| comma_menu::choose(a, ProgramTarget::Current),
            ));
            out.push(Command::plain(
                s,
                "program-close",
                "閉じる",
                &[Key::Escape],
                |a, _| a.comma_menu.open = false,
            ));
        }
        Screen::Edit => {
            out.push(Command::plain(
                s,
                "edit-close",
                "閉じる",
                &[Key::Escape],
                |a, _| a.edit_menu.open = false,
            ));
            if a.undo.available() {
                out.push(Command::plain(
                    s,
                    "edit-undo",
                    "元に戻す",
                    &[Key::U],
                    |a, _| {
                        a.edit_menu.open = false;
                        super::super::undo::apply(a);
                    },
                ));
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
                out.push(Command::plain(
                    s,
                    "edit-pitch",
                    "Pitch",
                    &[Key::P],
                    |a, _| edit_menu::choose(a, Field::Pitch),
                ));
                out.push(Command::plain(
                    s,
                    "edit-velocity",
                    "Velocity",
                    &[Key::V],
                    |a, _| edit_menu::choose(a, Field::Velocity),
                ));
                out.push(Command::plain(s, "edit-mbt", "MBT", &[Key::M], |a, _| {
                    edit_menu::choose(a, Field::Mbt)
                }));
                out.push(Command::plain(
                    s,
                    "edit-duration",
                    "Duration",
                    &[Key::D],
                    |a, _| edit_menu::choose(a, Field::Duration),
                ));
                out.push(Command::plain(
                    s,
                    "edit-lyrics",
                    "Lyrics",
                    &[Key::L],
                    |a, _| edit_menu::choose(a, Field::Lyrics),
                ));
            }
        }
        Screen::Play => {
            out.push(Command::plain(
                s,
                "play-close",
                "閉じる",
                &[Key::Escape],
                |a, _| a.playback.open = false,
            ));
            out.push(Command::plain(
                s,
                "play-stop",
                "停止",
                &[Key::D],
                |a, _| {
                    transport::stop(a);
                    a.playback.open = false;
                },
            ));
            if a.loaded.is_none() {
                return;
            }
            for (id, name, key, mode) in [
                ("play-all", "All tracks from start", Key::A, Mode::AllTracks),
                (
                    "play-all-measure",
                    "All tracks from measure",
                    Key::C,
                    Mode::AllTracksFromMeasure,
                ),
                (
                    "play-measure-start",
                    "Measure start to end",
                    Key::S,
                    Mode::MeasureStart,
                ),
                ("play-measure", "Current measure", Key::M, Mode::Measure),
                ("play-track", "Entire current track", Key::T, Mode::Track),
                (
                    "play-position",
                    "Current MBT to end",
                    Key::P,
                    Mode::Position,
                ),
                ("play-beat", "Beat start to end", Key::B, Mode::Beat),
                ("play-beat-only", "Current beat", Key::I, Mode::BeatOnly),
                ("play-note", "Single note", Key::E, Mode::Event),
                ("play-chord", "Same-tick notes", Key::N, Mode::Notes),
                ("play-last", "前回の方法で再生", Key::Space, a.playback.last),
            ] {
                out.push(Command::plain(s, id, name, &[key], move |a, _| {
                    playback::start(a, mode)
                }));
            }
        }
        _ => {}
    }
}
