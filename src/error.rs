use std::fmt;
use std::io;

#[cfg(all(windows, feature = "edgehtml"))]
use winrt;

/// webviewcontrol’s uniform error type.
///
/// The particular variants that are available vary by platform. Here are the variants you can
/// expect:
///
/// - EdgeHTML: the poorly named `Io` for OS errors (the HWND side of things), or `Rt` for WinRT
///   errors (the WebViewControl side of things). As the WinRT errors don’t implement
///   `std::error::Error`, the `source()` method will return `None` for these.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    #[cfg(all(windows, feature = "edgehtml"))]
    Rt(winrt::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "I/O error: {}", err),
            #[cfg(all(windows, feature = "edgehtml"))]
            Error::Rt(ref err) => write!(f, "WinRT error: {:?}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::Io(ref err) => Some(err),
            #[cfg(all(windows, feature = "edgehtml"))]
            Error::Rt(_) => None, // Doesn’t implement std::error::Error
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

#[cfg(all(windows, feature = "edgehtml"))]
impl From<winrt::Error> for Error {
    fn from(error: winrt::Error) -> Error {
        Error::Rt(error)
    }
}
