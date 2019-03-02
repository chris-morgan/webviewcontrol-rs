use gtk::Widget;
use glib_sys::GAsyncQueue;

struct WebViewControl {
    window: Widget,
    scroller: Widget,
    webview: Widget,
    inspector_window: Widget,
    queue: *mut GAsyncQueue,
    ready: int,
    js_busy: int,
    should_exit: int,
}
