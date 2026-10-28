use crate::{app::App, play::audio_sequence::Sequence};
use cat_smf_editor::{
    score::display_rows::DisplayRow, score::musical_time::Position, score::note_view::note_on,
};
use eframe::egui;
mod sequence;
mod sounding;
use sequence::sequence_track;
pub(crate) use sounding::{expand_all, performance, silent, sounding, Expansion, Sounding};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    Pattern(u64),
    Arrangement(Option<u64>),
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
    pub parameter_preview: Option<crate::view::parameter_editor::preview::Preview>,
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
            parameter_preview: None,
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
    let tick = source.map_or(0, |i| loaded.visible().tracks[app.track].events[i].tick);
    (tick, map.map(|m| m.position(tick)))
}

#[cfg(test)]
pub fn sequence(app: &App, mode: Mode) -> Option<Sequence> {
    sequence_track(app, app.track, mode)
}

pub fn start(app: &mut App, mode: Mode) {
    let pattern = if let Mode::Pattern(id) = mode {
        Some(id)
    } else {
        app.patterns
            .editing
            .as_ref()
            .filter(|c| c.standalone)
            .map(|c| c.pattern)
    };
    if let Some(id) = pattern {
        if matches!(
            mode,
            Mode::Pattern(_) | Mode::Track | Mode::AllTracks | Mode::AllTracksFromMeasure
        ) {
            app.playback.open = false;
            if let Err(error) = crate::view::patterns::preview::start(app, id) {
                app.load_error = Some(error);
            }
            return;
        }
        if let Err(error) = crate::view::patterns::preview::prepare(app, id) {
            app.load_error = Some(error);
            return;
        }
    }
    if let Mode::Arrangement(scene) = mode {
        super::arrangement::start(app, scene);
        return;
    }
    app.playback.repeat = None;
    app.playback.last = mode;
    app.playback.open = false;
    app.audition.stop();
    app.playback.active = None;
    if let Some(loaded) = &app.loaded
        && let Err(error) = cat_smf_editor::score::same_measure::validate(&loaded.music)
    {
        app.playback.pending = None;
        app.load_error = Some(error);
        return;
    }
    app.playback.pending = crate::play::transport::prepare(app, mode);
}

pub fn parts(
    app: &App,
    mode: Mode,
) -> Vec<(usize, cat_smf_editor::score::instrument::Patch, Sequence)> {
    let Some(loaded) = &app.loaded else {
        return Vec::new();
    };
    if let Some(context) = &app.patterns.editing
        && context.standalone
    {
        if matches!(
            mode,
            Mode::Pattern(_) | Mode::Track | Mode::AllTracks | Mode::AllTracksFromMeasure
        ) {
            return crate::view::patterns::preview::prepare(app, context.pattern)
                .map(|p| p.parts)
                .unwrap_or_default();
        }
        let patch = match crate::view::patterns::preview::prepare(app, context.pattern) {
            Ok(p) => p.parts[0].1.clone(),
            Err(_) => return vec![],
        };
        return sequence_track(app, context.track, mode)
            .map(|s| vec![(context.track, patch, s)])
            .unwrap_or_default();
    }
    let curves = cat_smf_editor::edit::parameter_curve::read(&loaded.music).ok();
    let automation_owner = curves.as_ref().and_then(|c| {
        c.curves
            .contains_key(&app.track)
            .then(|| c.owner(app.track))
    });
    let tracks = if mode == Mode::AllTracks {
        (0..loaded.music.tracks.len()).collect()
    } else if let Some(owner) = automation_owner {
        owner.into_iter().collect()
    } else if current_track_is_empty(app) {
        (0..loaded.music.tracks.len()).collect()
    } else {
        vec![app.track]
    };
    tracks
        .into_iter()
        .filter(|&track| {
            !cat_smf_editor::score::pattern::is_source(&loaded.music, track)
                && !loaded.music.tracks[track]
                    .events
                    .iter()
                    .any(cat_smf_editor::edit::parameter_curve::owned)
        })
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
    crate::view::commands::route(app, ctx);
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
    app.loaded.as_ref().is_some_and(|loaded| {
        !loaded.music.tracks[app.track]
            .events
            .iter()
            .any(cat_smf_editor::edit::parameter_curve::owned)
            && !loaded.visible().tracks[app.track]
                .events
                .iter()
                .any(|e| note_on(e) || cat_smf_editor::score::same_measure::owned(e))
    })
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
mod same_measure_tests;
#[cfg(test)]
mod tests;
