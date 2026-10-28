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

#[test]
fn singer_header_uses_character_and_style_without_hiding_effects() {
    use cat_smf_editor::voicevox::Voice;
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    let mut effect = loaded.patches[0].layers[0].instrument.clone().unwrap();
    effect.preset_name = Some("Hall".into());
    loaded.patches[0].effects.push(effect);
    let ctx = egui::Context::default();
    for voice in [
        Some(Voice::default()),
        Some(Voice {
            name: "四国めたん".into(),
            style_name: "あまあま".into(),
            style_id: 3000,
            ..Voice::default()
        }),
        None,
    ] {
        Voice::write(&mut loaded.music.tracks[0], voice.as_ref()).unwrap();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                super::header::show(loaded, ui, 0);
            });
        });
        let text: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect();
        assert!(text.contains(&"Hall"));
        if let Some(voice) = voice {
            assert!(text.contains(&voice.label().as_str()), "{text:?}");
            assert!(!text
                .iter()
                .any(|s| s.contains("Organ") || s.contains("Flute")));
        } else {
            assert!(text
                .iter()
                .any(|s| s.contains("Organ") && s.contains("Flute")));
        }
    }
}

fn song(meter: Option<&[u8]>) -> App {
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: meter
                .map(|m| event(0, 0xff, 0x58, m))
                .into_iter()
                .chain([
                    event(0, 0x90, 0, &[60, 100]),
                    event(480, 0x80, 0, &[60, 0]),
                    event(480, 0xff, 0x2f, &[]),
                ])
                .collect(),
        }],
    })
}

/// Runs the whole view for one frame and returns every drawn text with its position.
fn frame(
    app: &mut App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> Vec<(String, egui::Pos2)> {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
    output
        .shapes
        .into_iter()
        .filter_map(|s| match s.shape {
            egui::Shape::Text(t) => Some((t.galley.text().to_owned(), t.pos)),
            _ => None,
        })
        .collect()
}

fn click(at: egui::Pos2) -> [Vec<egui::Event>; 2] {
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    [
        vec![egui::Event::PointerMoved(at), button(true)],
        vec![button(false)],
    ]
}

fn popup_shown(texts: &[(String, egui::Pos2)]) -> bool {
    texts
        .iter()
        .any(|(t, _)| t.starts_with("Invalid time signature"))
}

#[test]
fn default_four_four_is_not_a_warning() {
    let mut app = song(None);
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    let texts = frame(&mut app, &ctx, vec![]);
    assert!(texts.iter().any(|(t, _)| t == "C4"), "{texts:?}");
    assert!(
        !texts.iter().any(|(t, _)| t.starts_with("Warnings")),
        "{texts:?}"
    );
}

#[test]
fn warnings_popup_opens_on_click_and_closes_without_touching_the_view() {
    let mut app = song(Some(&[0, 2, 24, 8]));
    let ctx = egui::Context::default();
    // Tooltips repeat the same text; keep them out so only the popup can draw it.
    ctx.style_mut(|s| s.interaction.tooltip_delay = f32::INFINITY);
    frame(&mut app, &ctx, vec![]);
    let texts = frame(&mut app, &ctx, vec![]);
    assert!(!popup_shown(&texts));
    let label = texts.iter().find(|(t, _)| t == "Warnings 1").unwrap().1 + egui::vec2(4.0, 4.0);
    let toggle = |app: &mut App| {
        let [press, release] = click(label);
        frame(app, &ctx, press);
        frame(app, &ctx, release);
        frame(app, &ctx, vec![])
    };
    assert!(popup_shown(&toggle(&mut app)), "click opens the popup");
    assert!(!popup_shown(&toggle(&mut app)), "a second click closes it");

    assert!(popup_shown(&toggle(&mut app)));
    let [press, release] = click(label + egui::vec2(500.0, 0.0));
    frame(&mut app, &ctx, press);
    frame(&mut app, &ctx, release);
    assert!(
        !popup_shown(&frame(&mut app, &ctx, vec![])),
        "a click outside closes it"
    );

    crate::view::event_selection::begin(&mut app);
    assert!(crate::view::event_selection::valid(&app));
    assert!(popup_shown(&toggle(&mut app)));
    let before = (app.track, app.cursor_row, app.selected_event, app.note_mode);
    let escape = || {
        [true, false].map(|pressed| egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
    };
    frame(&mut app, &ctx, escape().into());
    assert!(
        !popup_shown(&frame(&mut app, &ctx, vec![])),
        "Esc closes it"
    );
    assert!(
        crate::view::event_selection::valid(&app),
        "Esc stays with the popup"
    );
    assert!(!app.help_open);
    assert_eq!(
        before,
        (app.track, app.cursor_row, app.selected_event, app.note_mode)
    );
    frame(&mut app, &ctx, escape().into());
    assert!(
        !crate::view::event_selection::valid(&app),
        "with the popup closed, Esc reaches the selection again"
    );
}
