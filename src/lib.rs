//! Experimentation ground. Fear not, more platforms will be added and a consistent API added
//! before it’s done.

pub mod error;
pub use error::Error;

#[macro_use]
extern crate lazy_static;

#[cfg(all(windows, feature = "edgehtml"))]
pub mod edge;

#[cfg(all(windows, feature = "edgehtml", feature = "druid-shell"))]
pub mod edge_druid_shell;

#[cfg(all(windows, feature = "edgehtml", feature = "druid"))]
pub mod edge_druid;

#[cfg(all(windows, feature = "edgehtml", feature = "winit"))]
pub mod edge_winit;

#[cfg(all(windows, feature = "mshtml"))]
pub mod mshtml;

pub enum Backend {
    #[cfg(all(windows, feature = "edgehtml"))]
    EdgeHTML,
    #[cfg(all(windows, feature = "mshtml"))]
    MSHTML,
    #[cfg(feature = "gtk-webkit2")]
    GtkWebkit2,
    #[cfg(feature = "cocoa")]
    Cocoa,
}

pub enum WebViewControl {
    #[cfg(all(windows, feature = "mshtml"))]
    MSHTML(mshtml::Control),
    #[cfg(all(windows, feature = "edgehtml"))]
    EdgeHTML(edge::Control),
    #[cfg(feature = "gtk-webkit2")]
    GtkWebkit2(gtk::Control),
    #[cfg(feature = "cocoa")]
    Cocoa(cocoa::Control),
}
