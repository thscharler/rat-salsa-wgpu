use crate::Control;
use crate::event_type::ConvertEvent;
use crate::font_data::FontData;
use crate::poll::PollEvents;
use ratatui::Terminal;
use ratatui::style::Color;
use ratatui_wgpu::shaders::AspectPreservingDefaultPostProcessor;
use ratatui_wgpu::{Builder, Dimensions, WgpuBackend};
use std::num::NonZeroU32;
use std::sync::Arc;
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
    pub(crate) rapid_blink: u64,
    pub(crate) slow_blink: u64,
    pub(crate) win_attr: WindowAttributes,
    /// window callback
    pub(crate) cr_window: Box<dyn FnOnce(&ActiveEventLoop, WindowAttributes) -> Window>,
    /// terminal callback
    pub(crate) cr_term: Box<
        dyn FnOnce(
            TerminalArg,
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
            rapid_blink: Default::default(),
            slow_blink: Default::default(),
            win_attr: WindowAttributes::default().with_title("rat-salsa & ratatui-wgpu"),
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

    /// Set the initial font size in pixel.
    /// This will be adjusted by the scaling factor of the system.
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

    /// Use the given interval in milliseconds as the rapid blink speed.
    ///
    /// Note that this is not enough to start blinking text. You also need
    /// to add [PollTick] for the actual rendering.
    pub fn rapid_blink_millis(mut self, t: u64) -> Self {
        self.rapid_blink = t;
        self
    }

    /// Use the given interval in milliseconds as the slow blink speed.
    ///
    /// Note that this is not enough to start blinking text. You also need
    /// to add [PollTick] for the actual rendering.
    pub fn slow_blink_millis(mut self, t: u64) -> Self {
        self.slow_blink = t;
        self
    }

    pub fn window_title(mut self, title: impl Into<String>) -> Self {
        self.win_attr = self.win_attr.with_title(title);
        self
    }

    pub fn window_position(mut self, pos: impl Into<winit::dpi::Position>) -> Self {
        self.win_attr = self.win_attr.with_position(pos);
        self
    }

    pub fn window_size(mut self, size: impl Into<winit::dpi::Size>) -> Self {
        self.win_attr = self.win_attr.with_inner_size(size);
        self
    }

    pub fn window_attr(mut self, attr: WindowAttributes) -> Self {
        self.win_attr = attr;
        self
    }

    pub fn window(
        mut self,
        window_init: impl FnOnce(&ActiveEventLoop, WindowAttributes) -> Window + 'static,
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
            TerminalArg,
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

/// Parameters passed to the terminal init function.
pub struct TerminalArg {
    pub window: Arc<Window>,
    pub fg_color: Color,
    pub bg_color: Color,
    pub rapid_blink: u64,
    pub slow_blink: u64,
}

fn create_font_by_family(family: String) -> impl FnOnce(&fontdb::Database) -> Vec<fontdb::ID> {
    move |fontdb| {
        fontdb
            .faces()
            .filter_map(|info| {
                for (v, _) in &info.families {
                    if v.as_str() == family.as_str() {
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
                Some(info.id)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

fn create_window(event_loop: &ActiveEventLoop, mut attr: WindowAttributes) -> Window {
    attr = attr.with_visible(false);
    // let attr = WindowAttributes::default()
    //     .with_position(PhysicalPosition::new(0, 0))
    //     .with_min_inner_size()
    //     .with_visible(false);
    event_loop.create_window(attr).expect("event-loop")
}

fn create_wgpu(
    arg: TerminalArg,
) -> Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>> {
    let size = arg.window.inner_size();

    let backend = futures_lite::future::block_on({
        let mut b = Builder::from_font(FontData.fallback_font())
            .with_width_and_height(Dimensions {
                width: NonZeroU32::new(size.width).expect("non-zero width"),
                height: NonZeroU32::new(size.height).expect("non-zero-height"),
            })
            .with_bg_color(arg.bg_color)
            .with_fg_color(arg.fg_color);
        if arg.rapid_blink > 0 {
            b = b.with_rapid_blink_millis(arg.rapid_blink);
        }
        if arg.slow_blink > 0 {
            b = b.with_slow_blink_millis(arg.slow_blink);
        }
        b.build_with_target(arg.window.clone())
    })
    .expect("ratatui-wgpu-backend");

    Terminal::new(backend).expect("ratatui-terminal")
}
