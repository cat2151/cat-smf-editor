use super::*;
use cat_smf_editor::score::chord_guide;

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    let music = &a.loaded.as_ref().unwrap().music;
    if !chord_guide::supported(music)
        || a.patterns.editing.is_some()
        || a.patterns.raw_context.is_some()
    {
        return;
    }
    if chord_guide::track(music) == Some(a.track) {
        out.push(Command::operation(
            s,
            "guide.insert-chord",
            Operation::InsertEvent {
                kind: Some(cat_smf_editor::edit::meta_input::Kind::Chord),
                text: None,
            },
        ));
        if let Some(index) = crate::view::command_input::guide_edit::selected(a)
            && music.tracks[a.track].events.get(index).is_some_and(|e| {
                chord_guide::chord(e).is_some()
                    || cat_smf_editor::score::key_signature::KeySignature::from_event(e).is_some()
            })
        {
            for (id, moving) in [("guide.edit-event", false), ("guide.move-event", true)] {
                out.push(Command::operation(s, id, Operation::GuideEvent { moving }));
            }
            out.push(Command::plain(s, "guide.delete-event", &[], |a, _| {
                if let Err(error) = crate::view::command_input::guide_edit::delete(a) {
                    a.load_error = Some(error);
                }
            }));
        }
    }
    if chord_guide::track(music).is_none() {
        out.push(Command::plain(s, "guide.add-track", &[], |a, _| {
            match a.edit_tracks(chord_guide::add) {
                Ok(()) => a.select_track(a.loaded.as_ref().unwrap().music.tracks.len() - 1),
                Err(error) => a.load_error = Some(error),
            }
        }));
    }
}
