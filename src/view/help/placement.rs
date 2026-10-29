/// Saved coordinates are logical pixels; the OS tests the physical rectangle
/// against all attached monitors, including those left of the primary monitor.
pub(super) fn visible(window: [f32; 4], scale: f32) -> bool {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Rect {
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MonitorFromRect(rect: *const Rect, flags: u32) -> *mut std::ffi::c_void;
        }
        let [x, y, w, h] = window;
        // Require the title-bar area to intersect a display, so the window can
        // be moved even if most of its body lies outside the desktop.
        let rect = Rect {
            left: (x * scale) as i32,
            top: (y * scale) as i32,
            right: ((x + w) * scale) as i32,
            bottom: ((y + h.min(32.0)) * scale) as i32,
        };
        // SAFETY: MonitorFromRect only reads this valid RECT during the call.
        !unsafe { MonitorFromRect(&rect, 0) }.is_null()
    }
    #[cfg(not(windows))]
    {
        let _ = (window, scale);
        true
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn disconnected_monitor_position_is_rejected() {
        assert!(!super::visible(
            [1_000_000.0, 1_000_000.0, 620.0, 680.0],
            1.0
        ));
    }
}
