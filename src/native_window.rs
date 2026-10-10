//! Shape the actual HWND, including hit testing outside its rounded corners.
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::{
        Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
        },
        Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn},
    },
    UI::{
        HiDpi::GetDpiForWindow,
        WindowsAndMessaging::{GetWindowRect, IsIconic},
    },
};

#[derive(Default)]
pub struct Corners {
    square: Option<bool>,
    use_region: bool,
    region: Option<(bool, i32, i32, u32)>,
}

fn hwnd(window: &impl HasWindowHandle) -> Option<HWND> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as HWND),
        _ => None,
    }
}

impl Corners {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut corners = Self::default();
        #[cfg(feature = "ui-qa")]
        {
            // Exercise the Windows 10 path on a Windows 11 QA host as well.
            corners.use_region =
                std::env::var("DOODLE_QA_VIEW").is_ok_and(|view| view == "layout-corners-fallback");
        }
        corners.sync(&cc.egui_ctx, cc);
        corners
    }

    pub fn sync(&mut self, ctx: &egui::Context, window: &impl HasWindowHandle) {
        let Some(hwnd) = hwnd(window) else { return };
        let square = ctx.input(|i| {
            i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
        });
        // SAFETY: the borrowed handle belongs to this live eframe window. All calls
        // run synchronously on its UI thread; attribute/RECT pointers stay valid.
        unsafe {
            if IsIconic(hwnd) != 0 {
                return;
            }
            if !self.use_region {
                if self.square == Some(square) {
                    return;
                }
                let preference = if square {
                    DWMWCP_DONOTROUND
                } else {
                    DWMWCP_ROUND
                };
                let result = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    (&preference as *const i32).cast(),
                    size_of::<i32>() as u32,
                );
                if result >= 0 {
                    self.square = Some(square);
                    return;
                }
                // The corner attribute is unavailable on Windows 10. Clip a native
                // region there, without making the rendering surface transparent.
                self.use_region = true;
            }
            let mut bounds = RECT::default();
            if GetWindowRect(hwnd, &mut bounds) == 0 {
                return;
            }
            let width = bounds.right - bounds.left;
            let height = bounds.bottom - bounds.top;
            let dpi = GetDpiForWindow(hwnd).max(96);
            let state = (square, width, height, dpi);
            if self.region == Some(state) || width <= 0 || height <= 0 {
                return;
            }
            // Remove the region entirely when maximized/fullscreen so it cannot
            // leave clipped corners or a stale small shape after a mode change.
            let region = if square {
                std::ptr::null_mut()
            } else {
                let diameter = (16 * dpi / 96) as i32;
                let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, diameter, diameter);
                if region.is_null() {
                    return;
                }
                region
            };
            if SetWindowRgn(hwnd, region, 1) != 0 {
                // Windows owns a successfully assigned region; never delete it.
                self.region = Some(state);
            } else if !region.is_null() {
                DeleteObject(region);
            }
        }
    }

    #[cfg(feature = "ui-qa")]
    pub fn assert_applied(&self, window: &impl HasWindowHandle, rounded: bool) {
        use windows_sys::Win32::Graphics::{
            Dwm::DwmGetWindowAttribute,
            Gdi::{CreateRectRgn, GetWindowRgn, PtInRegion, COMPLEXREGION, RGN_ERROR},
        };
        let hwnd = hwnd(window).expect("native Windows handle");
        // Read back OS state rather than asserting only the cached request.
        unsafe {
            if !self.use_region {
                let mut actual = 0i32;
                assert!(
                    DwmGetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                        (&mut actual as *mut i32).cast(),
                        size_of::<i32>() as u32,
                    ) >= 0
                );
                assert_eq!(
                    actual,
                    if rounded {
                        DWMWCP_ROUND
                    } else {
                        DWMWCP_DONOTROUND
                    }
                );
            } else {
                let region = CreateRectRgn(0, 0, 0, 0);
                assert!(!region.is_null());
                let kind = GetWindowRgn(hwnd, region);
                let mut bounds = RECT::default();
                assert_ne!(GetWindowRect(hwnd, &mut bounds), 0);
                let w = bounds.right - bounds.left;
                let h = bounds.bottom - bounds.top;
                let corners = [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)];
                let edges = [(w / 2, 0), (w / 2, h - 1), (0, h / 2), (w - 1, h / 2)];
                let corners_clipped = corners.iter().all(|&(x, y)| PtInRegion(region, x, y) == 0);
                let edges_intact = edges.iter().all(|&(x, y)| PtInRegion(region, x, y) != 0);
                DeleteObject(region);
                if rounded {
                    assert_eq!(kind, COMPLEXREGION);
                    assert!(corners_clipped, "all four corners must be clipped");
                    assert!(edges_intact, "resizing must preserve all four edges");
                } else {
                    assert_eq!(kind, RGN_ERROR, "maximized/fullscreen must have no region");
                }
            }
        }
    }
}
