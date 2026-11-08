//! The title reflects keyboard ownership, using the Windows color scheme.
use super::*;

pub(super) fn active(app: &App, ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().focused.unwrap_or(i.focused))
        && !crate::view::modal::is_open(app)
        && !crate::view::modal::owns_frame(ctx)
        && matches!(
            crate::view::commands::owner(app, ctx),
            crate::view::commands::Screen::Drums | crate::view::commands::Screen::DrumInput
        )
}

pub(super) fn style(app: &App, ctx: &egui::Context) -> std::sync::Arc<egui::Style> {
    let original = ctx.style();
    if active(app, ctx) {
        let color = accent().unwrap_or(original.visuals.selection.bg_fill);
        // Pick the higher-contrast foreground for light as well as dark accents.
        let text = if color.to_normalized_gamma_f32()[..3]
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let linear = if *c <= 0.04045 {
                    *c / 12.92
                } else {
                    ((*c + 0.055) / 1.055).powf(2.4)
                };
                linear * [0.2126, 0.7152, 0.0722][i]
            })
            .sum::<f32>()
            > 0.179
        {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        };
        let mut style = (*original).clone();
        style.visuals.widgets.open.weak_bg_fill = color;
        style.visuals.override_text_color = Some(text);
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.fg_stroke.color = text;
        }
        ctx.set_style(style);
    }
    original
}

#[cfg(windows)]
fn accent() -> Option<egui::Color32> {
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmGetColorizationColor(color: *mut u32, opaque: *mut i32) -> i32;
    }
    let mut color = 0;
    let mut opaque = 0;
    // SAFETY: Both pointers refer to initialized, writable values of the API's types.
    let result = unsafe { DwmGetColorizationColor(&mut color, &mut opaque) };
    (result >= 0)
        .then(|| egui::Color32::from_rgb((color >> 16) as u8, (color >> 8) as u8, color as u8))
}

#[cfg(not(windows))]
fn accent() -> Option<egui::Color32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_follows_native_focus_and_keyboard_owner() {
        let mut app = super::super::tests::fixture();
        let ctx = egui::Context::default();
        for focused in [true, false, true] {
            let mut input = egui::RawInput::default();
            input
                .viewports
                .get_mut(&egui::ViewportId::ROOT)
                .unwrap()
                .focused = Some(focused);
            let _ = ctx.run(input, |ctx| {
                assert_eq!(active(&app, ctx), focused);
                app.drum_editor.input = Some(Input::Note {
                    text: "60".into(),
                    focus: false,
                });
                assert_eq!(active(&app, ctx), focused);
                app.player_selection.open = true;
                assert!(!active(&app, ctx));
                app.player_selection.open = false;
                crate::view::modal::mark_owned(ctx);
                assert!(!active(&app, ctx));
            });
        }
    }

    #[test]
    fn window_restores_shared_style_after_drawing() {
        let mut app = super::super::tests::fixture();
        let ctx = egui::Context::default();
        let original = ctx.style();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .focused = Some(true);
        let _ = ctx.run(input, |ctx| {
            super::super::widgets::show(&mut app, ctx, None, &mut None);
            assert_eq!(ctx.style(), original);
        });
    }
}
