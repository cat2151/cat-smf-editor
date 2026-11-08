//! Alt+F4 quits the app from any window. egui reports it exactly like the × button, so a winit
//! message hook notes the key before the close request arrives.
use std::sync::atomic::{AtomicBool, Ordering};

static PRESSED: AtomicBool = AtomicBool::new(false);

/// The leading fields of the Win32 `MSG` that winit hands to the hook.
#[cfg(windows)]
#[repr(C)]
struct Msg {
    hwnd: isize,
    message: u32,
    wparam: usize,
}

#[cfg(windows)]
pub(crate) fn hook(options: &mut eframe::NativeOptions) {
    use winit::platform::windows::EventLoopBuilderExtWindows;
    const WM_SYSKEYDOWN: u32 = 0x0104;
    const VK_F4: usize = 0x73;
    options.event_loop_builder = Some(Box::new(|builder| {
        builder.with_msg_hook(|msg| {
            // SAFETY: winit passes a valid MSG that lives for the duration of the call.
            let msg = unsafe { &*(msg as *const Msg) };
            if msg.message == WM_SYSKEYDOWN && msg.wparam == VK_F4 {
                PRESSED.store(true, Ordering::SeqCst);
            }
            false
        });
    }));
}

/// The editor's close takes the press, so a window only steps aside for it.
pub(crate) fn take() -> bool {
    PRESSED.swap(false, Ordering::SeqCst)
}

/// A window's own close: false while Alt+F4 quits the app instead, which keeps the window open.
pub(crate) fn closes(ctx: &eframe::egui::Context) -> bool {
    if !ctx.input(|i| i.viewport().close_requested()) {
        return false;
    }
    if PRESSED.load(Ordering::SeqCst) {
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::CancelClose);
        return false;
    }
    true
}
