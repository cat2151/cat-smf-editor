use crate::{
    app::App,
    play::audio_sequence::{Note, Sequence},
};
use cat_smf_editor::{
    score::audition_timing::TempoMap, score::display_rows::DisplayRow,
    score::musical_time::Position, score::note_view::note_on,
};
use eframe::egui;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    AllTracks,
    AllTracksFromMeasure,
    MeasureStart,
    Measure,
    Track,
    #[default]
    Position,
    Beat,
    BeatOnly,
    Event,
    Notes,
}
pub struct Playback {
    pub open: bool,
    pub last: Mode,
    pub pending: Option<crate::play::transport::Prepared>,
    pub active: Option<crate::play::transport::Active>,
    pub repeat: Option<(u64, u64)>,
    pub follow: bool,
    pub mixer_open: bool,
    /// Keys go to the open mixer; otherwise to the views beside it.
    pub mixer_focus: bool,
    pub mixer_track: usize,
    pub mixer_master: bool,
}

impl Playback {
    pub fn mixer_focused(&self) -> bool {
        self.mixer_open && self.mixer_focus
    }
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            open: false,
            last: Mode::default(),
            pending: None,
            active: None,
            repeat: None,
            follow: true,
            mixer_open: false,
            mixer_focus: true,
            mixer_track: 0,
            mixer_master: true,
        }
    }
}

pub fn location(app: &App) -> (u64, Option<Position>) {
    let Some(loaded) = &app.loaded else {
        return (0, None);
    };
    let map = loaded.time.map(app.track);
    if let Some(tick) = app.track_view.cursor_tick {
        return (tick, map.map(|m| m.position(tick)));
    }
    if let Some(position) = &app.cursor_position {
        return (
            map.and_then(|m| m.tick_at(position)).unwrap_or(0),
            Some(position.clone()),
        );
    }
    let source = if app.note_mode {
        match app
            .cursor_row
            .map(|r| loaded.display_rows[app.track].get(r))
        {
            Some(DisplayRow::Beat(p)) => {
                return (map.and_then(|m| m.tick_at(&p)).unwrap_or(0), Some(p));
            }
            Some(DisplayRow::Event(i)) => Some(i),
            None => app.selected_event,
        }
    } else {
        app.cursor_row.or(app.selected_event)
    };
    let tick = source.map_or(0, |i| loaded.music.tracks[app.track].events[i].tick);
    (tick, map.map(|m| m.position(tick)))
}

#[cfg(test)]
pub fn sequence(app: &App, mode: Mode) -> Option<Sequence> {
    sequence_track(app, app.track, mode)
}

fn sequence_track(app: &App, track: usize, mode: Mode) -> Option<Sequence> {
    let loaded = app.loaded.as_ref()?;
    if mode == Mode::AllTracks {
        let mut notes = Vec::new();
        let mut end: f64 = 0.0;
        for (track, source) in loaded.music.tracks.iter().enumerate() {
            let end_tick = source.events.last().map_or(0, |e| e.tick);
            let tempo = TempoMap::new(&loaded.music, track);
            end = end.max(tempo.duration(0, end_tick));
            for (i, event) in source.events.iter().enumerate().filter(|(_, e)| note_on(e)) {
                let duration = loaded.note_views[track].durations[i]
                    .unwrap_or(end_tick.saturating_sub(event.tick))
                    .min(end_tick.saturating_sub(event.tick));
                notes.push(Note {
                    start: tempo.duration(0, event.tick),
                    duration: tempo.duration(event.tick, duration),
                    pitch: event.data[0],
                    velocity: event.data[1],
                });
            }
        }
        notes.sort_by(|a, b| a.start.total_cmp(&b.start));
        return Some(Sequence { notes, end });
    }
    let events = &loaded.music.tracks[track].events;
    let end_tick = events.last()?.tick;
    let (tick, position) = location(app);
    let mut start = tick;
    let mut end = end_tick;
    if mode == Mode::Track {
        start = 0;
    }
    if matches!(
        mode,
        Mode::Measure | Mode::MeasureStart | Mode::Beat | Mode::BeatOnly
    ) {
        let map = loaded.time.map(app.track)?;
        let mut p = position?;
        p.tick_numerator = 0;
        p.tick_denominator = 1;
        if matches!(mode, Mode::Measure | Mode::MeasureStart) {
            p.beat = 1;
        }
        start = map.tick_at(&p)?;
        if mode == Mode::Measure {
            p.measure += 1;
            end = map.tick_at(&p)?.min(end);
        } else if mode == Mode::BeatOnly {
            p.beat += 1;
            end = map.tick_at(&p)?.min(end);
        }
    }
    let tempo = TempoMap::new(&loaded.music, track);
    let notes: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            note_on(e)
                && match mode {
                    Mode::Event if track == app.track => app.selected_event == Some(*i),
                    Mode::Event => e.tick == tick,
                    Mode::Notes => e.tick == tick,
                    _ => e.tick >= start && e.tick < end,
                }
        })
        .map(|(i, e)| {
            let duration =
                loaded.note_views[track].durations[i].unwrap_or(end.saturating_sub(e.tick));
            let duration = if matches!(mode, Mode::Notes | Mode::Event) {
                duration
            } else {
                duration.min(end.saturating_sub(e.tick))
            };
            Note {
                start: tempo.duration(start, e.tick.saturating_sub(start)),
                duration: if matches!(mode, Mode::Event | Mode::Notes)
                    && loaded.note_views[track].durations[i].is_none()
                {
                    f64::INFINITY
                } else {
                    tempo.duration(e.tick, duration)
                },
                pitch: e.data[0],
                velocity: e.data[1],
            }
        })
        .collect();
    let end = if matches!(mode, Mode::Event | Mode::Notes) {
        notes
            .iter()
            .map(|n| n.start + n.duration)
            .fold(0.0, f64::max)
    } else {
        tempo.duration(start, end.saturating_sub(start))
    };
    Some(Sequence { notes, end })
}

pub fn start(app: &mut App, mode: Mode) {
    app.playback.repeat = None;
    app.playback.last = mode;
    app.playback.open = false;
    app.audition.stop();
    app.playback.active = None;
    app.playback.pending = crate::play::transport::prepare(app, mode);
}

pub fn parts(
    app: &App,
    mode: Mode,
) -> Vec<(usize, cat_smf_editor::score::instrument::Patch, Sequence)> {
    let Some(loaded) = &app.loaded else {
        return Vec::new();
    };
    let tracks = if mode == Mode::AllTracks || current_track_is_empty(app) {
        (0..loaded.music.tracks.len()).collect()
    } else {
        vec![app.track]
    };
    tracks
        .into_iter()
        .filter_map(|track| {
            let sequence = sequence_track(
                app,
                track,
                if mode == Mode::AllTracks {
                    Mode::Track
                } else {
                    mode
                },
            )?;
            if sequence.notes.is_empty() && sequence.end <= 0.0 {
                return None;
            }
            Some((track, loaded.patches[track].clone(), sequence))
        })
        .collect()
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if ctx.wants_keyboard_input() || app.help_open || app.playback.mixer_focused() {
        return;
    }
    let mut chosen = None;
    ctx.input_mut(|input| {
        input.events.retain(|event| {
            let egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = event
            else {
                return true;
            };
            if *modifiers != egui::Modifiers::NONE {
                return true;
            }
            if app.playback.open {
                chosen = match key {
                    egui::Key::A => Some(Mode::AllTracks),
                    egui::Key::C => Some(Mode::AllTracksFromMeasure),
                    egui::Key::S => Some(Mode::MeasureStart),
                    egui::Key::M => Some(Mode::Measure),
                    egui::Key::T => Some(Mode::Track),
                    egui::Key::P => Some(Mode::Position),
                    egui::Key::B => Some(Mode::Beat),
                    egui::Key::I => Some(Mode::BeatOnly),
                    egui::Key::E => Some(Mode::Event),
                    egui::Key::N => Some(Mode::Notes),
                    egui::Key::Space => Some(app.playback.last),
                    egui::Key::D => {
                        crate::play::transport::stop(app);
                        app.playback.open = false;
                        None
                    }
                    egui::Key::Escape => {
                        app.playback.open = false;
                        None
                    }
                    _ => return false,
                };
            } else {
                match key {
                    egui::Key::P => app.playback.open = true,
                    egui::Key::Space => chosen = Some(app.playback.last),
                    _ => return true,
                }
            }
            false
        });
    });
    if let Some(mode) = chosen {
        start(app, mode);
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.playback.open {
        return;
    }
    let mut open = true;
    let mut chosen = None;
    let mut stopped = false;
    let empty_track = current_track_is_empty(app);
    egui::Window::new("Play")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("Start → earlier · End ↓ later");
            egui::Grid::new("play-scope")
                .spacing(egui::vec2(8.0, 8.0))
                .show(ui, |ui| {
                    ui.label("End ↓ / Start →");
                    for heading in [
                        "Current position / note",
                        "Beat / measure start",
                        "Song start",
                    ] {
                        ui.strong(heading);
                    }
                    ui.end_row();
                    type Actions = &'static [(Mode, &'static str)];
                    let rows: [(&str, [Actions; 3]); 4] = [
                        (
                            "Note end",
                            [
                                &[
                                    (Mode::Event, "e — Single note"),
                                    (Mode::Notes, "n — Same-tick notes"),
                                ],
                                &[],
                                &[],
                            ],
                        ),
                        (
                            "Beat end",
                            [&[], &[(Mode::BeatOnly, "i — Current beat")], &[]],
                        ),
                        (
                            "Measure end",
                            [&[], &[(Mode::Measure, "m — Current measure")], &[]],
                        ),
                        (
                            "Track / song end",
                            [
                                &[(Mode::Position, "p — Current MBT to end")],
                                &[
                                    (Mode::Beat, "b — Beat start to end"),
                                    (Mode::MeasureStart, "s — Measure start to end"),
                                    (
                                        Mode::AllTracksFromMeasure,
                                        "c — All tracks from current measure to end",
                                    ),
                                ],
                                &[
                                    (Mode::Track, "t — Entire current track"),
                                    (Mode::AllTracks, "a — All tracks from start"),
                                ],
                            ],
                        ),
                    ];
                    for (heading, cells) in rows {
                        ui.strong(heading);
                        for actions in cells {
                            scope_cell(ui, actions, empty_track, &mut chosen);
                        }
                        ui.end_row();
                    }
                });
            if ui.button("d — Stop").clicked() {
                stopped = true;
            }
            ui.label("Esc — Close · Space — Repeat last method");
        });
    app.playback.open = open;
    if stopped {
        crate::play::transport::stop(app);
        app.playback.open = false;
    } else if let Some(mode) = chosen {
        start(app, mode);
    }
}

pub fn all_tracks(mode: Mode) -> bool {
    matches!(mode, Mode::AllTracks | Mode::AllTracksFromMeasure)
}

fn current_track_is_empty(app: &App) -> bool {
    app.loaded
        .as_ref()
        .is_some_and(|loaded| !loaded.music.tracks[app.track].events.iter().any(note_on))
}

fn scope_cell(
    ui: &mut egui::Ui,
    actions: &[(Mode, &str)],
    empty_track: bool,
    chosen: &mut Option<Mode>,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(192.0);
        ui.set_min_size(egui::vec2(192.0, 60.0));
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            for &(mode, label) in actions {
                let button = if all_tracks(mode) || empty_track {
                    let selection = ui.visuals().selection;
                    egui::Button::new(egui::RichText::new(label).color(selection.stroke.color))
                        .fill(selection.bg_fill)
                } else {
                    egui::Button::new(label)
                };
                if ui.add(button).clicked() {
                    *chosen = Some(mode);
                }
            }
        });
    });
}

#[cfg(test)]
mod empty_track_tests;
#[cfg(test)]
mod tests;
