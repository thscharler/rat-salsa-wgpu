use log::debug;
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
    /// font loading callback
    pub(crate) fonts: Option<Box<dyn FnOnce(&fontdb::Database) -> Vec<fontdb::ID> + 'static>>,
    /// font size
    pub(crate) font_size: f64,
    pub(crate) bg_color: Color,
    pub(crate) fg_color: Color,
    /// window callback
    pub(crate) window: Option<Box<dyn FnOnce(&ActiveEventLoop) -> Window>>,
    /// terminal callback
    pub(crate) term: Option<
        Box<
            dyn FnOnce(
                Arc<Window>,
                Vec<Font<'static>>,
                f64,
                Color,
                Color,
            ) -> Terminal<
                WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>,
            >,
        >,
    >,

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
            fonts: Some(Box::new(create_fonts)),
            font_size: 24.0,
            bg_color: Color::Black,
            fg_color: Color::White,
            window: Some(Box::new(create_window)),
            term: Some(Box::new(create_wgpu)),
            _phantom: Default::default(),
        })
    }

    pub fn font_family(mut self, font_family: impl Into<String>) -> Self {
        self.fonts = Some(Box::new(create_font_by_family(font_family.into())));
        self
    }

    pub fn fonts(
        mut self,
        font_init: impl FnOnce(&fontdb::Database) -> Vec<fontdb::ID> + 'static,
    ) -> Self {
        self.fonts = Some(Box::new(font_init));
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

    pub fn window(
        mut self,
        window_init: impl FnOnce(&ActiveEventLoop) -> Window + 'static,
    ) -> Self {
        self.window = Some(Box::new(window_init));
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
        self.term = Some(Box::new(wgpu_init));
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
        .with_title("rat-salsa")
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
        Builder::from_font(Font::new(include_bytes!("CascadiaMono-Regular.ttf")).expect("font"))
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
