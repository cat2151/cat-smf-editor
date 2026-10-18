use super::{preparing, tick, App};
use cat_smf_editor::score::audition_timing::TempoMap;
use eframe::egui;
use std::sync::Arc;

#[derive(Clone)]
struct Timing {
    version: (u64, u64),
    tempos: Arc<Vec<TempoMap>>,
    total: f64,
}

fn stopped_timing(app: &App, ctx: &egui::Context) -> Option<Timing> {
    let loaded = app.loaded.as_ref()?;
    let version = (app.song_id, app.generation);
    let id = egui::Id::new("transport-timing");
    if let Some(cached) = ctx.data_mut(|d| d.get_temp::<Timing>(id))
        && cached.version == version
    {
        return Some(cached);
    }
    let tempos: Vec<_> = (0..loaded.music.tracks.len())
        .map(|track| TempoMap::new(&loaded.music, track))
        .collect();
    let total = loaded
        .music
        .tracks
        .iter()
        .zip(&tempos)
        .map(|(track, tempo)| tempo.duration(0, track.events.last().map_or(0, |e| e.tick)))
        .fold(0.0, f64::max);
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
                .zip(app.loaded.as_ref())
                .map(|(active, loaded)| {
                    loaded
                        .music
                        .tracks
                        .iter()
                        .zip(&active.tempos)
                        .map(|(track, tempo)| {
                            tempo.duration(0, track.events.last().map_or(0, |e| e.tick))
                        })
                        .fold(0.0, f64::max)
                })
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
    use super::clock;

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
