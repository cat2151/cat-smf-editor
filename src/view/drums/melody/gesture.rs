use super::*;
use cat_smf_editor::model::Music;
use egui::{Pos2, Rect, Response, Ui};

enum Edit {
    Draw(Vec<(u8, usize)>),
    Resize { pitch: u8, anchor: usize, end: u64 },
}

pub(super) struct Gesture {
    base: Music,
    track: usize,
    number: u64,
    limit: u64,
    edit: Edit,
    last: (usize, usize),
}

pub(super) struct Grid<'a> {
    pub rect: Rect,
    pub clip: Rect,
    pub row_h: f32,
    pub rows: &'a [u8],
    pub measure: &'a drums::Measure,
    pub notes: &'a [Note],
    pub base: &'a Music,
    pub track: usize,
    pub limit: u64,
}

impl Gesture {
    pub(super) fn cursor(&self) -> (u8, usize) {
        match &self.edit {
            Edit::Resize { pitch, .. } => (*pitch, self.last.1),
            Edit::Draw(cells) => cells.last().copied().unwrap(),
        }
    }
    pub(super) fn valid(&self, a: &App) -> bool {
        self.track == a.track
            && self.number == a.drum_editor.page + 1
            && a.loaded.as_ref().is_some_and(|l| {
                cat_smf_editor::smf_io::encode(l.visible()).ok()
                    == cat_smf_editor::smf_io::encode(&self.base).ok()
            })
    }

    pub(super) fn music(&self) -> Result<Music, String> {
        let mut music = self.base.clone();
        match &self.edit {
            Edit::Resize { pitch, anchor, end } => {
                resize_to(
                    &mut music,
                    self.track,
                    self.number,
                    *pitch,
                    *anchor,
                    *end,
                    self.limit,
                )?;
            }
            Edit::Draw(cells) => {
                let mut previous = None;
                let mut cells = cells.clone();
                cells.sort_unstable();
                cells.dedup();
                for (pitch, step) in cells {
                    let time = cat_smf_editor::score::musical_time::MusicalTime::new(&music);
                    let measure = drums::Measure::new(
                        time.map(self.track).ok_or("Missing measure map")?,
                        music.division,
                        self.number,
                    )?;
                    let start = measure.steps[step];
                    let end = measure.steps.get(step + 1).copied().unwrap_or(measure.end);
                    let occupied = step_notes::notes(&music.tracks[self.track])
                        .iter()
                        .any(|n| {
                            n.pitch == pitch && n.start < end && n.end.max(n.start + 1) > start
                        });
                    if occupied {
                        previous = None;
                        continue;
                    }
                    if let Some((p, anchor, last)) = previous
                        && p == pitch
                        && last + 1 == step
                    {
                        resize_to(
                            &mut music,
                            self.track,
                            self.number,
                            pitch,
                            anchor,
                            end,
                            self.limit,
                        )?;
                        previous = Some((pitch, anchor, step));
                    } else {
                        step_notes::toggle(
                            &mut music,
                            self.track,
                            pitch,
                            self.number,
                            step,
                            VELOCITY,
                        )?;
                        previous = Some((pitch, step, step));
                    }
                }
            }
        }
        Ok(music)
    }

    pub(super) fn commit(self, a: &mut App) {
        if !self.valid(a) {
            return;
        }
        let result = self.music().and_then(|music| {
            a.edit_music(|m| {
                *m = music;
                Ok(())
            })
        });
        super::super::result(a, result);
    }
}

fn resize_to(
    music: &mut Music,
    track: usize,
    number: u64,
    pitch: u8,
    anchor: usize,
    target: u64,
    limit: u64,
) -> Result<(), String> {
    let time = cat_smf_editor::score::musical_time::MusicalTime::new(music);
    let measure = drums::Measure::new(
        time.map(track).ok_or("Missing measure map")?,
        music.division,
        number,
    )?;
    let tick = measure.steps[anchor];
    let boundary = measure
        .steps
        .get(anchor + 1)
        .copied()
        .unwrap_or(measure.end);
    let Some(note) = step_notes::notes(&music.tracks[track])
        .into_iter()
        .find(|n| n.pitch == pitch && n.start < boundary && n.end.max(n.start + 1) > tick)
    else {
        return Ok(());
    };
    let map = time.map(track).ok_or("Missing measure map")?;
    let number = u64::try_from(map.position(note.start).measure).map_err(|_| "Measure overflow")?;
    let measure = drums::Measure::new(map, music.division, number)?;
    let anchor = measure
        .steps
        .partition_point(|&tick| tick <= note.start)
        .saturating_sub(1);
    let find = |music: &Music| {
        step_notes::notes(&music.tracks[track])
            .into_iter()
            .find(|n| n.pitch == pitch && n.start == note.start)
    };
    while let Some(before) = find(music) {
        if before.end == target {
            break;
        }
        let length = if before.end < target {
            Length::Longer
        } else {
            Length::Shorter
        };
        step_notes::resize(music, track, pitch, number, anchor, length, limit)?;
        if find(music).is_none_or(|n| n.end == before.end) {
            break;
        }
    }
    Ok(())
}

impl Grid<'_> {
    fn at(&self, pos: Pos2) -> Option<(usize, usize)> {
        if !self.clip.contains(pos) || pos.y < self.rect.top() + super::widgets::HEADER {
            return None;
        }
        let row =
            ((pos.y - self.rect.top() - super::widgets::HEADER) / self.row_h).floor() as usize;
        let step = (0..self.measure.steps.len()).find(|&s| {
            let left = self.rect.left() + super::widgets::x(s);
            (left..left + super::widgets::CELL.x).contains(&pos.x)
        })?;
        (row < self.rows.len()).then_some((row, step))
    }

    pub(super) fn interact(
        &self,
        ui: &Ui,
        response: &Response,
        state: &mut State,
    ) -> Option<Gesture> {
        let completed = crate::view::timeline_drag::completed(ui, self.rect);
        let origin = completed.map(|pair| pair.0).or_else(|| {
            response
                .drag_started_by(egui::PointerButton::Primary)
                .then(|| ui.input(|i| i.pointer.press_origin()))
                .flatten()
        });
        if response.hovered()
            && let Some(pos) = ui.input(|i| i.pointer.hover_pos())
            && let Some((row, step)) = self.at(pos)
        {
            let end = self
                .measure
                .steps
                .get(step + 1)
                .copied()
                .unwrap_or(self.measure.end);
            let pitch = self.rows[row];
            if self
                .notes
                .iter()
                .any(|n| n.pitch == pitch && n.end > self.measure.steps[step] && n.end <= end)
                && pos.x
                    >= self.rect.left() + super::widgets::x(step) + super::widgets::CELL.x - 7.0
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }
        if let Some(origin) = origin
            && let Some((row, step)) = self.at(origin)
        {
            let pitch = self.rows[row];
            let start = self.measure.steps[step];
            let end = self
                .measure
                .steps
                .get(step + 1)
                .copied()
                .unwrap_or(self.measure.end);
            let covering = self
                .notes
                .iter()
                .find(|n| n.pitch == pitch && n.start < end && n.end.max(n.start + 1) > start);
            let edit = match covering {
                Some(note)
                    if note.end <= end
                        && origin.x
                            >= self.rect.left()
                                + super::widgets::x(step)
                                + super::widgets::CELL.x
                                - 7.0 =>
                {
                    Some(Edit::Resize {
                        pitch,
                        anchor: step,
                        end: note.end,
                    })
                }
                None => Some(Edit::Draw(vec![(pitch, step)])),
                _ => None,
            };
            state.gesture = edit.map(|edit| Gesture {
                base: self.base.clone(),
                track: self.track,
                number: self.measure.number,
                limit: self.limit,
                edit,
                last: (row, step),
            });
        }
        if let Some(gesture) = &mut state.gesture
            && let Some(pos) = completed
                .map(|pair| pair.1)
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
            && let Some((row, step)) = self.at(pos)
        {
            match &mut gesture.edit {
                Edit::Resize { end, .. } => {
                    *end = self
                        .measure
                        .steps
                        .get(step + 1)
                        .copied()
                        .unwrap_or(self.measure.end);
                }
                Edit::Draw(cells) => {
                    let (from_row, from_step) = gesture.last;
                    let distance = from_step.abs_diff(step);
                    for i in 1..=distance {
                        let s = if step > from_step {
                            from_step + i
                        } else {
                            from_step - i
                        };
                        let r = (from_row as f32
                            + (row as f32 - from_row as f32) * i as f32 / distance as f32)
                            .round() as usize;
                        let cell = (self.rows[r], s);
                        if !cells.contains(&cell) {
                            cells.push(cell);
                        }
                    }
                }
            }
            gesture.last = (row, step);
            state.pitch = Some(gesture.cursor().0);
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.pointer.primary_released()) {
            return state.gesture.take();
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            state.gesture = None;
        }
        None
    }
}
