# webviewcontrol-rs

## Goals

A cross-platform web view control, meeting these requirements:

- pure Rust (not needing to build any C++ code or bind to non-platform libraries);
- can use EdgeHTML on suitable versions of Windows, without making the whole app run in the UWP sandbox;
- properly a control, rather than *needing* to be the entire window (though that’s the most common case—so provide a handy function that uses winit for that; on that topic, dialogs don’t belong in this crate, but rather in another crate).

## Similar projects

- [Boscop/web-view](https://github.com/Boscop/web-view) binds [zserge/webview](https://github.com/zserge/webview). Depends on C or C++ code, doesn’t currently support EdgeHTML (though it’s in progress), doesn’t currently support high or mixed DPI environments (though there’s a patch that starts that), requires that it be the entire window.

- [quadrupleslap/tether](https://github.com/quadrupleslap/tether) uses EdgeHTML only on Windows, requires running in the UWP sandbox and adding an appx manifest and C++/CX stuff.

## Status

Windows.Web.Ui.Interop.WebViewControl (EdgeHTML) basic usage is working. Nothing more advanced than navigating to a URL is really supported at present.

No other browser engines are currently supported.

winit usage is working fine.

druid-shell usage is not panning out terribly well, because it’s drawing on top of this. You can see the contents of the WebViewControl while resizing the window, but at other times you’ll see blackness. You are unable to interact with the contents, either.

druid usage is unmaintained (it compiles, though probably not the example, but hasn’t been touched since before major changes; I don’t think I ever had it working, either; I went and focused on druid-shell instead).
