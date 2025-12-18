use ratatui::Terminal;
use ratatui::style::Color;
use ratatui_wgpu::shaders::AspectPreservingDefaultPostProcessor;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::dpi::PhysicalPosition;
use winit::error::EventLoopError;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

/// Captures some parameters for [crate::run_tui()].
pub struct RunConfig<Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    /// winit event-loop
    pub(crate) event_loop: Option<EventLoop<Event>>,
    /// window callback
    pub(crate) window: Option<Box<dyn FnOnce(&ActiveEventLoop) -> Window>>,
    /// terminal callback
    pub(crate) term: Option<Box<
        dyn FnOnce(
            Arc<Window>,
        )
            -> Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>>,
    >>,

    /// List of all event-handlers for the application.
    ///
    /// Defaults to PollTimers, PollCrossterm, PollTasks. Add yours here.
    // pub(crate) poll: Vec<Box<dyn PollEvents<Event, Error>>>,
    pub(crate) _phantom: PhantomData<(Event, Error)>,
}

impl<Event, Error> RunConfig<Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    pub fn default() -> Result<Self, EventLoopError> {
        Ok(Self {
            event_loop: Some(EventLoop::<Event>::with_user_event().build()?),
            window: Some(Box::new(create_window)),
            term: Some(Box::new(create_wgpu)),
            _phantom: Default::default(),
        })
    }

    pub fn window(
        mut self,
        window_init: impl FnOnce(&ActiveEventLoop) -> Window + 'static,
    ) -> Self {
        self.window = Some(Box::new(window_init));
        self
    }

    pub fn terminal(
        mut self,
        wgpu_init: impl FnOnce(
            Arc<Window>,
        ) -> Terminal<
            WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>,
        > + 'static,
    ) -> Self {
        self.term = Some(Box::new(wgpu_init));
        self
    }
}

fn create_window(event_loop: &ActiveEventLoop) -> Window {
    let attr = WindowAttributes::default()
        .with_position(PhysicalPosition::new(0, 0))
        .with_title("rat-salsa")
        .with_visible(false);

    event_loop.create_window(attr).expect("event-loop")
}

fn create_wgpu(
    window: Arc<Window>,
) -> Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>> {
    let size = window.inner_size();

    let backend = futures_lite::future::block_on(
        Builder::from_font(
            Font::new(include_bytes!("CascadiaMono-Regular.ttf")).expect("font"))
            // todo: fonts
            .with_width_and_height(Dimensions {
                width: NonZeroU32::new(size.width).expect("non-zero width"),
                height: NonZeroU32::new(size.height).expect("non-zero-height"),
            })
            .with_bg_color(Color::Black)
            .with_fg_color(Color::White)
            .with_font_size_px(20)
            .build_with_target(window.clone()),
    )
    .expect("ratatui-wgpu-backend");

    Terminal::new(backend).expect("ratatui-terminal")
}
