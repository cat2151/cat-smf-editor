//! Device-free native delivery with the stopped-callback metadata refresh used by Voice.
use super::*;
pub(crate) fn verify(prepared: crate::play::transport::Prepared) {
    use crate::play::{automation::RuntimePlan, event_timeline::Timeline};
    for (track, patch, sequence) in prepared.parts {
        let plan = prepared.active.automation.owner(track);
        if sequence.notes.is_empty() && plan.is_none() {
            continue;
        }
        let mut native = Native::open(48000, &patch).unwrap();
        let runtime = plan.map(|owner| RuntimePlan::new(owner, &sequence, 0., 48000));
        let timeline = Timeline::new(&sequence, 0., 48000);
        let bindings = runtime
            .as_ref()
            .map(|p| native.prepare_automation(&p.owner).unwrap())
            .unwrap_or_default();
        let mut render = unsafe { native.render() };
        render.set_automation(bindings);
        let mut output = [0.; 128];
        let mut frame = 0;
        let mut peak = 0f32;
        let mut refreshes = 0;
        while frame < timeline.end {
            let count = runtime
                .as_ref()
                .map_or(64, |p| p.next(frame, 64))
                .min((timeline.end - frame) as usize);
            let mut ok = render.process_scheduled(
                &timeline,
                runtime.as_ref(),
                frame,
                &mut output[..count * 2],
                2,
            );
            if !ok && render.parameter_error == Some(-6) {
                let runtime = runtime.as_ref().unwrap();
                native.pump();
                render.set_automation(native.prepare_automation(&runtime.owner).unwrap());
                ok = render.process_scheduled(
                    &timeline,
                    Some(runtime),
                    frame,
                    &mut output[..count * 2],
                    2,
                );
                refreshes += 1;
            }
            assert!(
                ok,
                "track {track} frame {frame} parameter error {:?}",
                render.parameter_error
            );
            assert!(output[..count * 2].iter().all(|v| v.is_finite()));
            peak = output[..count * 2].iter().fold(peak, |p, v| p.max(v.abs()));
            frame += count as u64;
        }
        if !sequence.notes.is_empty() {
            assert!(peak > 0.00001, "track {track} silent");
        }
        eprintln!("track {track}: native peak={peak}, metadata refreshes={refreshes}");
    }
}
