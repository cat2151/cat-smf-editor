//! Explicit note events, interleaved with automation at their source times.
use super::{Effect, Instrument, Render};
use crate::play::automation::{LayerBindings, RuntimePlan};
use cat_smf_editor::host::parameters::{AutomationEvent, MAX_AUTOMATION_EVENTS};

fn push(events: &mut Vec<AutomationEvent>, event: AutomationEvent) -> Result<(), i32> {
    if events.len() == events.capacity() || events.len() == MAX_AUTOMATION_EVENTS {
        return Err(-7);
    }
    events.push(event);
    Ok(())
}
fn values(
    events: &mut Vec<AutomationEvent>,
    binding: Option<&LayerBindings>,
    tick: f64,
) -> Result<(), i32> {
    if let Some(binding) = binding {
        for lane in &binding.lanes {
            let value = lane.at(tick).ok_or(-3)?;
            push(
                events,
                AutomationEvent::parameter(lane.value.with_value(value)),
            )?;
        }
    }
    Ok(())
}

impl<R: Instrument, E: Effect> Render<R, E> {
    pub fn process_live(&mut self, words: &[u32], out: &mut [f32], channels: usize) -> bool {
        self.event_block(
            words.iter().map(|&word| (0.0, Some(word))),
            None,
            out,
            channels,
        )
    }

    pub fn process_scheduled(
        &mut self,
        timeline: &crate::play::event_timeline::Timeline,
        plan: Option<&RuntimePlan>,
        frame: u64,
        out: &mut [f32],
        channels: usize,
    ) -> bool {
        if let Some(plan) = plan {
            if self.automation.iter().map(|b| b.lanes.len()).sum::<usize>()
                != plan.owner.curves.len()
            {
                self.parameter_error = Some(-6);
                return false;
            }
            self.event_block(
                plan.events_at(frame)
                    .iter()
                    .map(|event| (event.tick, event.midi)),
                plan.is_grid(frame).then(|| plan.sample_tick(frame)),
                out,
                channels,
            )
        } else {
            self.event_block(
                timeline
                    .events_at(frame)
                    .iter()
                    .map(|event| (event.seconds, Some(event.word))),
                None,
                out,
                channels,
            )
        }
    }

    fn event_block(
        &mut self,
        marks: impl Iterator<Item = (f64, Option<u32>)> + Clone,
        grid: Option<f64>,
        out: &mut [f32],
        channels: usize,
    ) -> bool {
        if channels == 0 || out.len() > self.scratch.len() || self.sung.active() {
            self.parameter_error = Some(-3);
            return false;
        }
        out.fill(0.0);
        let solo = self.layers.iter().any(|(_, layer)| layer.solo);
        for (index, (render, layer)) in self.layers.iter_mut().enumerate() {
            self.ordered_events.clear();
            let binding = self.automation.iter().find(|b| b.layer == index);
            let mut grid = grid;
            let mut marks = marks.clone().peekable();
            let build = (|| -> Result<(), i32> {
                while marks.peek().is_some() || grid.is_some() {
                    let tick = match (marks.peek(), grid) {
                        (Some(event), Some(grid)) => event.0.min(grid),
                        (Some(event), None) => event.0,
                        (None, Some(grid)) => grid,
                        (None, None) => break,
                    };
                    if grid == Some(tick) {
                        grid = None;
                    }
                    values(&mut self.ordered_events, binding, tick)?;
                    while let Some((_, midi)) = marks.next_if(|e| e.0 == tick) {
                        if let Some(word) = midi {
                            if !matches!((word >> 20) & 15, 8 | 9) {
                                push(&mut self.ordered_events, AutomationEvent::note(word))?;
                                continue;
                            }
                            if let Some(pitch) = super::routed_pitch(
                                layer,
                                self.drums.as_ref(),
                                ((word >> 8) & 127) as u8,
                            ) {
                                push(
                                    &mut self.ordered_events,
                                    AutomationEvent::note(
                                        (word & !0x7f00) | (u32::from(pitch) << 8),
                                    ),
                                )?;
                            }
                        }
                    }
                }
                Ok(())
            })();
            if let Err(code) = build {
                self.parameter_error = Some(code);
                return false;
            }
            let scratch = &mut self.scratch[..out.len()];
            if let Err(code) = render.ordered(&self.ordered_events, scratch, channels) {
                self.parameter_error = Some(code);
                return false;
            }
            let Ok(gain) = layer.gain() else {
                return false;
            };
            if layer.mute || (solo && !layer.solo) {
                continue;
            }
            for (sum, sample) in out.iter_mut().zip(scratch) {
                *sum += *sample * gain;
            }
        }
        if !out.iter().all(|v| v.is_finite())
            || self
                .effects
                .as_mut()
                .is_some_and(|effect| !effect.apply(out, channels))
            || !out.iter().all(|v| v.is_finite())
        {
            out.fill(0.0);
            return false;
        }
        true
    }
}
