//! A control backed by a Win32 EdgeHTML WebView.
//!
//! This uses Windows.Web.UI.Interop.WebViewControl, and requires at a minimum the October 2018
//! build of Windows 10, 17763. (TODO: support detecting whether it’ll work ahead of time.)
//!
//! Most of the Windows.Web.UI.Interop namespace was added in build 17083, but that may not be
//! sufficient for this library, which may depend on AddInitializeScript and perhaps GotFocus and
//! LostFocus, all of which were only introduced in build 17763.
//!
//! See https://docs.microsoft.com/en-us/microsoft-edge/dev-guide#webview for some details of
//! limitations (e.g. Push API isn’t supported). Further known limitations:
//!
//! 1. If you want a native menu, you need to associate an hMenu with the HWND this control uses,
//!    and then it works fine.
//! 2. The WebViewControl will be focused automatically if something on the page displayed takes
//!    focus (e.g. https://www.duckduckgo.com), but not otherwise (e.g. http://www.example.com).
//! 3. Keyboard-based focus switching between controls outside the WebViewControl and inside does
//!    not work out of the box, and I haven’t done anything about that yet.
//! 4. If the WebViewControl is focused, then Alt+F4 won’t work.
//! 5. If the WebViewControl is focused and has no associated hMenu, Alt+Space won’t work.
//!    (I dunno, do we need to add the system menu to it?)
//! 6. For that matter, control sizing is untested in the presence of a menu.
//! 7. It may crash if you look at it funny (e.g. try to navigate to a non-URL).
//!
//! I believe the focus issues are mostly because we’re interacting with it through this Win32
//! interop wrapper rather than the UWP way; the control is actually being run in a separate
//! process. If you’re willing to have your entire program run in the UWP sandbox, you can do
//! things that way, but this crate doesn’t support that at this time, because the author of this
//! crate doesn’t want to live in the UWP sandbox, wants regular Win32 stuff, dislikes the pain of
//! actually buiblding an APPX package if you’re not using Microsoft’s tooling from top to bottom
//! (e.g. C♯ and XAML), and suspects that using EdgeHTML through UWP will require the use of
//! C++/WinRT, with the winrt crate not yet being capable enough. (This last especially may be
//! incorrect.) But if you really just want a Rust UWP EdgeHTML-powered window with no other
//! controls, https://github.com/quadrupleslap/tether is probably a good place to look.

use std::cell::RefCell;
use std::io;
use std::mem;
use std::ptr;
use std::rc::Rc;

use winapi::shared::minwindef::{HINSTANCE, UINT};
use winapi::shared::windef::{HWND, RECT};
use winapi::shared::winerror::{S_FALSE, S_OK};
use winapi::um::winnt::LPCWSTR;
use winapi::um::{libloaderapi, winuser};
use winapi::winrt::roapi::{RoInitialize, RO_INIT_SINGLETHREADED};

use winrt::windows::foundation::{
    metadata::ApiInformation, AsyncOperationCompletedHandler, EventRegistrationToken, Rect,
    TypedEventHandler, Uri,
};
use winrt::windows::web::ui::{
    interop::{IWebViewControlSite, WebViewControl, WebViewControlProcess},
    IWebViewControl,
    //IWebViewControl2,
    WebViewControlScriptNotifyEventArgs,
};
use winrt::{ComPtr, FastHString, RtDefaultConstructible, RuntimeContext};

use crate::error::Error;

/// Dangerously pretend that the wrapped value is Send.
///
/// There are various things where the winrt crate currently unnecessarily requires Send. I’m not
/// sure if they’re all strictly unnecessary or whether there may be some cases where the Sendness
/// is actually of value, but with Windows.Web.UI.Interop.* at least we’re stuck with operating in
/// a single-threaded apartment, and so we can reasonably drop the Send requirement: nothing should
/// ever be escaping to other threads. See https://github.com/contextfree/winrt-rust/issues/63 for
/// discussion. This is naturally quite a dangerous thing to do, but I am courageous.
struct FakeSend<T>(T);
unsafe impl<T> Send for FakeSend<T> {}

struct HInstanceWrapper(HINSTANCE);
unsafe impl Sync for HInstanceWrapper {}
lazy_static! {
    static ref OUR_HINSTANCE: HInstanceWrapper =
        HInstanceWrapper(unsafe { libloaderapi::GetModuleHandleW(ptr::null()) });
}

// L"WebViewControl Host"
static HOST_CLASS_NAME: [u16; 20] = [
    b'W' as u16,
    b'e' as u16,
    b'b' as u16,
    b'V' as u16,
    b'i' as u16,
    b'e' as u16,
    b'w' as u16,
    b'C' as u16,
    b'o' as u16,
    b'n' as u16,
    b't' as u16,
    b'r' as u16,
    b'o' as u16,
    b'l' as u16,
    b' ' as u16,
    b'H' as u16,
    b'o' as u16,
    b's' as u16,
    b't' as u16,
    0,
];

pub fn is_available() -> bool {
    ApiInformation::is_type_present(&FastHString::from("Windows.Web.UI.Interop.WebViewControl"))
        .unwrap_or(false)
    // When we start using AddInitializeScript, which has a higher baseline, switch to this:
    // ApiInformation::is_method_present(
    //     &FastHString::from("Windows.Web.UI.Interop.WebViewControl"),
    //     &FastHString::from("AddInitializeScript"),
    // ).unwrap_or(false)
}

unsafe fn register_host_class() {
    winuser::RegisterClassExW(&winuser::WNDCLASSEXW {
        cbSize: mem::size_of::<winuser::WNDCLASSEXW>() as UINT,
        style: winuser::CS_HREDRAW | winuser::CS_VREDRAW | winuser::CS_OWNDC,
        lpfnWndProc: Some(winuser::DefWindowProcW),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: OUR_HINSTANCE.0,
        hIcon: ptr::null_mut(),
        hCursor: ptr::null_mut(),
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: HOST_CLASS_NAME.as_ptr(),
        hIconSm: ptr::null_mut(),
    });
}

/// Create a window/control for a web view.
///
/// The provided parent SHOULD not be null. Things may break if it is.
///
/// The provided position and size are specified in physical pixels.
fn new_hwnd(parent: HWND, position: (i32, i32), size: (i32, i32)) -> Result<HWND, Error> {
    // Idempotent, as subsequent attempts will silently fail; meh.
    unsafe {
        register_host_class();
    }

    let handle = unsafe {
        winuser::CreateWindowExW(
            0,
            HOST_CLASS_NAME.as_ptr(),
            [0].as_ptr() as LPCWSTR,
            winuser::WS_CHILD | winuser::WS_VISIBLE,
            position.0,
            position.1,
            size.0,
            size.1,
            parent,
            // TODO: fill out hMenu
            ptr::null_mut(),
            OUR_HINSTANCE.0,
            ptr::null_mut(),
        )
    };

    if handle.is_null() {
        return Err(Error::Io(io::Error::last_os_error()));
    }

    Ok(handle)
}

/// Initialize a single-threaded winrt context. This must be called before instantiating a control.
pub fn runtime_context() -> RuntimeContext {
    // RuntimeContext::init() does RO_INIT_MULTITHREADED, but we need a single-threaded context.
    // See https://github.com/contextfree/winrt-rust/issues/62 for details.
    let hr = unsafe { RoInitialize(RO_INIT_SINGLETHREADED) };
    assert!(
        hr == S_OK || hr == S_FALSE,
        "failed to call RoInitialize: error {}",
        hr
    );
    unsafe { mem::transmute::<(), RuntimeContext>(()) }
}

/// What HWND to associate the WebViewControl with, and how to handle resizing.
pub enum HwndType {
    /// Use the top-level window’s HWND. This causes resizing to only affect the WebViewControl,
    /// not the HWND, which the user has… presumably already handled? But what about programmatic
    /// resizing, do we want to support that, hmm? Maybe this type isn’t useful after all? TODO.
    FillWindow(HWND),
    /// Use the HWND passed, taking ownership of it (so that on control destruction
    /// DestroyWindow will be called).
    ConsumeHwnd(HWND),
    /// Create a new HWND with the window as its parent.
    NewHwndInWindow(HWND),
}

#[derive(Clone)]
pub struct Process {
    process: ComPtr<WebViewControlProcess>,
}

impl Process {
    pub fn new() -> Process {
        let process = WebViewControlProcess::new();
        process
            .add_process_exited(&TypedEventHandler::new(move |_proc, _result| {
                eprintln!("WebViewControlProcess exited, should we do anything about it?");
                Ok(())
            }))
            .unwrap();

        Process { process }
    }

    pub fn create_control(
        &self,
        hwnd_type: HwndType,
        position: (i32, i32),
        size: (i32, i32),
        callback: Option<impl FnOnce(Control) + 'static>,
    ) -> Result<Control, Error> {
        let hwnd = match hwnd_type {
            HwndType::FillWindow(hwnd) => hwnd,
            HwndType::ConsumeHwnd(hwnd) => hwnd,
            HwndType::NewHwndInWindow(parent) => new_hwnd(parent, position, size)?,
        };

        let operation = self.process.create_web_view_control_async(
            hwnd as usize as i64,
            Rect {
                X: position.0 as f32,
                Y: position.1 as f32,
                Width: size.0 as f32,
                Height: size.1 as f32,
            },
        )?;

        let control = Control {
            inner: Rc::new(RefCell::new(ControlInner {
                hwnd,
                is_window_hwnd: match hwnd_type {
                    HwndType::FillWindow(_) => true,
                    _ => false,
                },
                control: None,
                queued_bounds_update: None,
            })),
        };

        // I believe AsyncOperationCompletedHandler should simply not require Send, but it does for
        // now. So, time to pretend Send with this menace.
        let mut control2 = FakeSend(control.clone());
        let mut callback = FakeSend(callback);
        operation
            .set_completed(&AsyncOperationCompletedHandler::new(
                move |sender, _args| {
                    // When it doesn’t require Send, the following four lines should reduce to this:
                    // control = operation.get_results().unwrap();
                    let web_view_control = unsafe { &mut *sender }.get_results().unwrap();
                    control2.0.control_created(web_view_control);
                    if let Some(callback) = callback.0.take() {
                        // XXX: unnecessary clone here, because this closure is FnMut rather than
                        // FnOnce as it could in theory safely be.
                        callback(control2.0.clone());
                    }
                    Ok(())
                },
            ))
            .unwrap();

        Ok(control)
    }
}

// A better solution would probably involve futures and Pin.
// Then we could hopefully do away with the Rc<RefCell<_>> wrapping.
#[derive(Clone)]
pub struct Control {
    inner: Rc<RefCell<ControlInner>>,
}

pub struct ControlInner {
    hwnd: HWND,
    is_window_hwnd: bool,

    // Option because it’s async.
    control: Option<ComPtr<WebViewControl>>,

    // Certain operations may be queued while the control is loading. For example, handling resize.
    queued_bounds_update: Option<Rect>,
}

impl ControlInner {
    /// Updates the WebViewControl’s bounds based on the HWND’s current values.
    /// Returns an error if it fails to get the window rect, which I think shouldn’t ever happen.
    /// Currently returns success if the control is simply not ready yet.
    /// TODO: revisit that decision, taking into account also what happens if the window is resized
    /// while the control is girding its loins.
    fn update_bounds(&mut self) -> Result<(), Error> {
        let mut rect = RECT {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        };
        if unsafe { winuser::GetWindowRect(self.hwnd, &mut rect) } == 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }
        self.update_bounds_from_rect(Rect {
            X: if self.is_window_hwnd {
                0.0
            } else {
                rect.left as f32
            },
            Y: if self.is_window_hwnd {
                0.0
            } else {
                rect.top as f32
            },
            Width: (rect.right - rect.left) as f32,
            Height: (rect.bottom - rect.top) as f32,
        })
    }

    fn update_bounds_from_rect(&mut self, rect: Rect) -> Result<(), Error> {
        println!("Updating bounds to {:?}", rect);
        if let Some(ref control) = self.control {
            let control_site = control.query_interface::<IWebViewControlSite>().unwrap();
            control_site.set_bounds(rect)?;
        } else {
            self.queued_bounds_update = Some(rect);
        }
        Ok(())
    }
}

impl Control {
    // For internal use, part of the CreateWebViewControlAsync completed handler.
    fn control_created(&mut self, web_view_control: Option<ComPtr<WebViewControl>>) {
        let mut inner = self.inner.borrow_mut();
        inner.control = web_view_control;
        if let Some(rect) = inner.queued_bounds_update {
            inner.queued_bounds_update = None;
            // There’s nothing we can do if this fails; maybe better to be silent like this?
            let _ = inner.update_bounds_from_rect(rect);
        }
    }

    pub fn resize(
        &self,
        position: Option<(i32, i32)>,
        size: Option<(i32, i32)>,
    ) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        if !inner.is_window_hwnd {
            let (x, y) = position.unwrap_or((0, 0));
            let (width, height) = size.unwrap_or((0, 0));
            let mut flags = winuser::SWP_NOZORDER;
            if position.is_none() {
                flags |= winuser::SWP_NOMOVE;
            }
            if size.is_none() {
                flags |= winuser::SWP_NOSIZE;
            }
            unsafe {
                winuser::SetWindowPos(inner.hwnd, ptr::null_mut(), x, y, width, height, flags);
                winuser::UpdateWindow(inner.hwnd);
            }
        }
        if let Some((width, height)) = size {
            // Bounds X and Y seem to be relative to the HWND, hence zeroing them.
            inner.update_bounds_from_rect(Rect {
                X: 0.0,
                Y: 0.0,
                Width: width as f32,
                Height: height as f32,
            })?;
        } else {
            inner.update_bounds()?;
        }
        Ok(())
    }

    /// Get the underlying HWND associated with this WebViewControl.
    ///
    /// Not sure why you’d want this, but I know we need it for internal stuff.
    pub fn get_hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Get the underlying Windows.Web.UI.Interop.WebViewControl instance.
    ///
    /// This allows you to do more advanced, engine-specific magicks.
    ///
    /// Returns None if the control hasn’t been created yet (it takes a second to get started).
    pub fn get_inner(&self) -> Option<ComPtr<WebViewControl>> {
        self.inner.borrow().control.clone()
    }
}

pub trait WebView {
    type Error;
    fn navigate(&self, url: &str) -> Result<(), Self::Error>;
}

impl WebView for Control {
    type Error = winrt::Error;
    fn navigate(&self, url: &str) -> Result<(), winrt::Error> {
        if let Some(ref control) = self.inner.borrow().control {
            control.navigate(&*Uri::create_uri(&FastHString::from(&*url))?)?;
        }
        Ok(())
    }
}

pub struct EdgeWebViewControl {
    control: ComPtr<WebViewControl>,
}

// The methods commented out need a new release of the winrt crate, and then typically some fixup
// because I haven’t sorted their signatures out.
impl EdgeWebViewControl {
    // --- Properties ---

    /// Returns true if the control is functioning and go_back() can work.
    pub fn can_go_back(&self) -> bool {
        self.control.get_can_go_back().unwrap_or(false)
    }

    /// Returns true if the control is functioning and go_forward() can work.
    pub fn can_go_forward(&self) -> bool {
        self.control.get_can_go_forward().unwrap_or(false)
    }

    /// Returns true if the control is functioning and contains an element that wants to be
    /// fullscreen.
    pub fn contains_full_screen_element(&self) -> bool {
        self.control
            .get_contains_full_screen_element()
            .unwrap_or(false)
    }

    // pub fn default_background_color(&self) {
    //     self.control.get_default_background_color()
    // }

    // pub fn set_default_background_color(&self) {
    //     self.control.set_default_background_color()
    // }

    // pub fn deferred_permission_requests(&self) {
    //     self.control.get_deferred_permission_requests()
    // }

    /// Retrieves the document title.
    ///
    /// Returns an empty string if the control is not functioning.
    pub fn document_title(&self) -> String {
        self.control
            .get_document_title()
            .map(|s| s.to_string())
            .unwrap_or(String::new())
    }

    // /// Sets the zoom factor for the contents of the control.
    // /// Returns 1.0 if the control is not functioning.
    // pub fn scale(&self) -> f64 {
    //     self.control.get_scale()
    // }

    // Skipped properties:
    //
    // • Bounds, because we manage that otherwise.
    // • IsVisible, purely because I can’t think why that exists yet.
    // • Process, because we don’t *want* to expose that cycle.
    // • Settings, because we’ll expose these otherwise, if ever.

    // --- Methods ---

    // pub fn add_initialize_script(&self, script: &str) {
    //     self.control.add_initialize_script(script.into())
    // }

    // pub fn build_local_stream_uri(&self) {}

    // /// The building block for taking a screenshot of the control.
    // pub fn capture_preview_to_stream_async(&self) {}

    pub fn capture_selected_content_to_data_package_async(&self) {}
    pub fn close(&self) {}
    pub fn get_deferred_permission_request_by_id(&self) {}
    pub fn go_back(&self) {}
    pub fn go_forward(&self) {}
    pub fn invoke_script_async(&self) {}
    pub fn move_focus(&self) {}
    pub fn navigate(&self) {}
    pub fn navigate_to_local_stream_uri(&self) {}
    pub fn navigate_to_string(&self) {}
    pub fn navigate_with_http_request_message(&self) {}
    pub fn refresh(&self) {}
    pub fn stop(&self) {}

    // --- Events ---

    // Skipped: various events to do with loading. If you really need them, take the control ComPtr
    // and do it yourself.

    /*
    pub fn add_accelerator_key_pressed<F>(&self, f: F)
        -> Result<EventRegistrationToken, winrt::Error>
        where F: FnMut(TODO) + 'static
    {
        let mut f = FakeSend(f);
        self.control.add_accelerator_key_pressed(&TypedEventHandler::new(
            move |_sender, args: *mut _| {
                let args = unsafe { &mut *args };
                f.0(args);
                Ok(())
            }
        ))
    }
    */

    pub fn add_contains_full_screen_element_changed<F>(
        &self,
        f: F,
    ) -> Result<EventRegistrationToken, winrt::Error>
    where
        F: FnMut(bool) + 'static,
    {
        let mut f = FakeSend(f);
        self.control
            .add_contains_full_screen_element_changed(&TypedEventHandler::new(
                move |sender: *mut IWebViewControl, _args| {
                    let sender = unsafe { &mut *sender };
                    f.0(sender.get_contains_full_screen_element()?);
                    Ok(())
                },
            ))
    }

    /*
    pub fn add_new_window_requested<F>(&self, f: F)
        -> Result<EventRegistrationToken, winrt::Error>
        where F: FnMut(TODO) + 'static
    {
        let mut f = FakeSend(f);
        self.control.add_new_window_requested(&TypedEventHandler::new(
            move |_sender, args: *mut _| {
                let args = unsafe { &mut *args };
                f.0(args);
                Ok(())
            }
        ))
    }

    pub fn add_permission_requested<F>(&self, f: F)
        -> Result<EventRegistrationToken, winrt::Error>
        where F: FnMut(TODO) + 'static
    {
        let mut f = FakeSend(f);
        self.control.add_permission_requested(&TypedEventHandler::new(
            move |_sender, args: *mut _| {
                let args = unsafe { &mut *args };
                f.0(args);
                Ok(())
            }
        ))
    }
    */

    /// Define a function to handle script notifications triggered from JavaScript like this:
    ///
    /// ```javascript
    /// window.external.notify(string)
    /// ```
    pub fn add_script_notify<F>(&self, f: F) -> Result<EventRegistrationToken, winrt::Error>
    where
        F: FnMut(String) + 'static,
    {
        // I do not know whether TypedEventHandler is unconditionally handled in the same thread or
        // not; but for our case at least, we do not need its Sendness. Let’s live dangerously!
        let mut f = FakeSend(f);
        self.control.add_script_notify(&TypedEventHandler::new(
            move |_sender, args: *mut WebViewControlScriptNotifyEventArgs| {
                let args = unsafe { &mut *args };
                // args also has get_uri(), but I figure we don’t need it… for now, at least.
                let value = args.get_value().map(|s| s.to_string())?;
                f.0(value);
                Ok(())
            },
        ))
    }

    /*
    pub fn add_unsafe_content_warning_displaying<F>(&self, f: F) -> Result<EventRegistrationToken, winrt::Error>
    where F: FnMut(TODO) + 'static
    {
        let mut f = FakeSend(f);
        self.control.add_unsafe_content_warning_displaying(&TypedEventHandler::new(
            move |_sender, args: *mut _| {
                let args = unsafe { &mut *args };
                f.0(args);
                Ok(())
            }
        ))
    }

    pub fn add_unsupported_uri_scheme_identified<F>(&self, f: F) -> Result<EventRegistrationToken, winrt::Error>
    where F: FnMut(TODO) + 'static
    {
        let mut f = FakeSend(f);
        self.control.add_unsupported_uri_scheme_identified(&TypedEventHandler::new(
            move |_sender, args: *mut _| {
                let args = unsafe { &mut *args };
                f.0(args);
                Ok(())
            }
        ))
    }

    pub fn add_unviewable_content_identified<F>(&self, f: F) -> Result<EventRegistrationToken, winrt::Error>
    where F: FnMut(TODO) + 'static
    {
        let mut f = FakeSend(f);
        self.control.add_unviewable_content_identified(&TypedEventHandler::new(
            move |_sender, args: *mut _| {
                let args = unsafe { &mut *args };
                f.0(args);
                Ok(())
            }
        ))
    }

    pub fn add_web_resource_requested<F>(&self, f: F) -> Result<EventRegistrationToken, winrt::Error>
    where F: FnMut(TODO) + 'static
    {
        let mut f = FakeSend(f);
        self.control.add_web_resource_requested(&TypedEventHandler::new(
            move |_sender, args: *mut _| {
                let args = unsafe { &mut *args };
                f.0(args);
                Ok(())
            }
        ))
    }
    */
}
