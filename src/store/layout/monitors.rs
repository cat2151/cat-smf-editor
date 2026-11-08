//! Windows desktop geometry is physical pixels, unaffected by differing monitor DPI.
use super::placement::{Resolved, Screen};

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{ffi::c_void, ptr::null_mut};
    type Handle = *mut c_void;
    #[derive(Clone, Copy, Default)]
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct Info {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
        device: [u16; 32],
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumDisplayMonitors(
            dc: Handle,
            clip: *const Rect,
            callback: unsafe extern "system" fn(Handle, Handle, *mut Rect, isize) -> i32,
            data: isize,
        ) -> i32;
        fn GetMonitorInfoW(monitor: Handle, info: *mut Info) -> i32;
        fn SetThreadDpiAwarenessContext(context: Handle) -> Handle;
        fn FindWindowExW(
            parent: Handle,
            after: Handle,
            class: *const u16,
            title: *const u16,
        ) -> Handle;
        fn GetWindowThreadProcessId(window: Handle, process: *mut u32) -> u32;
        fn SetWindowPos(
            window: Handle,
            after: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
    }
    #[link(name = "shcore")]
    unsafe extern "system" {
        fn GetDpiForMonitor(monitor: Handle, kind: i32, x: *mut u32, y: *mut u32) -> i32;
    }

    struct DpiContext(Handle);
    impl DpiContext {
        fn physical() -> Self {
            // SAFETY: changes only this thread, restored by Drop.
            Self(unsafe { SetThreadDpiAwarenessContext(-4isize as Handle) })
        }
    }
    impl Drop for DpiContext {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: the context was returned for this thread.
                unsafe {
                    SetThreadDpiAwarenessContext(self.0);
                }
            }
        }
    }

    struct Survey {
        screens: Vec<Screen>,
        failed: bool,
    }
    unsafe extern "system" fn collect(
        monitor: Handle,
        _: Handle,
        _: *mut Rect,
        data: isize,
    ) -> i32 {
        // SAFETY: EnumDisplayMonitors invokes synchronously with our live Survey.
        let survey = unsafe { &mut *(data as *mut Survey) };
        let mut info = Info {
            size: std::mem::size_of::<Info>() as u32,
            monitor: Rect::default(),
            work: Rect::default(),
            flags: 0,
            device: [0; 32],
        };
        let (mut x, mut y) = (96, 96);
        // SAFETY: buffers are valid and the monitor came from EnumDisplayMonitors.
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0
            || unsafe { GetDpiForMonitor(monitor, 0, &mut x, &mut y) } != 0
        {
            survey.failed = true;
            return 0;
        }
        let name = String::from_utf16_lossy(
            &info.device[..info.device.iter().position(|c| *c == 0).unwrap_or(32)],
        );
        let Some(number) = name
            .strip_prefix(r"\\.\DISPLAY")
            .and_then(|n| n.parse().ok())
        else {
            survey.failed = true;
            return 0;
        };
        let geometry = |r: Rect| [r.left, r.top, r.right - r.left, r.bottom - r.top];
        survey.screens.push(Screen {
            number,
            bounds: geometry(info.monitor),
            work: geometry(info.work),
            scale: x as f32 / 96.0,
        });
        1
    }

    pub(super) fn enumerate() -> Result<Vec<Screen>, String> {
        let _dpi = DpiContext::physical();
        let mut survey = Survey {
            screens: Vec::new(),
            failed: false,
        };
        // SAFETY: callback and its data remain valid for this synchronous call.
        let ok = unsafe {
            EnumDisplayMonitors(
                null_mut(),
                std::ptr::null(),
                collect,
                &mut survey as *mut Survey as isize,
            )
        };
        if ok == 0 || survey.failed || survey.screens.is_empty() {
            return Err("could not read monitor geometry and DPI".into());
        }
        Ok(survey.screens)
    }

    /// Find only windows belonging to this process, never another application's same title.
    pub(super) fn window(title: &str) -> Option<isize> {
        let title: Vec<_> = title.encode_utf16().chain([0]).collect();
        let mut window = null_mut();
        loop {
            // SAFETY: null parent/class select top-level windows; title is terminated.
            window = unsafe { FindWindowExW(null_mut(), window, std::ptr::null(), title.as_ptr()) };
            if window.is_null() {
                return None;
            }
            let mut process = 0;
            // SAFETY: process is a valid output buffer.
            unsafe {
                GetWindowThreadProcessId(window, &mut process);
            }
            if process == std::process::id() {
                return Some(window as isize);
            }
        }
    }

    pub(super) fn place(window: isize, rect: Resolved) -> bool {
        let _dpi = DpiContext::physical();
        let [x, y, w, h] = rect.outer;
        // SAFETY: window was found in this process. No activation or z-order change.
        unsafe { SetWindowPos(window as Handle, null_mut(), x, y, w, h, 0x0014) != 0 }
    }
}

pub(crate) fn enumerate() -> Result<Vec<Screen>, String> {
    #[cfg(windows)]
    {
        windows::enumerate()
    }
    #[cfg(not(windows))]
    {
        Err("monitor_placement is currently supported on Windows only".into())
    }
}

/// Retries until the native viewport exists; remembers its handle so a recreated window is placed.
pub(crate) fn apply(ctx: &eframe::egui::Context, title: &str, target: Resolved) {
    #[cfg(windows)]
    {
        let key = eframe::egui::Id::new(("dock-monitor-placement", ctx.viewport_id()));
        let Some(window) = windows::window(title) else {
            ctx.request_repaint();
            return;
        };
        let stamp = (window, target, ctx.native_pixels_per_point().unwrap_or(1.0));
        let previous = ctx.data(|d| d.get_temp::<((isize, Resolved, f32), u8)>(key));
        let remaining = match previous {
            Some((old, remaining)) if old == stamp => remaining,
            _ => 3,
        };
        if remaining == 0 {
            return;
        }
        // Builder inner-size commands and WM_DPICHANGED can run after this callback.
        // Settle the outer frame over the next passes, then leave manual moves/resizes alone.
        if windows::place(window, target) {
            ctx.data_mut(|d| d.insert_temp(key, (stamp, remaining - 1)));
        }
        ctx.request_repaint();
    }
    #[cfg(not(windows))]
    {
        let _ = (ctx, title, target);
    }
}
