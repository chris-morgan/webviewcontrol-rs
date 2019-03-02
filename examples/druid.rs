use std::borrow::Cow;

use druid_shell::win_main;
use druid_shell::platform::WindowBuilder;

use druid::{Ui, UiMain, UiState};

use winrt::windows::foundation::Uri;
use winrt::windows::web::ui::interop::{WebViewControl};
use winrt::{FastHString};

use webviewcontrol::edge::{self, runtime_context, Process};

fn main() {
    let _rt = runtime_context();

    if !edge::is_available() {
        panic!("EdgeHTML control is not available!");
    }
    let mut args = std::env::args();
    // Ignore program name argument.
    args.next();
    let url: Cow<str> = args
        .next()
        .map(|url| url.into())
        .unwrap_or("http://www.example.com".into());
    println!("Opening a web view to {}", url);

    let process = Process::new();
    let widget = process
        .new_widget(
            &window,
            Some(move |control: &winrt::ComPtr<WebViewControl>| {
                println!("Control created!");
                control
                    .navigate(
                        &Uri::create_uri(&FastHString::from(&*url)).unwrap(),
                    )
                    .unwrap();
            }),
        )
        .unwrap();

    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    state.set_root(widget.ui(&mut state));
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("It’s a WebView! (in theory, anyway)");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
