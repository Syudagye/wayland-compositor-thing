use smithay::{
    backend::renderer::{element::texture::TextureRenderElement, ImportAll, ImportMem, Renderer},
    render_elements,
};

use crate::state::elements::WindowRenderElement;

render_elements! {
    pub OutputRenderElements<R> where R: ImportAll + ImportMem;
    Window = WindowRenderElement<R>,
    DebugUi = TextureRenderElement<<R as Renderer>::TextureId>,
}

impl<R: Renderer + std::fmt::Debug> std::fmt::Debug for OutputRenderElements<R>
where
    <R as Renderer>::TextureId: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Window(arg0) => f.debug_tuple("Window").field(arg0).finish(),
            Self::DebugUi(arg0) => f.debug_tuple("DebugUi").field(arg0).finish(),
            Self::_GenericCatcher(arg0) => f.debug_tuple("_GenericCatcher").field(arg0).finish(),
        }
    }
}
