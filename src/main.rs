//! Experimentation ground. Fear not, it’ll all be turned into proper stuff in lib.rs and beyond
//! when it works.

use winit::dpi::LogicalSize;
use winit::os::windows::WindowExt;
use winit::{EventsLoop, WindowBuilder};

use winapi::winrt::roapi::RO_INIT_SINGLETHREADED;
use winrt::windows::foundation::Uri;
use winrt::windows::web::ui::interop::{
    IWebViewControlSite, WebViewControlProcess, WebViewControlProcessOptions,
};
use winrt::{FastHString, RtDefaultConstructible, RuntimeContext};

use winapi::shared::winerror::{S_FALSE, S_OK};
use winapi::winrt::roapi::RoInitialize;
use winrt::windows::foundation::Rect;
use winrt::RtAsyncOperation;

fn main() {
    //let _rt = RuntimeContext::init();

    // RuntimeContext::init() does RO_INIT_MULTITHREADED, but the options object creation fails
    // with code winapi::shared::winerror::RO_E_UNSUPPORTED_FROM_MTA, so I seem to need
    // RO_INIT_SINGLETHREADED.
    let hr = unsafe { RoInitialize(RO_INIT_SINGLETHREADED) };
    assert!(
        hr == S_OK || hr == S_FALSE,
        "failed to call RoInitialize: error {}",
        hr
    );
    let _rt = unsafe { std::mem::transmute::<(), RuntimeContext>(()) };

    let mut events_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("It’s a WebView! (in theory, anyway)")
        .build(&events_loop)
        .unwrap();

    let host_window_handle = window.get_hwnd();

    let options = WebViewControlProcessOptions::new();
    let process = WebViewControlProcess::create_with_options(&options).unwrap();

    println!("I created a WebViewControlProcess!");
    let LogicalSize { width, height } = window.get_inner_size().unwrap();
    let operation = process
        .create_web_view_control_async(
            host_window_handle as usize as i64,
            Rect {
                X: 0.0,
                Y: 0.0,
                Width: width as f32,
                Height: height as f32,
            },
        )
        .expect("Creation call failed");

    println!("the async operation has begun…");

    let control = operation
        .blocking_get()
        .expect("Creation async task failed")
        .expect("creation produced None");

    println!("the control is gotten! Now to navigate—");

    let control_site = control.query_interface::<IWebViewControlSite>().unwrap();
    control_site.set_is_visible(true).unwrap();

    control.navigate(&Uri::create_uri(&FastHString::from("http://www.example.com")).unwrap()).unwrap();

    println!("Well, I seem to have made it through the gauntlet?");

    events_loop.run_forever(|event| {
        println!("{:?}", event);

        match event {
            winit::Event::WindowEvent {
                event: winit::WindowEvent::CloseRequested,
                ..
            } => winit::ControlFlow::Break,
            _ => winit::ControlFlow::Continue,
        }
    });
}
