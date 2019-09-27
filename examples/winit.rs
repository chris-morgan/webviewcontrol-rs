use std::borrow::Cow;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::desktop::EventLoopExtDesktop;
use winit::window::WindowBuilder;

use webviewcontrol::edge::{self, init_single_threaded_apartment, Control, Process, WebView};
use webviewcontrol::edge_winit::{new_control, HwndType};

fn main() {
    init_single_threaded_apartment();

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

    let mut event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("It’s a WebView!")
        .build(&event_loop)
        .unwrap();

    let process = Process::new();
    let control = new_control(
        &process,
        &window,
        HwndType::FillWindow,
        None,
        None,
        Some(move |control: Control| {
            println!("Control created!");
            control.navigate(&url).unwrap();
        }),
    )
    .unwrap();
    control.focus();

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::Focused(false) => {
                    println!("Window lost focus, TODO record whether control was focused");
                }
                WindowEvent::Focused(true) => {
                    println!("Window gained focus, TODO only refocus control if it was before");
                    control.focus();
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(size) => {
                    let size: (u32, u32) = size.to_physical(window.hidpi_factor()).into();
                    // Error in resizing? Meh.
                    let _ = control.resize(None, Some((size.0 as i32, size.1 as i32)));
                }
                _ => (),
            },
            _ => (),
        }
    });
}
