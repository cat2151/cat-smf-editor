//! Song playback never changes the pattern editor's local cursor or projection.
use super::{song::Song, *};
use crate::play::{arrangement, playback::Mode, transport};
use cat_smf_editor::score::audition_timing::TempoMap;

pub(super) fn play(a: &mut App, row: usize, tick: u64, solo: bool) {
    let result = Song::read(a).and_then(|song| {
        let track = song.rows.get(row).ok_or("Missing overview row")?.track;
        let music = &a.loaded.as_ref().unwrap().music;
        let tempo = TempoMap::new(music, 0);
        Ok(arrangement::remaining_tracks(
            a,
            None,
            tempo.seconds_at(tick),
            f64::INFINITY,
            solo.then_some([track].as_slice()),
        ))
    });
    match result {
        Ok(prepared) => {
            transport::stop(a);
            a.playback.pending = prepared;
            a.playback.last = Mode::Arrangement(None);
            a.track_overview.cell = (row, tick);
            a.track_overview.solo = solo;
            a.playback.open = false;
        }
        Err(error) => a.load_error = Some(error),
    }
}

pub(super) fn playing(a: &App) -> Option<u64> {
    let active = a.playback.active.as_ref()?;
    (active.mode == Mode::Arrangement(None))
        .then(|| transport::tick_for(a, active.clock_track))
        .flatten()
}

pub(crate) fn seek(a: &mut App, tick: u64) {
    let active = a
        .playback
        .pending
        .as_ref()
        .map(|p| &p.active)
        .or(a.playback.active.as_ref());
    let Some(active) = active.filter(|p| p.mode == Mode::Arrangement(None)) else {
        return;
    };
    let tracks: Option<Vec<_>> = active.containers.as_ref().map(|ids| {
        a.loaded
            .as_ref()
            .and_then(|l| l.arrangement.as_ref())
            .map(|(c, _)| {
                ids.iter()
                    .filter_map(|id| c.tracks.get(id).copied())
                    .collect()
            })
            .unwrap_or_default()
    });
    let Some(tempo) = active.tempos.first() else {
        return;
    };
    let mut duration = f64::INFINITY;
    if let Some((start, end)) = a.playback.repeat {
        if (start..end).contains(&tick) {
            duration = tempo.duration(tick, end - tick);
        } else {
            a.playback.repeat = None;
            a.playback.repeat_mode = None;
        }
    }
    let prepared =
        arrangement::remaining_tracks(a, None, tempo.seconds_at(tick), duration, tracks.as_deref());
    a.audition.stop();
    a.playback.active = None;
    a.playback.pending = prepared;
}

pub(super) fn repeat(a: &mut App, action: crate::view::timeline_repeat::Action) {
    use crate::view::timeline_repeat::Action;
    match action {
        Action::Start(range) => {
            a.track_overview.solo = false;
            transport::repeat::start_song(a, range);
        }
        other => crate::view::timeline_repeat::apply(a, other),
    }
}

pub(super) fn jump(a: &mut App, row: usize, tick: u64) {
    let result = Song::read(a).and_then(|song| {
        let Some(o) = song.at(row, tick) else {
            return Ok(());
        };
        crate::view::patterns::library::edit(a, o.pattern)?;
        crate::view::patterns::timing::bind_occurrence(a, o.pattern, o.placement, o.scene);
        if let Some(edit) = &mut a.patterns.editing {
            edit.placement = Some(o.scene.map_or(o.placement, |s| s.1));
        }
        crate::cursor::moves::at_tick(a, tick - o.start);
        a.track_overview.open = false;
        a.piano_roll.open = false;
        a.tempo_curve.open = false;
        a.parameter_editor.close();
        a.patterns.root_focus = true;
        Ok(())
    });
    if let Err(error) = result {
        a.load_error = Some(error);
    }
}

pub(crate) fn menu(a: &mut App, ctx: &egui::Context) {
    let mut action = None;
    let mut stopped = false;
    let mut open = true;
    crate::view::presentation::window(
        ctx,
        crate::view::commands::Screen::Play,
        egui::Id::new("Play"),
        "Play",
    )
    .collapsible(false)
    .resizable(false)
    .open(&mut open)
    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
    .show(ctx, |ui| {
        if ui.button("Enter — Song").clicked() {
            action = Some(false);
        }
        if ui.button("T — Container").clicked() {
            action = Some(true);
        }
        if ui.button("D — Stop").clicked() {
            stopped = true;
        }
    });
    if stopped {
        transport::stop(a);
    }
    a.playback.open = open && action.is_none() && !stopped;
    if let Some(solo) = action {
        let (row, tick) = a.track_overview.cell;
        play(a, row, tick, solo);
    }
}

pub(crate) fn collect_menu(out: &mut Vec<Command>) {
    for (id, key, solo) in [
        ("overview.song-play", Key::Enter, false),
        ("overview.container-play", Key::T, true),
    ] {
        out.push(Command::plain(Screen::Play, id, &[key], move |a, _| {
            let (row, tick) = a.track_overview.cell;
            play(a, row, tick, solo);
        }));
    }
}
