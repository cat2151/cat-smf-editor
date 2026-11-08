use super::*;

pub(super) fn collect(a: &App, screen: Screen, out: &mut Vec<Command>) {
    if screen != Screen::Global {
        return;
    }
    out.push(Command::plain(screen, "editor.register", &[], |a, c| {
        super::super::command_input::register_editor(a, c);
    }));
    if a.external_editor.executable.is_none() {
        return;
    }
    let directory = a.config.path.as_ref().and_then(|p| p.parent());
    let mut targets = Vec::new();
    if let Some(directory) = directory {
        targets.push((
            "config.edit-layout",
            a.layout
                .path()
                .map(std::path::Path::to_owned)
                .unwrap_or_else(|| crate::store::layout::location::path(directory)),
        ));
        targets.push((
            "config.edit-settings",
            directory.join("settings").with_extension("toml"),
        ));
    }
    if let Some(path) = descriptions::external_path() {
        targets.push(("config.edit-command-palette", path));
    }
    for (id, path) in targets {
        out.push(Command::plain(screen, id, &[], move |a, ctx| {
            a.external_editor.request(
                path.clone(),
                ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT),
            );
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }));
    }
}
