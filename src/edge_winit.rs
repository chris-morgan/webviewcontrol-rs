use crate::edge::{self, Control, Process};

use winapi::shared::windef::HWND;

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::platform::windows::WindowExtWindows;
use winit::window::{CreationError, Window};

pub enum HwndType {
    FillWindow,
    ConsumeHwnd(HWND),
    NewHwndInWindow,
}

pub fn new_control<F>(
    process: &Process,
    window: &Window,
    hwnd_type: HwndType,
    position: Option<LogicalPosition>,
    size: Option<LogicalSize>,
    callback: Option<F>,
) -> Result<Control, CreationError>
where
    F: FnOnce(Control) + 'static
{
    let window_hwnd = window.get_hwnd() as *mut _;
    let hwnd_type = match hwnd_type {
        HwndType::FillWindow => edge::HwndType::FillWindow(window_hwnd),
        HwndType::ConsumeHwnd(hwnd) => edge::HwndType::ConsumeHwnd(hwnd),
        HwndType::NewHwndInWindow => edge::HwndType::NewHwndInWindow(window_hwnd),
    };
    // Fill in defaults for position and size, and convert them to physical units.
    let dpi_factor = window.get_hidpi_factor();
    let position = position
        .unwrap_or(LogicalPosition { x: 0.0, y: 0.0 })
        .to_physical(dpi_factor)
        .into();
    let size: (u32, u32) = size.or(window.get_inner_size()).unwrap_or(LogicalSize {
        width: 1024.0,
        height: 768.0,
    }).to_physical(dpi_factor).into();
    process.create_control(
        hwnd_type,
        // The true size will be sorted out by size(), which will queue the size change until
        // the control is created.
        position,
        (size.0 as i32, size.1 as i32),
        callback,
    ).map_err(|err| CreationError::OsError(err.to_string()))
}
