//! Compare rendered handles and scale positions, then operate at those positions.
use super::*;

fn draw(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let outcome = super::super::body::show(app, ui);
                if let Some((track, gain, dragging)) = outcome.volume_change {
                    super::super::volume::change(app, track, gain, dragging).unwrap();
                }
                if let Some((target, slot)) = outcome.chain {
                    crate::view::effects::open(app, target, slot);
                }
            });
        },
    )
}

fn handles(output: &egui::FullOutput) -> Vec<egui::Pos2> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Circle(c) if c.radius > 5.0 => Some(c.center),
            _ => None,
        })
        .collect()
}

fn labels(output: &egui::FullOutput, label: &str) -> Vec<egui::Pos2> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .collect()
}

fn press(app: &mut crate::app::App, ctx: &egui::Context, pos: egui::Pos2) {
    for pressed in [true, false] {
        draw(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    draw(app, ctx, vec![]);
}

#[test]
fn effect_rows_show_priority_names_before_truncated_preset_details() {
    use cat_smf_editor::patch_reference::{PatchReference, Source};

    let mut app = app();
    let mut delay = crate::view::effects::tests::effect("org.surge-synth-team.surge-xt-fx");
    delay.plugin.name = "Surge XT Effects".into();
    let display = "Surge XT Effects: Delay/Ping-Pong 1-16";
    delay.preset_name = Some(display.into());
    delay.reference = Some(PatchReference {
        plugin: delay.plugin.clone(),
        source: Source::Effect {
            path: "X:/fx_presets/Delay/Ping-Pong 1-16.srgfx".into(),
            value: "Delay/Ping-Pong 1-16.srgfx".into(),
        },
        display: display.into(),
    });
    let mut tone = crate::view::effects::tests::effect("com.tone3000.plugin");
    tone.plugin.name = "TONE3000".into();
    let tone_preset = "Bogner Fullstack — Modern High Gain";
    tone.preset_name = Some(tone_preset.into());

    let mut master_tone = tone.clone();
    master_tone.preset_name = Some(format!("TONE3000: {tone_preset}"));
    app.audition
        .set_master(cat_smf_editor::master_effects::Settings {
            effects: vec![delay.clone(), master_tone],
            ..Default::default()
        });
    app.loaded.as_mut().unwrap().patches[0].effects = vec![delay, tone];
    let ctx = egui::Context::default();
    draw(&mut app, &ctx, vec![]);
    let output = draw(&mut app, &ctx, vec![]);
    for (label, visible_prefix) in [
        (
            "1 Delay · Ping-Pong 1-16 · Surge XT Effects".to_owned(),
            "1 Delay · Ping",
        ),
        (format!("2 TONE3000 · {tone_preset}"), "2 TONE3000 · Bog"),
    ] {
        let shown: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == label => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(shown.len(), 2, "{label}: master and track");
        assert!(!shown[0].galley.elided, "{label}: master");
        let track = &shown[1].galley;
        assert!(track.elided, "{label}: track");
        let visible = track.rows[0].text();
        assert!(visible.starts_with(visible_prefix), "{visible}");
    }
}

#[test]
fn unequal_effect_chains_share_a_scale_and_clicks_match_its_labels() {
    for counts in [[0, 0], [0, 3], [2, 1]] {
        let mut app = app();
        for (track, count) in counts.into_iter().enumerate() {
            app.loaded.as_mut().unwrap().patches[track].effects = (0..count)
                .map(|slot| crate::view::effects::tests::effect(&format!("fx-{track}-{slot}")))
                .collect();
        }
        let ctx = egui::Context::default();
        draw(&mut app, &ctx, vec![]);
        let output = draw(&mut app, &ctx, vec![]);
        let knobs = handles(&output);
        assert_eq!(knobs.len(), 2);
        assert!(
            (knobs[0].y - knobs[1].y).abs() < 0.1,
            "{counts:?}: {knobs:?}"
        );
        // Each strip carries its own scale, whose 0 dB label meets its handle.
        let zero = labels(&output, "+0 dB");
        assert_eq!(
            zero.len(),
            4,
            "scale plus current-value label for each strip"
        );
        for (track, knob) in knobs.iter().enumerate() {
            assert!((zero[track * 2].y - knob.y).abs() < 0.1);
        }
        for db in [6, 0, -6, -12, -18, -24, -30, -36] {
            let output = draw(&mut app, &ctx, vec![]);
            let marks = labels(&output, &format!("{db:+} dB"));
            // Current-value labels follow each scale in paint order; choose the
            // scale by its y coordinate within the fader, above the value labels.
            let y = marks.iter().map(|p| p.y).min_by(f32::total_cmp).unwrap();
            for (track, knob) in knobs.iter().enumerate() {
                press(&mut app, &ctx, egui::pos2(knob.x, y));
                assert_eq!(
                    app.audition.gain(track),
                    crate::play::track_volume::db_to_gain(db),
                    "{counts:?}, track {track}, {db} dB"
                );
                assert_eq!(app.playback.mixer_track, track);
                assert!(!app.playback.mixer_master);
            }
        }
        if counts[1] > 0 {
            let output = draw(&mut app, &ctx, vec![]);
            let effect = labels(&output, "1 fx-1-0")[0];
            press(&mut app, &ctx, effect);
            assert!(app.effect_editor.is_open());
            assert_eq!(app.track, 1);
        }
    }
}
