use std::time::Duration;

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::{
        space::SpaceElement,
        utils::{send_frames_surface_tree, under_from_surface_tree},
        Window, WindowSurfaceType,
    },
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    render_elements,
    utils::{IsAlive, Logical, Point, Rectangle},
    wayland::{compositor::SurfaceData, seat::WaylandFocus, shell::xdg::ToplevelSurface},
    xwayland::X11Surface,
};

// #[derive(Debug, Clone, PartialEq)]
// pub enum WindowElement {
//     Wayland(Window),
//     X11(X11Surface),
//     //TODO: XWayland
// }

// impl WindowElement {
//     pub fn wl_surface(&self) -> Option<WlSurface> {
//         match self {
//             WindowElement::Wayland(w) => w.wl_surface().map(|s| s.into_owned()),
//             WindowElement::X11(w) => w.wl_surface(),
//         }
//     }
//
//     pub fn surface_under(
//         &self,
//         location: Point<f64, Logical>,
//         window_type: WindowSurfaceType,
//     ) -> Option<(WlSurface, Point<i32, Logical>)> {
//         match self {
//             WindowElement::Wayland(w) => w.surface_under(location, window_type),
//             WindowElement::X11(w) => w.wl_surface().and_then(|surface| {
//                 under_from_surface_tree(&surface, location, (0, 0), window_type)
//             }),
//         }
//     }
//
//     // pub fn toplevel(&self) -> &ToplevelSurface {
//     //     match self {
//     //         WindowElement::Wayland(w) => w.toplevel(),
//     //     }
//     // }
//
//     // pub fn on_commit(&self) {
//     //     match self {
//     //         WindowElement::Wayland(w) => w.on_commit(),
//     //     }
//     // }
//
//     pub fn geometry(&self) -> Rectangle<i32, Logical> {
//         match self {
//             WindowElement::Wayland(w) => w.geometry(),
//             WindowElement::X11(w) => w.geometry(),
//         }
//     }
//
//     pub fn send_frame<T, F>(
//         &self,
//         output: &Output,
//         time: T,
//         throttle: Option<Duration>,
//         primary_scan_out_output: F,
//     ) where
//         T: Into<Duration>,
//         F: FnMut(&WlSurface, &SurfaceData) -> Option<Output> + Copy,
//     {
//         match self {
//             WindowElement::Wayland(w) => {
//                 w.send_frame(output, time, throttle, primary_scan_out_output)
//             }
//             WindowElement::X11(w) => {
//                 if let Some(surface) = w.wl_surface() {
//                     send_frames_surface_tree(
//                         &surface,
//                         output,
//                         time,
//                         throttle,
//                         primary_scan_out_output,
//                     );
//                 }
//             }
//         }
//     }
// }
//
// impl IsAlive for WindowElement {
//     fn alive(&self) -> bool {
//         match self {
//             Self::Wayland(w) => w.alive(),
//             Self::X11(w) => w.alive(),
//         }
//     }
// }
//
// impl SpaceElement for WindowElement {
//     fn bbox(&self) -> Rectangle<i32, Logical> {
//         match self {
//             Self::Wayland(w) => w.bbox(),
//             Self::X11(w) => w.bbox(),
//         }
//     }
//
//     fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
//         match self {
//             Self::Wayland(w) => w.is_in_input_region(point),
//             Self::X11(w) => w.is_in_input_region(point),
//         }
//     }
//
//     fn set_activate(&self, activated: bool) {
//         match self {
//             Self::Wayland(w) => w.set_activate(activated),
//             Self::X11(w) => w.set_activate(activated),
//         }
//     }
//
//     fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
//         match self {
//             Self::Wayland(w) => w.output_enter(output, overlap),
//             Self::X11(w) => w.output_enter(output, overlap),
//         }
//     }
//
//     fn output_leave(&self, output: &Output) {
//         match self {
//             Self::Wayland(w) => w.output_leave(output),
//             Self::X11(w) => w.output_leave(output),
//         }
//     }
// }
//
// render_elements!(
//     pub WindowRenderElement<R> where R: ImportAll + ImportMem;
//     Window=WaylandSurfaceRenderElement<R>,
// );
//
// impl<R: Renderer + std::fmt::Debug> std::fmt::Debug for WindowRenderElement<R> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Window(arg0) => f.debug_tuple("Window").field(arg0).finish(),
//             Self::_GenericCatcher(arg0) => f.debug_tuple("_GenericCatcher").field(arg0).finish(),
//         }
//     }
// }
//
// impl<R> AsRenderElements<R> for WindowElement
// where
//     R: Renderer + ImportAll + ImportMem,
//     <R as Renderer>::TextureId: Texture + 'static,
// {
//     type RenderElement = WindowRenderElement<R>;
//
//     fn render_elements<C: From<Self::RenderElement>>(
//         &self,
//         renderer: &mut R,
//         location: Point<i32, smithay::utils::Physical>,
//         scale: smithay::utils::Scale<f64>,
//         alpha: f32,
//     ) -> Vec<C> {
//         match self {
//             WindowElement::Wayland(w) => AsRenderElements::<R>::render_elements::<
//                 WindowRenderElement<R>,
//             >(w, renderer, location, scale, alpha),
//             WindowElement::X11(surface) => AsRenderElements::<R>::render_elements::<
//                 WindowRenderElement<R>,
//             >(surface, renderer, location, scale, alpha),
//         }
//         .into_iter()
//         .map(C::from)
//         .collect()
//     }
// }
