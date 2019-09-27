use std::ptr;

use druid_shell::window::WindowHandle;
use druid::{Widget, BoxConstraints, BaseState, LayoutCtx, PaintCtx, UpdateCtx, EventCtx, Env, Event};
use druid::kurbo::Size;

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
            // geometry: Geometry {
            //     pos: (0.0, 0.0),
            //     size: (100.0, 100.0),
            // },
        })
    }
}

type TODO = ();

pub struct WebViewWidget {
    // XXX: is cloning the WindowHandle even *vaguely* acceptable?
    // Somehow or other, I need access to the DPI for when resizing.
    window: WindowHandle,
    control: Control,
    // geometry: Geometry,
}

impl Widget<TODO> for WebViewWidget {
    fn paint(&mut self, _paint_ctx: &mut PaintCtx, _base_state: &BaseState, _data: &TODO, _env: &Env) {
        // TODO: get this back again. Druid used to pass a geometry here, but now I’m not sure
        // whether the position relative to the *window* can be fetched at all.
        // // Geometry doesn’t implement PartialEq. Maybe omission, maybe this is just a bad idea.
        // if geom.pos != self.geometry.pos && geom.size != self.geometry.size {
        //     self.geometry = geom.clone();

        //     if let Err(err) = self.control.resize(
        //         Some(self.window.px_to_pixels_xy(geom.pos.0, geom.pos.1)),
        //         Some(self.window.px_to_pixels_xy(geom.size.0, geom.size.1)),
        //     ) {
        //         eprintln!("WebViewWidget::paint: resize failed, {}", err);
        //         // … but don’t do anything else, just ignore it.
        //     }
        // }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &TODO,
        _env: &Env,
    ) -> Size {
        bc.constrain((100.0, 100.0))
    }

    fn event(&mut self, _event: &Event, _ctx: &mut EventCtx, _data: &mut TODO, _env: &Env) {}
    fn update(&mut self, _ctx: &mut UpdateCtx, _old: Option<&TODO>, _data: &TODO, _env: &Env) {}
}
