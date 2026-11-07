use super::{preparing, tick, App};
use crate::play::playback::Mode;
use cat_smf_editor::score::{audition_timing::TempoMap, pattern, scene::arrangement};
use eframe::egui;
use std::sync::Arc;

/// The song, its edit, the view and the conductor origin of the shown pattern.
type Version = (u64, u64, bool, Option<u64>, u64);

#[derive(Clone)]
struct Timing {
    version: Version,
    tempos: Arc<Vec<TempoMap>>,
    total: f64,
}

fn shown(app: &App) -> Option<u64> {
    let context = app.patterns.editing.as_ref()?;
    context.standalone.then_some(context.pattern)
}

/// The length of what plays: the running Song or Scene, else the pattern playing or shown, else
/// the file. Raw counts the ticks a container stores; Notes counts the Song and the plain tracks,
/// because those stored ticks are no time in the music.
fn total(app: &App, tempos: &[TempoMap], mode: Option<Mode>) -> Option<f64> {
    let loaded = app.loaded.as_ref()?;
    let tempo = tempos.first()?;
    let song = |scene| {
        let (catalog, collection) = loaded.arrangement.as_ref()?;
        let (_, end) =
            arrangement::occurrences_in(&loaded.music, catalog, collection, scene).ok()?;
        Some(tempo.duration(0, end))
    };
    let pattern = match mode {
        Some(Mode::Arrangement(scene)) => return song(scene),
        Some(Mode::Pattern(id)) => Some(id),
        _ => shown(app),
    };
    if let Some(id) = pattern {
        let (catalog, _) = loaded.arrangement.as_ref()?;
        return Some(tempo.duration(0, catalog.patterns.get(&id)?.length().ok()?));
    }
    let tracks = loaded
        .music
        .tracks
        .iter()
        .enumerate()
        .zip(tempos)
        .filter(|((index, _), _)| !app.note_mode || !pattern::is_source(&loaded.music, *index))
        .map(|((_, track), tempo)| tempo.duration(0, track.events.last().map_or(0, |e| e.tick)))
        .fold(0.0, f64::max);
    Some(song(None).map_or(tracks, |song| song.max(tracks)))
}

fn stopped_timing(app: &App, ctx: &egui::Context) -> Option<Timing> {
    let loaded = app.loaded.as_ref()?;
    let version = (
        app.song_id,
        app.generation,
        app.note_mode,
        shown(app),
        shown(app)
            .and_then(|id| crate::view::patterns::timing::pattern_tick(app, id).ok())
            .unwrap_or(0),
    );
    let id = egui::Id::new("transport-timing");
    if let Some(cached) = ctx.data_mut(|d| d.get_temp::<Timing>(id))
        && cached.version == version
    {
        return Some(cached);
    }
    let preview = shown(app).and_then(|id| crate::view::patterns::timing::pattern(app, id).ok());
    let tempos: Vec<_> = (0..loaded.music.tracks.len())
        .map(|track| {
            preview
                .clone()
                .unwrap_or_else(|| TempoMap::new(&loaded.music, track))
        })
        .collect();
    let total = total(app, &tempos, None)?;
    let timing = Timing {
        version,
        tempos: Arc::new(tempos),
        total,
    };
    ctx.data_mut(|d| d.insert_temp(id, timing.clone()));
    Some(timing)
}

fn measure_width(app: &App, ctx: &egui::Context) -> usize {
    let version = (app.song_id, app.generation);
    let id = egui::Id::new("transport-measure-width");
    if let Some((cached, width)) = ctx.data_mut(|d| d.get_temp::<((u64, u64), usize)>(id))
        && cached == version
    {
        return width;
    }
    let width = app.loaded.as_ref().map_or(1, |loaded| {
        loaded
            .music
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(track, source)| {
                let end = source.events.last().map_or(0, |event| event.tick);
                loaded.time.map(track).map(|map| map.position(end).measure)
            })
            .max()
            .unwrap_or(1)
            .to_string()
            .len()
    });
    ctx.data_mut(|d| d.insert_temp(id, (version, width)));
    width
}

fn clock(seconds: f64, hours: bool) -> String {
    let millis = (seconds.max(0.0) * 1000.0).round() as u64;
    if hours {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            millis / 3_600_000,
            millis / 60_000 % 60,
            millis / 1000 % 60,
            millis % 1000
        )
    } else {
        format!(
            "{:02}:{:02}.{:03}",
            millis / 60_000,
            millis / 1000 % 60,
            millis % 1000
        )
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if preparing(app) {
            ui.spinner();
            ui.colored_label(egui::Color32::LIGHT_GRAY, "Preparing playback…");
        } else {
            let playing = tick(app);
            let tick = playing.unwrap_or_else(|| super::playback::location(app).0);
            let position = app
                .loaded
                .as_ref()
                .and_then(|l| l.time.map(app.track))
                .map(|m| m.position(tick));
            let tick_text = if app.note_mode {
                String::new()
            } else {
                format!(" · Tick {tick}")
            };
            let timing = playing
                .is_none()
                .then(|| stopped_timing(app, ui.ctx()))
                .flatten();
            let seconds =
                if let Some(active) = app.playback.active.as_ref().filter(|_| playing.is_some()) {
                    active.start_seconds + app.audition.seconds().unwrap_or(0.0)
                } else {
                    timing
                        .as_ref()
                        .and_then(|t| t.tempos.get(app.track))
                        .map_or(0.0, |tempo| tempo.duration(0, tick))
                };
            let total_seconds = app
                .playback
                .active
                .as_ref()
                .and_then(|active| total(app, &active.tempos, Some(active.mode)))
                .or_else(|| timing.as_ref().map(|t| t.total))
                .filter(|seconds| seconds.is_finite());
            let hours = total_seconds.unwrap_or(seconds) >= 3600.0;
            let total = total_seconds
                .map(|total| format!(" / {}", clock(total, hours)))
                .unwrap_or_default();
            let measure_width = measure_width(app, ui.ctx());
            let color = egui::Color32::LIGHT_GRAY;
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            let prefix = if playing.is_some() {
                "▶ Playing"
            } else {
                "Stopped"
            };
            let width = |text: &str| {
                ui.fonts(|fonts| {
                    fonts
                        .layout_no_wrap(text.into(), font.clone(), color)
                        .size()
                        .x
                })
            };
            let mut gap = width("▶ Playing").max(width("Stopped")) - width(prefix);
            let mut job = egui::text::LayoutJob::default();
            job.append(
                prefix,
                0.0,
                egui::TextFormat::simple(
                    font.clone(),
                    if playing.is_some() {
                        egui::Color32::LIGHT_GREEN
                    } else {
                        color
                    },
                ),
            );
            let mut append = |text: &str, color| {
                job.append(
                    text,
                    std::mem::take(&mut gap),
                    egui::TextFormat::simple(font.clone(), color),
                );
            };
            append(&format!(" {}{total} · ", clock(seconds, hours)), color);
            if let Some(position) = position {
                append(&format!("{:>measure_width$}|", position.measure), color);
                append(
                    &position.beat.to_string(),
                    crate::view::beat_color(position.beat),
                );
                append(
                    &format!(
                        "|{:>3}",
                        position.tick_numerator / position.tick_denominator
                    ),
                    color,
                );
            } else {
                append("—", color);
            }
            append(&tick_text, color);
            ui.label(job).on_hover_text(
                "p / Space  再生menu / 前回の再生
Shift+Space  演奏停止",
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("☰").clicked() {
                crate::view::comma_menu::open(app);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{clock, stopped_timing, total, App, Mode, TempoMap};
    use crate::view::patterns::{self, workspace_tests::verification_song};
    use cat_smf_editor::score::{pattern::operations, scene::arrangement};
    use eframe::egui;

    /// A Song stopped on its container, whose stored bodies run far past the Song's end.
    fn song() -> (App, egui::Context, u64, f64, f64) {
        let (mut a, c, first, _, _) = verification_song();
        crate::play::transport::stop(&mut a);
        let source = a.track;
        a.edit_tracks(|m| {
            for _ in 0..40 {
                operations::create(m, source, None)?;
            }
            Ok(())
        })
        .unwrap();
        patterns::refresh(&mut a);
        let music = &a.loaded.as_ref().unwrap().music;
        let tempo = TempoMap::new(music, 0);
        let (_, end) = arrangement::occurrences(music, None).unwrap();
        let stored = music.tracks[source].events.last().unwrap().tick;
        let (song, stored) = (tempo.duration(0, end), tempo.duration(0, stored));
        assert!(song > 0.0 && song * 2.0 < stored, "{song} {stored}");
        (a, c, first, song, stored)
    }
    fn tempos(a: &App) -> Vec<TempoMap> {
        let music = &a.loaded.as_ref().unwrap().music;
        (0..music.tracks.len())
            .map(|track| TempoMap::new(music, track))
            .collect()
    }

    #[test]
    fn a_song_lasts_to_its_end_not_to_the_ticks_its_containers_store() {
        let (a, _, _, song, _) = song();
        let tempos = tempos(&a);
        assert_eq!(
            total(&a, &tempos, Some(Mode::Arrangement(None))),
            Some(song)
        );
    }

    #[test]
    fn only_raw_counts_the_ticks_a_container_stores() {
        let (mut a, c, _, song, stored) = song();
        a.patterns.editing = None;
        a.track = 0;
        assert_eq!(stopped_timing(&a, &c).unwrap().total, song);
        a.note_mode = false;
        assert_eq!(stopped_timing(&a, &c).unwrap().total, stored);
    }

    #[test]
    fn a_pattern_duration_cache_follows_the_song_cursor_and_conductor() {
        let (mut a, c, first, _, _) = song();
        patterns::library::edit(&mut a, first).unwrap();
        a.patterns.workspace.song_cell.measure = 1;
        let length = a
            .loaded
            .as_ref()
            .unwrap()
            .arrangement
            .as_ref()
            .unwrap()
            .0
            .patterns[&first]
            .length()
            .unwrap();
        let seconds = length as f64 / 960.0;
        assert_eq!(stopped_timing(&a, &c).unwrap().total, seconds);
        a.edit_tracks(|m| {
            cat_smf_editor::edit::insert(
                &mut m.tracks[0],
                cat_smf_editor::model::Event {
                    tick: 1920,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x51,
                    data: vec![15, 66, 64],
                },
            );
            Ok(())
        })
        .unwrap();
        a.patterns.workspace.song_cell.measure = 2;
        let slower = stopped_timing(&a, &c).unwrap();
        assert_eq!(slower.total, seconds * 2.0);
        assert_eq!(
            total(&a, &slower.tempos, Some(Mode::Pattern(first))),
            Some(seconds * 2.0)
        );
    }

    #[test]
    fn time_format_handles_millisecond_second_minute_and_hour_carries() {
        assert_eq!(clock(0.0, false), "00:00.000");
        assert_eq!(clock(0.001, false), "00:00.001");
        assert_eq!(clock(59.9996, false), "01:00.000");
        assert_eq!(clock(3599.999, false), "59:59.999");
        assert_eq!(clock(3599.9996, false), "60:00.000");
        assert_eq!(clock(0.0, true), "00:00:00.000");
        assert_eq!(clock(3599.9996, true), "01:00:00.000");
        assert_eq!(clock(359999.999, true), "99:59:59.999");
    }
}
