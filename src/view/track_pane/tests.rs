use crate::app::App;
use cat_smf_editor::{
    chain_settings::PluginState,
    host::PluginKey,
    model::{Event, Music, Track},
    score::instrument::ResolvedLayer,
    score::key_signature::KeySignatures,
};
use eframe::egui::{self, Color32};

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.into(),
    }
}

fn fixture() -> App {
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                event(0, 0xff, 3, b"Lead"),
                event(0, 0xff, 0x59, &[0, 0]),
                event(0, 0x90, 0, &[60, 100]),
                event(480, 0x80, 0, &[60, 0]),
                event(480, 0xff, 0x2f, &[]),
            ],
        }],
    });
    let layers = &mut app.loaded.as_mut().unwrap().patches[0].layers;
    let template = layers[0].clone();
    layers.clear();
    for (id, name) in [(1, "Organ"), (2, "Flute"), (3, "Strings")] {
        let mut layer: ResolvedLayer = template.clone();
        layer.settings.id = id;
        layer.instrument = Some(PluginState {
            reference: None,
            plugin: PluginKey {
                format: "CLAP".into(),
                id: "fixture".into(),
                name: "fixture".into(),
                vendor: "fixture".into(),
                bundle_path: "fixture.clap".into(),
            },
            state: vec![1],
            preset_name: Some(name.into()),
            bypass: false,
        });
        layers.push(layer);
    }
    app
}

/// Every drawn text piece with its color.
fn pieces(app: &mut App) -> Vec<(String, Color32)> {
    let ctx = egui::Context::default();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        ));
    }
    let mut pieces = Vec::new();
    for shape in output.unwrap().shapes {
        if let egui::Shape::Text(text) = shape.shape {
            let job = &text.galley.job;
            for section in &job.sections {
                pieces.push((
                    job.text[section.byte_range.clone()].to_owned(),
                    section.format.color,
                ));
            }
        }
    }
    pieces
}

fn color(pieces: &[(String, Color32)], text: &str) -> Color32 {
    pieces
        .iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("{text:?} is not drawn in {pieces:?}"))
        .1
}

#[test]
fn header_names_every_layer_and_rows_color_contents_and_notes() {
    let mut app = fixture();
    let keys = KeySignatures::new(&app.loaded.as_ref().unwrap().music);
    let pieces = pieces(&mut app);
    let layers = ["Organ", "Flute", "Strings"].map(|name| color(&pieces, name));
    assert!(layers[0] != layers[1] && layers[1] != layers[2] && layers[0] != layers[2]);
    color(&pieces, " │ ");
    assert_eq!(color(&pieces, "Lead"), super::content::VALUE_COLOR);
    assert_ne!(color(&pieces, "Track name: "), super::content::VALUE_COLOR);
    assert_eq!(
        color(&pieces, "C4"),
        crate::view::pitch_keyboard::note_color(60, keys.map(0).and_then(|k| k.at(0)))
    );
}
