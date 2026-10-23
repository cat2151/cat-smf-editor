use super::Action;
use crate::app::App;
use eframe::egui;
use std::path::Path;

pub(super) fn prepare(app: &mut App) {
    if app.saved.directory.is_some() && writes_backup(app) && claim_today(app) {
        app.config.navigation.backup_tip_visible = true;
    }
}

/// Follows a direct save that copied the replaced file into backup.
pub(super) fn notice(app: &mut App) {
    if claim_today(app) {
        app.config.navigation.backup_notice = true;
    }
}

fn claim_today(app: &mut App) -> bool {
    let today = local_date();
    if app.config.navigation.backup_tip_date.as_deref() == Some(&today) {
        return false;
    }
    app.config.navigation.backup_tip_date = Some(today);
    app.config.persist();
    true
}

/// Owns input until dismissed, like the other file dialogs.
pub(crate) fn notice_dialog(app: &mut App, ctx: &egui::Context) -> bool {
    if !app.config.navigation.backup_notice {
        return false;
    }
    crate::view::commands::route(app, ctx);
    if !app.config.navigation.backup_notice {
        return true;
    }
    let mut close = false;
    egui::Modal::new(egui::Id::new("backup-notice")).show(ctx, |ui| {
        ui.set_min_width(370.0);
        if let Some(loaded) = &app.loaded {
            ui.strong(&loaded.name);
        }
        body(app, ui);
        if let Some(error) = &app.config.navigation.departure_error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        close |= ui.button("enter / esc: 閉じる").clicked();
    });
    super::super::comma_menu::swallow(ctx);
    if close {
        close_notice(app);
    }
    true
}

pub(crate) fn close_notice(app: &mut App) {
    app.config.navigation.backup_notice = false;
    app.config.navigation.departure_error = None;
}

pub(super) fn show(app: &mut App, ui: &mut egui::Ui) {
    if app.config.navigation.backup_tip_visible {
        body(app, ui);
    }
}

fn body(app: &mut App, ui: &mut egui::Ui) {
    let Some(root) = &app.saved.directory else {
        return;
    };
    let directory = root.join("backup");
    ui.set_max_width(650.0);
    ui.label(
        "TIPS : ファイルを上書き保存すると、元のファイルはバックアップディレクトリに保存されます",
    );
    let clicked = ui.link(directory.display().to_string()).clicked();
    if clicked && let Err(error) = open_directory(&directory) {
        app.config.navigation.departure_error = Some(error);
    }
    ui.separator();
}

/// Only a confirmed save that replaces an existing file copies it into backup.
fn writes_backup(app: &App) -> bool {
    match app.config.navigation.departure {
        None => false,
        Some(_) if app.saved.is_untitled() => false,
        Some(Action::EnableAutoSave) => true,
        Some(_) => app.saved.dirty(),
    }
}

fn local_date() -> String {
    let mut local = [0u16; 8];
    // SAFETY: this aligned buffer holds all eight WORDs of SYSTEMTIME.
    unsafe { GetLocalTime(local.as_mut_ptr()) };
    format!("{:04}-{:02}-{:02}", local[0], local[1], local[3])
}

pub(super) fn open_directory(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let result = (|| -> Result<(), String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: path is NUL terminated and lives through the synchronous call;
        // null operation selects the shell's default open operation.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result <= 32 {
            return Err(format!("ShellExecuteW failed ({result})"));
        }
        Ok(())
    })();
    result.map_err(|e| format!("Cannot open {}: {e}", path.display()))
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLocalTime(time: *mut u16);
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        window: *mut std::ffi::c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
}
