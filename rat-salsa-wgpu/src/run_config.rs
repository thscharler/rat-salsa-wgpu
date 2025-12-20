use crate::Control;
use crate::event_type::ConvertEvent;
use crate::font_data::FALLBACK_FONT;
use crate::poll::PollEvents;
use log::debug;
use ratatui::Terminal;
use ratatui::style::Color;
use ratatui_wgpu::shaders::AspectPreservingDefaultPostProcessor;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
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
    pub(crate) event_loop: EventLoop<Result<Control<Event>, Error>>,
    /// app event type
    pub(crate) event_type: Box<dyn ConvertEvent<Event>>,
    /// font loading callback
    pub(crate) cr_fonts: Box<dyn FnOnce(&fontdb::Database) -> Vec<fontdb::ID> + 'static>,
    /// font size
    pub(crate) font_size: f64,
    pub(crate) bg_color: Color,
    pub(crate) fg_color: Color,
    pub(crate) window_title: String,
    /// window callback
    pub(crate) cr_window: Box<dyn FnOnce(&ActiveEventLoop) -> Window>,
    /// terminal callback
    pub(crate) cr_term: Box<
        dyn FnOnce(
            Arc<Window>,
            Vec<Font<'static>>,
            f64,
            Color,
            Color,
        )
            -> Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>>,
    >,

    /// List of all event-handlers for the application.
    ///
    /// Defaults to PollTimers, PollCrossterm, PollTasks. Add yours here.
    pub(crate) poll: Vec<Box<dyn PollEvents<Event, Error> + Send>>,
}

impl<Event, Error> RunConfig<Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    pub fn new(event_type: impl ConvertEvent<Event> + 'static) -> Result<Self, EventLoopError> {
        Ok(Self {
            event_loop: EventLoop::with_user_event().build()?,
            event_type: Box::new(event_type),
            cr_fonts: Box::new(create_fonts),
            font_size: 24.0,
            bg_color: Color::Black,
            fg_color: Color::White,
            window_title: "rat-salsa & ratatui-wgpu".to_string(),
            cr_window: Box::new(create_window),
            cr_term: Box::new(create_wgpu),
            poll: Default::default(),
        })
    }

    pub fn font_family(mut self, font_family: impl Into<String>) -> Self {
        self.cr_fonts = Box::new(create_font_by_family(font_family.into()));
        self
    }

    pub fn fonts(
        mut self,
        font_init: impl FnOnce(&fontdb::Database) -> Vec<fontdb::ID> + 'static,
    ) -> Self {
        self.cr_fonts = Box::new(font_init);
        self
    }

    pub fn font_size(mut self, pt_size: f64) -> Self {
        self.font_size = pt_size;
        self
    }

    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    pub fn fg_color(mut self, color: Color) -> Self {
        self.fg_color = color;
        self
    }

    pub fn window_title(mut self, title: String) -> Self {
        self.window_title = title;
        self
    }

    pub fn window(
        mut self,
        window_init: impl FnOnce(&ActiveEventLoop) -> Window + 'static,
    ) -> Self {
        self.cr_window = Box::new(window_init);
        self
    }

    /// Create the WgpuBackend.
    ///
    /// wgpu_init:
    /// - window
    /// - list of fonts
    /// - font-size
    /// - bg-color
    /// - fg-color
    pub fn terminal(
        mut self,
        wgpu_init: impl FnOnce(
            Arc<Window>,
            Vec<Font>,
            f64,
            Color,
            Color,
        ) -> Terminal<
            WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>,
        > + 'static,
    ) -> Self {
        self.cr_term = Box::new(wgpu_init);
        self
    }

    /// Add one more poll impl.
    pub fn poll(mut self, poll: impl PollEvents<Event, Error> + Send + 'static) -> Self {
        self.poll.push(Box::new(poll));
        self
    }
}

fn create_font_by_family(family: String) -> impl FnOnce(&fontdb::Database) -> Vec<fontdb::ID> {
    move |fontdb| {
        fontdb
            .faces()
            .filter_map(|info| {
                for (v, _) in &info.families {
                    if v.as_str() == family.as_str() {
                        debug!("use font {}", info.post_script_name);
                        return Some(info.id);
                    }
                }
                None
            })
            .collect::<Vec<_>>()
    }
}

fn create_fonts(fontdb: &fontdb::Database) -> Vec<fontdb::ID> {
    fontdb
        .faces()
        .filter_map(|info| {
            if (info.monospaced
                || info.post_script_name.contains("Emoji")
                || info.post_script_name.contains("emoji"))
                && info.index == 0
            {
                debug!("use font {}", info.post_script_name);
                Some(info.id)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

fn create_window(event_loop: &ActiveEventLoop) -> Window {
    let attr = WindowAttributes::default()
        .with_position(PhysicalPosition::new(0, 0))
        .with_visible(false);

    event_loop.create_window(attr).expect("event-loop")
}

fn create_wgpu(
    window: Arc<Window>,
    fonts: Vec<Font<'static>>,
    font_size: f64,
    bg_color: Color,
    fg_color: Color,
) -> Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>> {
    let size = window.inner_size();
    let font_size = (font_size * window.scale_factor()).round() as u32;

    let backend = futures_lite::future::block_on(
        Builder::from_font(Font::new(FALLBACK_FONT).expect("font"))
            .with_fonts(fonts)
            .with_width_and_height(Dimensions {
                width: NonZeroU32::new(size.width).expect("non-zero width"),
                height: NonZeroU32::new(size.height).expect("non-zero-height"),
            })
            .with_bg_color(bg_color)
            .with_fg_color(fg_color)
            .with_font_size_px(font_size)
            .build_with_target(window.clone()),
    )
    .expect("ratatui-wgpu-backend");

    Terminal::new(backend).expect("ratatui-terminal")
}
