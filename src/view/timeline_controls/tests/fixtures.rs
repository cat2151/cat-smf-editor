//! Exhaustive fixture construction for the production screen registry.
use super::*;
use crate::{app::App, view};
use eframe::egui;

pub(super) struct Fixture {
    pub app: App,
    pub ctx: egui::Context,
    pub screen: Screen,
    workspace: Option<view::patterns::workspace::Pane>,
    time: f64,
    pub shapes: Vec<egui::epaint::ClippedShape>,
}

impl Fixture {
    pub fn expected_click(&self) -> Click {
        match self.screen {
            Screen::Staff | Screen::PianoRoll | Screen::Tempo | Screen::TrackOverview => {
                Click::SeekPlaying
            }
            Screen::Parameter => Click::Cursor,
            Screen::Song => Click::Song,
            Screen::Scene => Click::Scene,
        }
    }

    pub fn expected_mode(&self) -> crate::play::playback::Mode {
        use crate::play::playback::Mode;
        match self.expected_click() {
            Click::Song => Mode::Arrangement(None),
            Click::Scene => Mode::Arrangement(self.app.patterns.workspace.scene),
            Click::SeekPlaying | Click::Cursor => Mode::AllTracks,
        }
    }

    pub fn visible(&mut self, visible: bool) {
        match self.screen {
            Screen::Staff | Screen::Song | Screen::Scene => self.app.note_mode = visible,
            Screen::PianoRoll => self.app.piano_roll.open = visible,
            Screen::Tempo => self.app.tempo_curve.open = visible,
            Screen::TrackOverview => self.app.track_overview.open = visible,
            Screen::Parameter => {
                if visible {
                    self.app.parameter_editor.open = true;
                } else {
                    self.app.parameter_editor.close();
                }
            }
        }
    }

    pub fn new(screen: Screen) -> Self {
        // A missing arm after adding a production ID is intentionally a compile error.
        let (mut app, ctx, workspace) = match screen {
            Screen::Song | Screen::Scene => {
                let (mut a, c, _, _, scene) = view::patterns::workspace_tests::verification_song();
                cat_smf_editor::score::scene::sizing::song(
                    &mut a.loaded.as_mut().unwrap().music,
                    80,
                )
                .unwrap();
                let p = if screen == Screen::Song {
                    view::patterns::workspace::Pane::Song
                } else {
                    let meters =
                        cat_smf_editor::score::scene::read(&a.loaded.as_ref().unwrap().music)
                            .unwrap()
                            .scenes
                            .iter()
                            .find(|s| s.id == scene)
                            .unwrap()
                            .meters
                            .clone();
                    let music = &mut a.loaded.as_mut().unwrap().music;
                    let mut collection = cat_smf_editor::score::scene::read(music).unwrap();
                    collection.song.clear();
                    cat_smf_editor::score::scene::write(music, &collection).unwrap();
                    cat_smf_editor::score::scene::sizing::scene(
                        &mut a.loaded.as_mut().unwrap().music,
                        scene,
                        (0..80).map(|i| meters[i % meters.len()].clone()).collect(),
                    )
                    .unwrap();
                    view::patterns::workspace::open_scene(&mut a, scene).unwrap();
                    view::patterns::workspace::Pane::Scene
                };
                (a, c, Some(p))
            }
            Screen::Parameter => (
                view::parameter_editor::tests::app(),
                egui::Context::default(),
                None,
            ),
            Screen::Staff | Screen::PianoRoll | Screen::Tempo | Screen::TrackOverview => {
                let mut music = view::tempo_curve::tests::music();
                for track in &mut music.tracks {
                    track.events.last_mut().unwrap().tick = 153600;
                    cat_smf_editor::edit::refresh_deltas(track).unwrap();
                }
                let mut a = view::tests::app(music);
                match screen {
                    Screen::Staff => {}
                    Screen::PianoRoll => {
                        a.track = 1;
                        a.piano_roll.open = true;
                    }
                    Screen::Tempo => a.tempo_curve.open = true,
                    Screen::TrackOverview => a.track_overview.open = true,
                    Screen::Song | Screen::Scene | Screen::Parameter => unreachable!(),
                }
                (a, egui::Context::default(), None)
            }
        };
        if screen == Screen::Parameter {
            let track = app.track;
            app.edit_tracks(|music| {
                for t in &mut music.tracks {
                    if let Some(e) = t.events.last_mut() {
                        e.tick = e.tick.max(153600);
                    }
                    cat_smf_editor::edit::refresh_deltas(t)?;
                }
                Ok(())
            })
            .unwrap();
            app.track = track;
            app.loaded.as_mut().unwrap().patches[0].layers[0].instrument =
                Some(cat_smf_editor::chain_settings::PluginState {
                    reference: None,
                    plugin: cat_smf_editor::host::PluginKey {
                        format: "CLAP".into(),
                        id: "fixture".into(),
                        name: "Fixture".into(),
                        vendor: String::new(),
                        bundle_path: "X:/fixture.clap".into(),
                    },
                    state: vec![],
                    preset_name: None,
                    bypass: false,
                });
        }
        let mut fixture = Self {
            app,
            ctx,
            screen,
            workspace,
            time: 20.0,
            shapes: Vec::new(),
        };
        for _ in 0..4 {
            fixture.frame(vec![]);
        }
        fixture
    }

    pub fn frame(&mut self, events: Vec<egui::Event>) {
        self.time += 0.02;
        if let Some(p) = self.workspace {
            self.ctx.data_mut(|d| {
                for measure in 1u64..=128 {
                    d.remove::<egui::Rect>(egui::Id::new((p.title(), "cell", 0usize, measure)));
                }
            });

            view::patterns::workspace_tests::frame(&mut self.app, &self.ctx, p.viewport(), events);
            self.shapes = self
                .ctx
                .data(|d| {
                    d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                        "workspace-shapes",
                        p.viewport(),
                    )))
                })
                .unwrap();
        } else {
            let output = self.ctx.run(
                egui::RawInput {
                    time: Some(self.time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1500.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| view::show(&mut self.app, ctx),
            );
            self.shapes = output.shapes;
        }
    }

    pub fn rect(&self, key: &str) -> egui::Rect {
        self.ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((self.screen, key))))
            .expect(key)
    }

    pub fn pointer(&mut self, pos: egui::Pos2, button: egui::PointerButton, pressed: bool) {
        self.frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: Default::default(),
            },
        ]);
    }

    pub fn bytes(&self) -> Vec<u8> {
        cat_smf_editor::smf_io::encode(&self.app.loaded.as_ref().unwrap().music).unwrap()
    }
}
