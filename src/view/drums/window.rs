//! Positions and sizes belong to each sequencer, independently of the edited track.
use crate::{app::App, store::config::Store};
use eframe::egui::{self, Pos2};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Drums,
    Kit,
    Melody,
}

impl Kind {
    fn key(self) -> &'static str {
        match self {
            Self::Drums => "drum-sequencer",
            Self::Kit => "drum-kit-sequencer",
            Self::Melody => "melody-step-sequencer",
        }
    }

    pub(super) fn id(self) -> egui::Id {
        egui::Id::new(self.key())
    }

    fn position(self, config: &Store) -> Option<Pos2> {
        config.drum_windows.get(self.key()).copied().map(Into::into)
    }

    fn remember(self, config: &mut Store, position: Pos2) {
        if position.x.is_finite() && position.y.is_finite() {
            // The normal exit hook persists this; moving does not write every frame.
            config
                .drum_windows
                .insert(self.key().into(), position.into());
        }
    }

    /// Every grid uses this shell so focus colors, style isolation and position
    /// persistence cannot require a second implementation when either changes. In a pane the
    /// body draws straight into it, and the pane's frame shows the keyboard ownership instead.
    pub(super) fn show(
        self,
        app: &mut App,
        ctx: &egui::Context,
        pane: Option<&mut egui::Ui>,
        open: &mut bool,
        body: impl FnOnce(&mut App, &mut egui::Ui),
    ) {
        if let Some(ui) = pane {
            return body(app, ui);
        }
        let original = super::title::style(app, ctx);
        let track_title = &app.loaded.as_ref().unwrap().track_titles[app.track];
        let name = match self {
            Self::Drums => "Drum sequencer",
            Self::Kit => "Drum kit",
            Self::Melody => "Step sequencer",
        };
        let width = (ctx.screen_rect().width() - 32.0).max(1.0);
        let builder = crate::view::presentation::window(
            ctx,
            crate::view::commands::Screen::Drums,
            self.id(),
            format!("{name} · {track_title}"),
        )
        .open(open)
        .collapsible(false)
        .max_width(width);
        // Deliberate layout differences: four fixed rows versus scrollable kit and pitch rows.
        let mut builder = match self {
            Self::Drums => builder
                .resizable([true, false])
                .scroll([false, false])
                .min_width(width.min(320.0))
                .default_width(width.min(1080.0)),
            Self::Kit | Self::Melody => builder.default_size([790.0, 500.0]).min_width(280.0),
        };
        if let Some(position) = self.position(&app.config) {
            builder = builder.default_pos(position);
        }
        if let Some(size) = app.config.drum_window_sizes.get(self.key()) {
            builder = builder.default_size(*size);
        }
        let response = builder.show(ctx, |ui| {
            ui.set_style(original.clone());
            let available = ui.available_size();
            body(app, ui);
            match self {
                Self::Kit | Self::Melody => available.max(ui.min_size()),
                Self::Drums => egui::vec2(available.x.max(ui.min_size().x), ui.min_size().y),
            }
        });
        ctx.set_style(original);
        if let Some(response) = response {
            self.remember(&mut app.config, response.response.rect.min);
            if let Some(size) = response.inner {
                app.config
                    .drum_window_sizes
                    .insert(self.key().into(), size.into());
            }
        }
    }
}

#[cfg(test)]
mod tests;
