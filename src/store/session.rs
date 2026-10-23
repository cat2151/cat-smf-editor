use crate::{app::App, store::config::Session};
use cat_smf_editor::{score::display_rows::DisplayRow, score::musical_time::Position};
use std::cmp::Ordering;

#[cfg(test)]
#[path = "session/startup_cache_tests.rs"]
mod startup_cache_tests;

#[cfg(test)]
#[path = "session/local_cache_tests.rs"]
mod local_cache_tests;

pub(crate) fn parse_mbt(text: &str) -> Option<Position> {
    let mut parts = text.split(':');
    let measure = parts.next()?.parse().ok()?;
    let beat = parts.next()?.parse().ok()?;
    let mut fraction = parts.next()?.split('/');
    let tick_numerator = fraction.next()?.parse().ok()?;
    let tick_denominator = fraction.next().map_or(Some(1), |s| s.parse().ok())?;
    if parts.next().is_some()
        || fraction.next().is_some()
        || measure == 0
        || beat == 0
        || tick_numerator > 262_140
        || tick_denominator == 0
        || tick_denominator > (1u128 << 63)
    {
        return None;
    }
    Some(Position {
        measure,
        beat,
        tick_numerator,
        tick_denominator,
    })
}

fn compare(a: &Position, b: &Position) -> Ordering {
    (a.measure, a.beat).cmp(&(b.measure, b.beat)).then_with(|| {
        (a.tick_numerator * b.tick_denominator).cmp(&(b.tick_numerator * a.tick_denominator))
    })
}

impl App {
    fn row_location(&self, row: usize) -> (Option<Position>, Option<u64>, Option<usize>) {
        let loaded = self.loaded.as_ref().unwrap();
        if let Some(position) = &self.cursor_position {
            return (
                Some(position.clone()),
                loaded
                    .time
                    .map(self.track)
                    .and_then(|m| m.tick_at(position)),
                None,
            );
        }
        let source = if self.note_mode {
            match loaded.display_rows[self.track].get(row) {
                DisplayRow::Beat(position) => return (Some(position), None, None),
                DisplayRow::Event(i) => i,
            }
        } else {
            row
        };
        let tick = loaded.music.tracks[self.track].events[source].tick;
        (
            loaded.time.map(self.track).map(|m| m.position(tick)),
            Some(tick),
            Some(source),
        )
    }

    pub fn session(&self) -> Option<Session> {
        let loaded = self.loaded.as_ref()?;
        let (mbt, tick) = self.cursor_row.map_or((None, None), |row| {
            let (position, tick, _) = self.row_location(row);
            (position.map(|p| p.to_string()), tick)
        });
        Some(Session {
            file: loaded.name.clone(),
            track: self.track + 1,
            mbt,
            tick,
            row: self.cursor_row,
            note_mode: self.note_mode,
            split: self.track_view.split.map(|tracks| tracks.map(|t| t + 1)),
            split_active: self.track_view.active,
        })
    }

    pub fn save_session(&mut self) {
        if self.startup.pending {
            return;
        }
        if !self.saved.is_untitled()
            && let Some(session) = self.session()
        {
            self.config.save(session);
        }
    }

    pub fn open_initial(&mut self, mid: Option<std::path::PathBuf>) {
        match mid {
            Some(path) => self.open_mid(path),
            None => match self.config.navigation.last_mid.clone() {
                None => {
                    self.install_new_song();
                }
                Some(path) if !path.exists() => self.missing_last_mid = Some(path),
                Some(_) => self.restore_session(),
            },
        }
        self.file_open_requested = self.loaded.is_none() && self.missing_last_mid.is_none();
    }

    pub fn restore_session(&mut self) {
        let Some(path) = self.config.navigation.last_mid.clone() else {
            return;
        };
        let session = self.config.session.clone();
        self.open_mid(path);
        if self.load_error.is_none()
            && let Some(session) = session
        {
            self.restore_position(&session);
        }
    }

    pub fn restore_position(&mut self, session: &Session) {
        let Some(loaded) = &self.loaded else {
            return;
        };
        if loaded.music.tracks.is_empty() {
            return;
        }
        self.select_track(
            session
                .track
                .saturating_sub(1)
                .min(loaded.music.tracks.len() - 1),
        );
        let tracks = self.loaded.as_ref().unwrap().music.tracks.len();
        let active = session.split_active.min(1);
        self.track_view.split = session
            .split
            .map(|split| split.map(|t| t.wrapping_sub(1)))
            .filter(|s| s.iter().all(|&t| t < tracks) && s[active] == self.track);
        self.track_view.active = if self.track_view.split.is_some() {
            active
        } else {
            0
        };
        self.note_mode = session.note_mode;
        let loaded = self.loaded.as_ref().unwrap();
        let count = if self.note_mode {
            loaded.display_rows[self.track].len()
        } else {
            loaded.music.tracks[self.track].events.len()
        };
        if count == 0 || session.row.is_none() {
            return;
        }
        let position = session.mbt.as_deref().and_then(parse_mbt);
        let matches = |row| {
            let (p, tick, _) = self.row_location(row);
            p == position && tick == session.tick
        };
        let row = if let Some(row) = session.row.filter(|&row| row < count && matches(row)) {
            row
        } else {
            // Binary search also handles compact runs with billions of empty beats.
            let mut low = 0;
            let mut high = count;
            while low < high {
                let mid = low + (high - low) / 2;
                let (p, tick, _) = self.row_location(mid);
                let ordering = match (&p, &position) {
                    (Some(a), Some(b)) => compare(a, b),
                    _ => tick.unwrap_or(0).cmp(&session.tick.unwrap_or(0)),
                };
                if ordering == Ordering::Less {
                    low = mid + 1;
                } else {
                    high = mid;
                }
            }
            low.min(count - 1)
        };
        self.cursor_row = Some(row);
        self.selected_event = self.row_location(row).2;
        if !self.note_mode
            && let Some(position) = position
        {
            let (_, actual, _) = self.row_location(row);
            if let Some(tick) = loaded
                .time
                .map(self.track)
                .and_then(|m| m.tick_at(&position))
                && actual != Some(tick)
                && tick <= loaded.music.tracks[self.track].events.last().unwrap().tick
            {
                self.cursor_position = Some(position);
                self.selected_event = None;
            }
        }
        // Restore silently and let the virtualized UI reveal the destination.
        self.reset_scroll = false;
        self.reveal_cursor = true;
    }
}

#[cfg(test)]
mod tests;
