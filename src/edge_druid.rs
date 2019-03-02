use std::ptr;

use druid_shell::window::WindowHandle;
use druid::{Geometry, Widget, BoxConstraints, LayoutResult, Ui, Id, LayoutCtx, PaintCtx};

use crate::Error;
use crate::edge::{Process, HwndType, Control};

impl Process {
    pub fn new_widget(
        &self,
        window: &WindowHandle,
        callback: Option<impl FnOnce(Control) + 'static>,
    ) -> Result<WebViewWidget, Error> {
        self.create_control(
            // FIXME: I think I declared that the HWND SHOULD NOT be null, yet here I am.
            // Under what circs can it be null, and should we return an error instead?
            HwndType::NewHwndInWindow(window.get_hwnd().unwrap_or(ptr::null_mut())),
            (0, 0),
            window.px_to_pixels_xy(100.0, 100.0),
            callback,
        ).map(|control| WebViewWidget {
            window: window.clone(),
            control: control,
            geometry: Geometry {
                pos: (0.0, 0.0),
                size: (100.0, 100.0),
            },
        })
    }
}

pub struct WebViewWidget {
    // XXX: is cloning the WindowHandle even *vaguely* acceptable?
    // Somehow or other, I need access to the DPI for when resizing.
    window: WindowHandle,
    control: Control,
    geometry: Geometry,
}

impl Widget for WebViewWidget {
    fn paint(&mut self, _paint_ctx: &mut PaintCtx, geom: &Geometry) {
        // Geometry doesn’t implement PartialEq. Maybe omission, maybe this is just a bad idea.
        if geom.pos != self.geometry.pos && geom.size != self.geometry.size {
            self.geometry = geom.clone();
            
            if let Err(err) = self.control.resize(
                Some(self.window.px_to_pixels_xy(geom.pos.0, geom.pos.1)),
                Some(self.window.px_to_pixels_xy(geom.size.0, geom.size.1)),
            ) {
                eprintln!("WebViewWidget::paint: resize failed, {}", err);
                // … but don’t do anything else, just ignore it.
            }
        }
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((100.0, 100.0)))
    }
}

impl WebViewWidget {
    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}
