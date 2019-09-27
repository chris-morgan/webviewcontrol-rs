use std::ptr;

use once_cell::sync::Lazy;

use winapi::shared::minwindef::HINSTANCE;
use winapi::um::libloaderapi;

pub struct HInstanceWrapper(pub HINSTANCE);
unsafe impl Send for HInstanceWrapper {}
unsafe impl Sync for HInstanceWrapper {}

pub static OUR_HINSTANCE: Lazy<HInstanceWrapper> =
    Lazy::new(|| HInstanceWrapper(unsafe { libloaderapi::GetModuleHandleW(ptr::null()) }));
