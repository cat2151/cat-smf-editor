use crate::app::App;
use eframe::egui::{self, Key, Modifiers};
use std::path::Path;

pub(super) fn prepare(app: &mut App) {
    if app.saved.directory.is_none() {
        return;
    }
    let today = local_date();
    if app.config.navigation.backup_tip_date.as_deref() != Some(&today) {
        app.config.navigation.backup_tip_visible = true;
        app.config.navigation.backup_tip_date = Some(today);
        app.config.persist();
    }
}

pub(super) fn show(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    if !app.config.navigation.backup_tip_visible {
        return;
    }
    let Some(root) = &app.saved.directory else {
        return;
    };
    let directory = root.join("backup");
    ui.set_max_width(650.0);
    ui.label(
        "TIPS : ファイルを上書き保存すると、元のファイルはバックアップディレクトリに保存されます",
    );
    let clicked = ui.link(directory.display().to_string()).clicked();
    if (clicked || ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::O)))
        && let Err(error) = open_directory(&directory)
    {
        app.config.navigation.departure_error = Some(error);
    }
    ui.separator();
}

fn local_date() -> String {
    let mut local = [0u16; 8];
    // SAFETY: this aligned buffer holds all eight WORDs of SYSTEMTIME.
    unsafe { GetLocalTime(local.as_mut_ptr()) };
    format!("{:04}-{:02}-{:02}", local[0], local[1], local[3])
}

fn open_directory(path: &Path) -> Result<(), String> {
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
