//! Pattern-library operations share the suspended searchable picker.
use super::*;
use crate::view::patterns::{self, Action};
use cat_smf_editor::score::pattern::{operations, Catalog};
fn midi_baseline(query: &str) -> Result<(u8, Option<u8>, u16), String> {
    let fields = query.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 3 {
        return Err("Expected channel, CC number or bend, value".into());
    }
    let channel = fields[0]
        .parse::<u8>()
        .ok()
        .filter(|c| (1..=16).contains(c))
        .ok_or("Channel must be 1–16")?
        - 1;
    let controller = if fields[1].eq_ignore_ascii_case("bend") {
        None
    } else {
        Some(
            fields[1]
                .parse::<u8>()
                .ok()
                .filter(|&cc| cat_smf_editor::score::pattern::automation::supported_cc(cc))
                .ok_or("Unsupported CC baseline")?,
        )
    };
    let value = fields[2]
        .parse::<u16>()
        .ok()
        .filter(|&v| v <= if controller.is_some() { 127 } else { 16383 })
        .ok_or("Invalid MIDI baseline value")?;
    Ok((channel, controller, value))
}
pub(super) struct State {
    action: Action,
    pattern: Option<u64>,
    sound: bool,
    structure: bool,
    tempo: bool,
    midi: bool,
    song: u64,
    query: String,
    search: commands::SearchQuery,
    selected: usize,
    composing: bool,
    error: Option<String>,
}
impl State {
    fn new(a: &App, action: Action) -> Result<Self, String> {
        Catalog::read(&a.loaded.as_ref().ok_or("No SMF is loaded")?.music)?;
        Ok(Self {
            action,
            pattern: None,
            sound: false,
            structure: false,
            tempo: action == Action::AuditionTempo,
            midi: false,
            song: a.song_id,
            query: if action == Action::AuditionTempo {
                a.patterns.tempo.0.to_string()
            } else {
                String::new()
            },
            search: Default::default(),
            selected: 0,
            composing: false,
            error: None,
        })
    }
    fn choices(&self, a: &App) -> Result<Vec<(Option<u64>, String)>, String> {
        let m = &a.loaded.as_ref().ok_or("No SMF is loaded")?.music;
        let c = Catalog::read(m)?;
        if self.sound {
            let out = c
                .tracks
                .iter()
                .filter(|(_, t)| c.sources.contains(t))
                .map(|(&id, &t)| {
                    (
                        Some(id),
                        m.tracks[t]
                            .name()
                            .unwrap_or_else(|| format!("Container {id}")),
                    )
                })
                .collect();
            Ok(out)
        } else {
            let mut p = c.patterns.values().collect::<Vec<_>>();
            p.sort_by_key(|p| (p.track, p.tick));
            Ok(p.into_iter()
                .map(|p| (Some(p.id), p.name.clone()))
                .collect())
        }
    }
    fn apply(&mut self, a: &mut App, choice: Option<u64>) -> Result<bool, String> {
        if self.song != a.song_id {
            return Err("Pattern input target changed".into());
        }
        if self.tempo {
            let bpm = self
                .query
                .trim()
                .parse::<f64>()
                .map_err(|_| "Invalid audition tempo")?;
            cat_smf_editor::score::audition_timing::TempoMap::constant(
                a.loaded.as_ref().ok_or("No SMF is loaded")?.music.division,
                bpm,
            )?;
            a.patterns.tempo.0 = bpm;
            return Ok(true);
        }
        if self.midi {
            let id = self.pattern.ok_or("Missing pattern")?;
            let (channel, controller, value) = midi_baseline(&self.query)?;
            patterns::workspace::mutate(a, |m| {
                let c = Catalog::read(m)?;
                let owner = c.patterns.get(&id).ok_or("Missing pattern")?.container;
                cat_smf_editor::score::pattern::automation::set_midi_baseline(
                    m, owner, channel, controller, value,
                )
            })?;
            return Ok(true);
        }
        if self.structure {
            let id = self.pattern.ok_or("Missing pattern")?;
            let division = a.loaded.as_ref().ok_or("No SMF is loaded")?.music.division;
            let meters = cat_smf_editor::score::pattern::body::parse_meters(&self.query, division)?;
            patterns::workspace::mutate(a, |m| operations::resize(m, id, meters))?;
            return Ok(true);
        }
        if self.sound {
            patterns::membership::move_to(
                a,
                self.pattern.ok_or("Missing pattern")?,
                choice.ok_or("Select a container")?,
            )?;
            a.patterns.selected = self.pattern;
            return Ok(true);
        }
        let id = choice.ok_or("Select a pattern")?;
        match self.action {
            Action::Create | Action::AuditionTempo => return Err("Invalid create input".into()),
            Action::Duplicate => patterns::library::create(a, Some(id))?,
            Action::Preview => {
                patterns::preview::start(a, id)?;
                a.patterns.selected = Some(id);
            }
            Action::Delete => {
                patterns::workspace::mutate(a, |m| operations::delete(m, id))?;
                a.patterns.selected = None;
            }
            Action::Structure => {
                let c = Catalog::read(&a.loaded.as_ref().ok_or("No SMF is loaded")?.music)?;
                self.query = c
                    .patterns
                    .get(&id)
                    .ok_or("Missing pattern")?
                    .meters
                    .iter()
                    .map(|m| format!("{}/{}", m.numerator, 1u64 << m.exponent))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.pattern = Some(id);
                self.structure = true;
                return Ok(false);
            }
            Action::MidiBaseline => {
                self.pattern = Some(id);
                self.midi = true;
                self.query.clear();
                self.error = None;
                return Ok(false);
            }
            Action::MoveContainer => {
                self.pattern = Some(id);
                self.sound = true;
                self.query.clear();
                self.selected = 0;
                self.error = None;
                return Ok(false);
            }
        }
        Ok(true)
    }
}
pub(super) fn choose(
    a: &mut App,
    ctx: &egui::Context,
    palette: &mut super::State,
    id: &str,
) -> bool {
    let Some(action) = Action::from_id(id) else {
        return false;
    };
    if action == Action::Create {
        patterns::library::add(a, ctx);
        palette.origin = None;
        palette.commands.clear();
    } else {
        match State::new(a, action) {
            Ok(state) => palette.pattern_args = Some(state),
            Err(e) => a.load_error = Some(e),
        }
    }
    super::clear(ctx);
    ctx.request_repaint();
    true
}
pub(super) fn handle(
    a: &mut App,
    ctx: &egui::Context,
    palette: &mut super::State,
    fresh: bool,
    invalid: bool,
) -> bool {
    let Some(state) = &mut palette.pattern_args else {
        return false;
    };
    if invalid || show(a, ctx, state, fresh) {
        if invalid {
            crate::play::transport::stop(a);
            a.load_error = Some("Pattern target changed".into());
        }
        if let Some(origin) = &palette.origin {
            origin.restore_focus(ctx);
        }
        palette.origin = None;
        palette.pattern_args = None;
        palette.commands.clear();
    }
    true
}
fn show(a: &mut App, ctx: &egui::Context, s: &mut State, fresh: bool) -> bool {
    let events = ctx.input(|i| i.events.clone());
    let ime = s.composing || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
    let text = events.iter().any(|e| {
        matches!(
            e,
            egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Ime(_)
        )
    });
    s.search.update(&s.query);
    let all = match s.choices(a) {
        Ok(c) => c,
        Err(e) => {
            s.error = Some(e);
            vec![]
        }
    };
    let mut choices = all
        .iter()
        .filter(|(_, label)| s.search.matches_text(label))
        .collect::<Vec<_>>();
    let mut enter = false;
    let mut cancel = false;
    if fresh {
        for e in &events {
            if let egui::Event::Ime(e) = e {
                match e {
                    egui::ImeEvent::Preedit(t) => s.composing = !t.is_empty(),
                    egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => s.composing = false,
                    _ => {}
                }
            }
            if !ime
                && let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } = e
            {
                if let Some(m) = crate::view::list_navigation::movement(*key, *modifiers) {
                    s.selected = m.apply(s.selected, choices.len());
                }
                if commands::matches(*modifiers, Modifiers::NONE) && !repeat {
                    enter |= *key == Key::Enter && !text;
                    cancel |= *key == Key::Escape;
                }
            }
        }
    }
    if cancel {
        crate::play::transport::stop(a);
        return true;
    }
    if enter && (s.structure || s.tempo || s.midi) {
        match s.apply(a, s.pattern) {
            Ok(true) => return true,
            Err(e) => s.error = Some(e),
            _ => (),
        }
    }
    if enter
        && !(s.structure || s.tempo || s.midi)
        && let Some((id, _)) = choices.get(s.selected)
    {
        match s.apply(a, *id) {
            Ok(true) => return true,
            Ok(false) => {
                super::clear(ctx);
                ctx.request_repaint();
                return false;
            }
            Err(e) => s.error = Some(e),
        }
    }
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            ime || !matches!(
                e,
                egui::Event::Key {
                    key: Key::Enter
                        | Key::Escape
                        | Key::ArrowDown
                        | Key::ArrowUp
                        | Key::PageDown
                        | Key::PageUp,
                    ..
                }
            )
        })
    });
    if !fresh {
        super::clear(ctx);
    }
    let mut clicked = None;
    egui::Modal::new(egui::Id::new("pattern-argument")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading(s.action.label());
        ui.label(if s.tempo {
            "Audition BPM"
        } else if s.midi {
            "Channel · CC / bend · value"
        } else if s.structure {
            "Measures (4/4 3/4)"
        } else if s.sound {
            "Container"
        } else {
            "Pattern"
        });
        let id = egui::Id::new("pattern-argument-query");
        ctx.memory_mut(|m| m.request_focus(id));
        let r = ui.add(egui::TextEdit::singleline(&mut s.query).id(id));
        r.request_focus();
        if r.changed() {
            s.search.update(&s.query);
            s.selected = 0;
            s.error = None;
            choices = all
                .iter()
                .filter(|(_, label)| s.search.matches_text(label))
                .collect();
        }
        s.selected = s.selected.min(choices.len().saturating_sub(1));
        egui::ScrollArea::vertical()
            .drag_to_scroll(false)
            .max_height(360.0)
            .show(ui, |ui| {
                for (i, (id, label)) in choices
                    .iter()
                    .enumerate()
                    .filter(|_| !(s.structure || s.tempo || s.midi))
                {
                    if ui.selectable_label(i == s.selected, label).clicked() {
                        clicked = Some(*id);
                    }
                }
            });
        if let Some(e) = &s.error {
            ui.colored_label(egui::Color32::LIGHT_RED, e);
        }
    });
    if let Some(choice) = clicked {
        match s.apply(a, choice) {
            Ok(done) => return done,
            Err(e) => s.error = Some(e),
        }
    }
    false
}
#[cfg(test)]
mod tests;
