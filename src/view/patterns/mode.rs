//! One boundary between the Notes projection and the saved SMF.
use super::*;
use cat_smf_editor::model::Event;
pub fn actual_source(a: &App, track: usize) -> bool {
    a.loaded
        .as_ref()
        .is_some_and(|l| cat_smf_editor::score::pattern::is_source(&l.music, track))
}
pub fn workspace_open(a: &App) -> bool {
    a.note_mode && (actual_source(a, a.track) || a.patterns.workspace_open)
}
/// Pattern framing is derived from the real window lifetime and real track role.
pub fn source(a: &App, track: usize) -> bool {
    workspace_open(a) && a.track == track && actual_source(a, track)
}
pub fn can_edit_notes(a: &App, track: usize) -> bool {
    !a.note_mode
        || a.loaded.as_ref().is_some_and(|l| {
            !cat_smf_editor::score::pattern::is_source(&l.music, track)
                || source(a, track) && editing(a, track).is_some_and(|c| c.standalone)
        })
}
pub fn close_workspace(a: &mut App) {
    let tick = crate::play::playback::location(a).0;
    if a.patterns.preview.is_some() || arrangement_playing(a) {
        crate::play::transport::stop(a);
    }
    a.patterns.workspace.dialog = None;
    a.patterns.workspace.return_to = None;
    workspace::reset_windows(a);
    // Leave the source so closing Song never changes Notes / Raw.
    if a.note_mode && actual_source(a, a.track) {
        let destination = a.loaded.as_ref().and_then(|l| {
            (0..l.music.tracks.len())
                .filter(|&track| !actual_source(a, track))
                .min_by_key(|&track| (track.abs_diff(a.track), track))
        });
        let Some(track) = destination else { return };
        a.select_track(track);
    }
    a.patterns.workspace_open = false;
    a.patterns.raw_context = None;
    a.note_edit = None;
    a.note_edit_request = None;
    if a.patterns.editing.take().is_some() {
        a.generation = a.generation.wrapping_add(1);
    }
    refresh(a);
    crate::cursor::moves::at_tick(a, tick);
}
/// The same live projection authorizes edits and supplies pattern identity.
pub fn editing(a: &App, track: usize) -> Option<&cat_smf_editor::score::pattern::editing::Context> {
    a.patterns.editing.as_ref().filter(|c| {
        workspace_open(a)
            && a.patterns.edit_song == a.song_id
            && c.track == track
            && a.track == track
            && a.loaded.as_ref().is_some_and(|l| l.display_music.is_some())
    })
}
pub fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if a.loaded.is_some()
        && (matches!(
            s,
            Screen::Main | Screen::Timeline | Screen::Scene | Screen::Song
        ))
        && !a.patterns.confirming_creation()
    {
        out.push(Command::plain(
            s,
            "view-mode",
            "Notes / Raw",
            &[eframe::egui::Key::V],
            |a, ctx| {
                toggle(a);
                super::presentation::focus_root(a, ctx);
            },
        ));
    }
}
fn same(a: &Event, b: &Event) -> bool {
    a.tick == b.tick && a.status == b.status && a.meta_type == b.meta_type && a.data == b.data
}
pub fn toggle(a: &mut App) {
    let mut tick = crate::play::playback::location(a).0;
    let mut selected = a.loaded.as_ref().and_then(|l| {
        a.selected_event.and_then(|i| {
            let events = &l.visible().tracks[a.track].events;
            let e = events.get(i)?;
            Some((
                e.clone(),
                events[..i].iter().filter(|old| same(old, e)).count(),
            ))
        })
    });
    let context = if a.note_mode {
        a.patterns.editing.as_ref()
    } else {
        a.patterns.raw_context.as_ref()
    };
    if let Some(context) = context.filter(|c| c.standalone)
        && let Some(p) = a
            .loaded
            .as_ref()
            .and_then(|l| cat_smf_editor::score::pattern::Catalog::read(&l.music).ok())
            .and_then(|c| c.patterns.get(&context.pattern).cloned())
    {
        let map = |t: u64| {
            if a.note_mode {
                t.checked_add(p.tick)
            } else {
                t.checked_sub(p.tick)
            }
        };
        tick = map(tick).unwrap_or(tick);
        if let Some((e, _)) = &mut selected {
            e.tick = map(e.tick).unwrap_or(e.tick);
        }
    }
    a.note_mode = !a.note_mode;
    a.piano_roll.open = false;
    a.tempo_curve.open = false;
    a.parameter_editor.close();
    if !a.note_mode {
        if a.patterns.preview.is_some() || arrangement_playing(a) {
            crate::play::transport::stop(a);
        }
        a.patterns.workspace.dialog = None;
        a.patterns.workspace.return_to = None;
        workspace::reset_windows(a);
        a.patterns.raw_context = a.patterns.editing.take();
        a.patterns.workspace_open = false;
    } else {
        a.patterns.editing = a.patterns.raw_context.take().filter(|c| {
            c.track == a.track
                && a.loaded.as_ref().is_some_and(|l| {
                    if c.standalone {
                        cat_smf_editor::score::pattern::Catalog::read(&l.music)
                            .ok()
                            .and_then(|catalog| catalog.patterns.get(&c.pattern).cloned())
                            .is_some_and(|p| tick < p.length().unwrap_or(0))
                            && c.project(&l.music).is_ok()
                    } else {
                        let time = cat_smf_editor::score::musical_time::MusicalTime::new(&l.music);
                        time.map(c.track)
                            .is_some_and(|m| m.position(tick).measure == c.measure)
                            && c.project(&l.music).is_ok()
                    }
                })
        });
        a.patterns.edit_song = a.song_id;
        // Sources derive their lifetime directly; only a placed song destination
        // needs an explicit companion-window request when its target is restored.
        a.patterns.workspace_open = a.patterns.editing.as_ref().is_some_and(|c| !c.standalone);
    }
    refresh(a);
    a.generation = a.generation.wrapping_add(1);
    crate::cursor::moves::at_tick(a, tick);
    if let Some(l) = &a.loaded {
        let events = &l.visible().tracks[a.track].events;
        let index = selected
            .as_ref()
            .and_then(|(e, n)| {
                events
                    .iter()
                    .enumerate()
                    .filter(|(_, next)| same(e, next))
                    .nth(*n)
                    .map(|(i, _)| i)
            })
            .or_else(|| events.iter().position(|e| e.tick == tick));
        if let Some(index) = index {
            let row = if a.note_mode {
                l.display_rows[a.track].event_row(index)
            } else {
                Some(index)
            };
            if let Some(row) = row {
                a.selected_event = Some(index);
                a.cursor_row = Some(row);
                a.cursor_position = None;
                a.track_view.cursor_tick = None;
            }
        }
    }
    a.reset_scroll = true;
}
fn arrangement_playing(a: &App) -> bool {
    a.playback
        .pending
        .as_ref()
        .is_some_and(|p| matches!(p.active.mode, crate::play::playback::Mode::Arrangement(_)))
        || a.playback
            .active
            .as_ref()
            .is_some_and(|p| matches!(p.mode, crate::play::playback::Mode::Arrangement(_)))
}
