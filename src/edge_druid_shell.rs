use std::any::Any;
use std::cell::Cell;

use druid_shell::piet::Piet;
use druid_shell::window::{WindowHandle, WinHandler, WinCtx};
use druid_shell::runloop;

use crate::edge::{Process, HwndType, Control};

pub struct WebViewHandler<F> where F: FnOnce(Control) + 'static {
    process: Process,
    control: Cell<Option<Control>>,
    callback: Cell<Option<F>>,
}

impl<F> WebViewHandler<F> where F: FnOnce(Control) + 'static {
    pub fn new(process: Process, callback: Option<F>) -> WebViewHandler<F> {
        WebViewHandler {
            process,
            control: Cell::new(None),
            callback: Cell::new(callback),
        }
    }

    fn with_control<C, R>(&self, f: C) -> Option<R>
        where C: FnOnce(&Control) -> R
    {
        if let Some(control) = self.control.replace(None) {
            let out = f(&control);
            self.control.set(Some(control));
            Some(out)
        } else {
            None
        }
    }
}

impl<F> WinHandler for WebViewHandler<F> where F: FnOnce(Control) + 'static {
    fn connect(&mut self, handle: &WindowHandle) {
        // This is nasty. No good place to send error handling because of how we had to create the
        // handler before the window.
        self.control.set(Some(self.process.create_control(
            // Two modes of operation to care about here: drawing straight to the window’s HWND,
            // or using a new HWND inside it. To opt into the second form, change FillWindow to
            // NewHwndInWindow. Sadly, this doesn’t help.
            HwndType::FillWindow(handle.get_hwnd().unwrap()),
            // The true size will be sorted out by size(), which will queue the size change until
            // the control is created.
            (0, 0),
            (0, 0),
            self.callback.take(),
        ).unwrap()));
    }

    fn paint(&mut self, _piet: &mut Piet, _ctx: &mut dyn WinCtx) -> bool {
        println!("Paint");
        false
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn size(&mut self, width: u32, height: u32, _ctx: &mut dyn WinCtx) {
        // FIXME: If the control isn’t created yet,
        println!("Size: {} × {}", width, height);
        self.with_control(|control| {
            if let Err(err) = control.resize(
                None,
                Some((width as i32, height as i32)),
            ) {
                eprintln!("WebViewHandler::size: resize failed, {}", err);
                // … but don’t do anything else, just ignore it.
            }
        });
    }

    fn destroy(&mut self, _ctx: &mut dyn WinCtx) {
        // The WebViewProcess will quit without our assistance, so don’t worry about it.
        runloop::request_quit();
    }
}
