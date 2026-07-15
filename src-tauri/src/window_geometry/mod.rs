//! Window-geometry provider for task 0012 (window climbing).
//!
//! Enumerates visible top-level windows on the desktop so the overlay can
//! treat their top edges as climbable ledges. This is inherently
//! platform-specific and best-effort: any OS/session where global window
//! geometry isn't available (Wayland, headless CI, permission denied) simply
//! reports an empty segment list, and the overlay falls back to
//! screen-bottom-only behavior - see the Acceptance Criteria in
//! `docs/tasks/active/0012-pet-playful-interactions-window-climbing.md`.
//!
//! Only macOS is implemented for now (this repo's dev/build environment).
//! Windows (`EnumWindows` + `DWMWA_EXTENDED_FRAME_BOUNDS`) and Linux X11
//! (`_NET_CLIENT_LIST_STACKING`) are tracked as follow-up work; both stubs
//! below already satisfy the "graceful degradation" acceptance criterion.

use serde::Serialize;

/// One horizontal ledge the pet can walk/climb onto: the top edge of a
/// window, in the OS's global desktop coordinate space (macOS "points" -
/// the same unit as CSS/logical pixels, not physical/device pixels),
/// `x0 < x1`. The overlay frontend subtracts its own window origin (also in
/// points) to translate into local canvas coordinates.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSegment {
    pub id: u32,
    pub x0: f64,
    pub x1: f64,
    pub y: f64,
}

#[cfg(target_os = "macos")]
pub fn enumerate_windows() -> Vec<WindowSegment> {
    macos::enumerate_windows()
}

#[cfg(not(target_os = "macos"))]
pub fn enumerate_windows() -> Vec<WindowSegment> {
    // Windows/Linux backends are follow-up work (see module docs); returning
    // an empty list here is the documented graceful-degradation path.
    Vec::new()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::WindowSegment;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;
    use core_graphics::geometry::CGRect;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowBounds, kCGWindowLayer,
        kCGWindowListOptionOnScreenOnly, kCGWindowNumber, kCGWindowOwnerPID,
    };

    /// Windows narrower/shorter than this are excluded (menu extras, tooltips,
    /// popovers) - climbing targets only "real" application windows.
    const MIN_WIDTH: f64 = 120.0;
    const MIN_HEIGHT: f64 = 60.0;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, rect: *mut CGRect) -> u8;
    }

    pub fn enumerate_windows() -> Vec<WindowSegment> {
        let Some(array) = copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID) else {
            // No Screen Recording permission (or no windows at all) - degrade
            // gracefully rather than erroring.
            return Vec::new();
        };

        let own_pid = std::process::id() as i64;

        // `copy_window_info` returns windows front-to-back. Keep that order:
        // it's what lets us later treat earlier entries as "in front of, and
        // therefore able to occlude" later ones.
        let mut windows: Vec<(u32, CGRect)> = Vec::new();

        for ptr in array.get_all_values() {
            let dict: CFDictionary<CFType, CFType> =
                unsafe { TCFType::wrap_under_get_rule(ptr as CFDictionaryRef) };

            if number_value(&dict, unsafe { kCGWindowOwnerPID }) == Some(own_pid) {
                continue;
            }
            // Layer 0 is the normal application-window layer; anything else
            // (menu bar, dock, desktop icons, tooltips) is not climbable
            // terrain.
            if number_value(&dict, unsafe { kCGWindowLayer }) != Some(0) {
                continue;
            }
            let Some(id) = number_value(&dict, unsafe { kCGWindowNumber }) else {
                continue;
            };
            let Some(rect) = bounds_value(&dict, unsafe { kCGWindowBounds }) else {
                continue;
            };
            if rect.size.width < MIN_WIDTH || rect.size.height < MIN_HEIGHT {
                continue;
            }

            windows.push((id as u32, rect));
        }

        // `kCGWindowListOptionOnScreenOnly` excludes minimized/off-space
        // windows, but it still reports windows that are fully or partially
        // hidden behind another window on top of them. Only the portion of
        // each window's top edge that isn't covered by a window in front of
        // it is actually visible/climbable, so clip each edge against every
        // window earlier in the (front-to-back) list.
        let mut segments = Vec::new();
        for (i, (id, rect)) in windows.iter().enumerate() {
            let y = rect.origin.y;
            let mut visible = vec![(rect.origin.x, rect.origin.x + rect.size.width)];

            for (_, front_rect) in &windows[..i] {
                let front_y0 = front_rect.origin.y;
                let front_y1 = front_rect.origin.y + front_rect.size.height;
                if front_y0 <= y && y <= front_y1 {
                    visible = subtract_interval(
                        visible,
                        front_rect.origin.x,
                        front_rect.origin.x + front_rect.size.width,
                    );
                    if visible.is_empty() {
                        break;
                    }
                }
            }

            let full_width = rect.size.width;
            let unsplit = visible.len() == 1
                && (visible[0].1 - visible[0].0 - full_width).abs() < f64::EPSILON;

            for (index, (x0, x1)) in visible.into_iter().enumerate() {
                if x1 - x0 < MIN_WIDTH {
                    continue;
                }
                segments.push(WindowSegment {
                    id: if unsplit {
                        *id
                    } else {
                        id.wrapping_mul(1000) + index as u32
                    },
                    x0,
                    x1,
                    y,
                });
            }
        }

        segments
    }

    /// Removes the `[cut_x0, cut_x1)` range from every interval in
    /// `intervals`, splitting an interval into two where the cut falls in
    /// its middle.
    fn subtract_interval(intervals: Vec<(f64, f64)>, cut_x0: f64, cut_x1: f64) -> Vec<(f64, f64)> {
        let mut result = Vec::new();
        for (a0, a1) in intervals {
            if cut_x1 <= a0 || cut_x0 >= a1 {
                result.push((a0, a1));
                continue;
            }
            if cut_x0 > a0 {
                result.push((a0, cut_x0));
            }
            if cut_x1 < a1 {
                result.push((cut_x1, a1));
            }
        }
        result
    }

    fn number_value(
        dict: &CFDictionary<CFType, CFType>,
        key: core_foundation::string::CFStringRef,
    ) -> Option<i64> {
        let key = unsafe { CFString::wrap_under_get_rule(key) };
        dict.find(key.as_CFType())
            .and_then(|value| value.downcast::<core_foundation::number::CFNumber>())
            .and_then(|number| number.to_i64())
    }

    fn bounds_value(
        dict: &CFDictionary<CFType, CFType>,
        key: core_foundation::string::CFStringRef,
    ) -> Option<CGRect> {
        let key = unsafe { CFString::wrap_under_get_rule(key) };
        let value = dict.find(key.as_CFType())?;
        let bounds_dict_ref = value.as_concrete_TypeRef().cast() as CFDictionaryRef;
        let mut rect = CGRect::new(
            &core_graphics::geometry::CGPoint::new(0.0, 0.0),
            &core_graphics::geometry::CGSize::new(0.0, 0.0),
        );
        let ok = unsafe { CGRectMakeWithDictionaryRepresentation(bounds_dict_ref, &mut rect) };
        if ok == 0 {
            None
        } else {
            Some(rect)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerate_windows_runs_without_panicking() {
        let segments = enumerate_windows();
        eprintln!("found {} window segments", segments.len());
        for segment in &segments {
            eprintln!("{:?}", segment);
        }
    }
}
