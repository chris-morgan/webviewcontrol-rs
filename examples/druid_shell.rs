use std::borrow::Cow;

use druid_shell::runloop;
use druid_shell::platform::{WindowBuilder, PresentStrategy};

use webviewcontrol::edge::{self, runtime_context, Process, Control, WebView};
use webviewcontrol::edge_druid_shell::WebViewHandler;

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
    druid_shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    builder.set_present_strategy(PresentStrategy::Hwnd);
    builder.set_handler(Box::new(WebViewHandler::new(
        process,
        Some(move |control: Control| {
            println!("Control created!");
            control.navigate(&url).unwrap();
        }),
    )));
    builder.set_title("It’s a WebView! (in theory, anyway)");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
