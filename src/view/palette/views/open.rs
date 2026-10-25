use super::*;

pub(super) fn show(app: &mut App, view: View) {
    app.piano_roll.open = false;
    app.tempo_curve.open = false;
    app.parameter_editor.close();
    app.layer_editor.open = false;
    app.drum_editor.open = false;
    app.effect_editor = Default::default();
    app.playback.mixer_open = false;
    app.playback.open = false;
    app.help_open = false;
    app.comma_menu.open = false;
    app.edit_menu.open = false;
    app.track_view.go_open = false;
    app.track_view.split_open = false;
    match view {
        View::Mixer => crate::play::mixer::open(app),
        View::DoubleScore => {
            if !app.note_mode {
                app.note_mode = true;
                crate::cursor::moves::sync_mode(app);
            }
            crate::cursor::split::split_on(app);
        }
        View::Tempo => app.tempo_curve.open_at_cursor(),
        View::Parameter(track) => {
            if let Some(owner) = app
                .loaded
                .as_ref()
                .and_then(|l| cat_smf_editor::edit::parameter_curve::read(&l.music).ok())
                .and_then(|c| c.owner(track))
                && owner != app.track
            {
                crate::cursor::split::go(app, owner, crate::play::playback::location(app).0);
            }
            crate::view::parameter_editor::open_existing(app, track);
        }
        View::Score(track)
        | View::Piano(track)
        | View::Instruments(track)
        | View::Effects(track)
        | View::Macros(track, _)
        | View::Variation(track, _) => {
            if app.track != track {
                crate::cursor::split::go(app, track, crate::play::playback::location(app).0);
            }
            match view {
                View::Score(_) => crate::cursor::split::split_off(app),
                View::Piano(_) => app.piano_roll.open_at_cursor(),
                View::Instruments(_) => crate::view::layers::open(app),
                View::Effects(_) => {
                    crate::view::effects::open(app, crate::view::effects::Target::Track(track), 0)
                }
                View::Macros(_, layer) => crate::view::macro_editor::open_existing(app, layer),
                View::Variation(_, layer) => {
                    crate::view::parameter_editor::open_existing_variation(app, layer)
                }
                _ => {}
            }
        }
    }
}
