use crate::_private::NonExhaustive;
use crate::event_type::ConvertEvent;
use crate::font_data::FontData;
use crate::poll::PollEvents;
use crate::{Control, PostProcessorBuilder};
use ratatui_core::terminal::Terminal;
use ratatui_core::style::Color;
use ratatui_wgpu::{Builder, ColorTable, Dimensions, Font, WgpuBackend};
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
    /// fallback font
    pub(crate) fallback_font: Option<(String, Font<'static>)>,
    /// font family
    pub(crate) font_family: Option<String>,
    /// font size
    pub(crate) font_size: Option<f64>,
    /// fallback symbol font
    pub(crate) symbol_font: Option<Font<'static>>,
    /// fallback emoji font
    pub(crate) emoji_font: Option<Font<'static>>,
    /// terminal colors
    pub(crate) bg_color: Color,
    pub(crate) fg_color: Color,
    /// blinking stuff.
    pub(crate) rapid_blink: u8,
    pub(crate) slow_blink: u8,
    /// window attributes
    pub(crate) win_attr: WindowAttributes,
    /// window callback
    pub(crate) cr_window: Box<dyn FnOnce(&ActiveEventLoop, WindowAttributes) -> Window>,
    /// terminal callback
    pub(crate) cr_term: Box<dyn FnOnce(TermInit) -> Terminal<WgpuBackend<'static, 'static>>>,

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
            cr_fonts: Box::new(mock_create_fonts),
            fallback_font: FontData
                .fallback_font()
                .map(|f| ("CascadiaMono-Regular".to_string(), f)),
            font_family: None,
            font_size: None,
            symbol_font: FontData.fallback_symbol_font(),
            emoji_font: FontData.fallback_emoji_font(),
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

    /// Set the primary fallback font.
    ///
    /// If you don't load any fonts, this one will be used.
    ///
    /// If a glyph can not be found in any of the regular fonts
    /// the fallback order is:
    /// * this fallback font
    /// * the [symbol_font]
    /// * the [emoji_font]
    ///
    /// __Note__
    ///
    /// The default-feature `fallback_font` will embed `CascadiaMono-Regular`
    /// as fallback font. If this feature is set, it will be used automatically.
    /// You only need this function if you want to set your own fallback.
    /// In that case you probably want to deactivate the feature and save 560KB
    /// of binary size.
    ///
    pub fn fallback_font(mut self, font_name: String, fallback_font: Font<'static>) -> Self {
        self.fallback_font = Some((font_name, fallback_font));
        self
    }

    /// Set the fallback symbol-font.
    /// When glyphs are not found in the other installed fonts
    /// this is one of the fallback fonts.
    pub fn symbol_font(mut self, symbol_font: Font<'static>) -> Self {
        self.symbol_font = Some(symbol_font);
        self
    }

    /// Set the fallback emoji-font.
    /// When glyphs are not found in the other installed fonts
    /// this is one of the fallback fonts.
    pub fn emoji_font(mut self, emoji_font: Font<'static>) -> Self {
        self.emoji_font = Some(emoji_font);
        self
    }

    /// Set the name of the font family that should be loaded as
    /// regular fonts.
    pub fn font_family(mut self, font_family: impl Into<String>) -> Self {
        let font_family = font_family.into();
        self.font_family = Some(font_family.clone());
        self.cr_fonts = Box::new(create_font_by_family(font_family));
        self
    }

    /// Set a constructor for the regular fonts.
    pub fn fonts(
        mut self,
        font_init: impl FnOnce(&fontdb::Database) -> Vec<fontdb::ID> + 'static,
    ) -> Self {
        self.font_family = None;
        self.cr_fonts = Box::new(font_init);
        self
    }

    /// Set the initial font size in pixel. Defaults to 22px.
    /// This will be adjusted by the scaling factor of the system.
    pub fn font_size(mut self, pt_size: f64) -> Self {
        self.font_size = Some(pt_size);
        self
    }

    /// Set the terminal bg color.
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// Set the terminal fg color.
    pub fn fg_color(mut self, color: Color) -> Self {
        self.fg_color = color;
        self
    }

    /// Set the divisor for rapid blinking.
    ///
    /// The divisor says the for every n-th blink event blinking is switched.
    ///
    /// Note that this is not enough to start blinking text. You also need
    /// to add [PollBlink] for the timer.
    pub fn rapid_blink(mut self, t: u8) -> Self {
        self.rapid_blink = t;
        self
    }

    /// Set the divisor for slow blinking.
    ///
    /// The divisor says the for every n-th blink event blinking is switched.
    ///
    /// Note that this is not enough to start blinking text. You also need
    /// to add [PollBlink] for the timer.
    pub fn slow_blink(mut self, t: u8) -> Self {
        self.slow_blink = t;
        self
    }

    /// Creates an icon from 32bpp RGBA data.
    ///
    /// The length of `rgba` must be divisible by 4, and `width * height` must equal
    /// `rgba.len() / 4`. Otherwise, this will panic.
    ///
    /// > In the examples you can find 'img_icon' that can convert images to this
    /// > format. You can then use `include_bytes!` to embed the icon.
    pub fn window_icon(mut self, rgba: Vec<u8>, width: u32, height: u32) -> Self {
        let icon = winit::window::Icon::from_rgba(rgba, width, height).expect("valid icon");
        self.win_attr = self.win_attr.with_window_icon(Some(icon));
        self
    }

    /// Set the window title.
    pub fn window_title(mut self, title: impl Into<String>) -> Self {
        self.win_attr = self.win_attr.with_title(title);
        self
    }

    /// Set the initial window position.
    pub fn window_position(mut self, pos: impl Into<winit::dpi::Position>) -> Self {
        self.win_attr = self.win_attr.with_position(pos);
        self
    }

    /// Set the initial window size.
    pub fn window_size(mut self, size: impl Into<winit::dpi::Size>) -> Self {
        self.win_attr = self.win_attr.with_inner_size(size);
        self
    }

    /// Set all the other window attributes.
    pub fn window_attr(mut self, attr: WindowAttributes) -> Self {
        self.win_attr = attr;
        self
    }

    /// If you absolutely must, you can set a window constructor here.
    ///
    /// You should create the window with `with_visible(false)` otherwise
    /// it might flicker at startup. The window will be set visible after
    /// the first render.
    pub fn window(
        mut self,
        window_init: impl FnOnce(&ActiveEventLoop, WindowAttributes) -> Window + 'static,
    ) -> Self {
        self.cr_window = Box::new(window_init);
        self
    }

    /// Create the WgpuBackend.
    ///
    /// This gets a [TermInit] struct with all the collected parameters.
    pub fn terminal(
        mut self,
        wgpu_init: impl FnOnce(TermInit) -> Terminal<WgpuBackend<'static, 'static>> + 'static,
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
pub struct TermInit {
    /// The fallback fonts to use.
    pub fallback_fonts: Vec<Font<'static>>,
    /// The regular fonts to use.
    pub fonts: Vec<Font<'static>>,
    /// Premultiplied font-size.
    pub font_size_px: u32,
    /// The window instance.
    pub window: Arc<Window>,
    /// Terminal fg color.
    pub fg_color: Color,
    /// Terminal bg color.
    pub bg_color: Color,
    /// Rapid blink rate.
    pub rapid_blink: u8,
    /// Slow blink rate.
    pub slow_blink: u8,

    pub non_exhaustive: NonExhaustive,
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

fn mock_create_fonts(_: &fontdb::Database) -> Vec<fontdb::ID> {
    Vec::default()
}

fn create_window(event_loop: &ActiveEventLoop, mut attr: WindowAttributes) -> Window {
    attr = attr.with_visible(false);
    event_loop.create_window(attr).expect("event-loop")
}

fn create_wgpu(arg: TermInit) -> Terminal<WgpuBackend<'static, 'static>> {
    let size = arg.window.inner_size();

    // VGA base 16 colors.
    let colors = ColorTable {
        BLACK: [0, 0, 0],
        RED: [170, 0, 0],
        GREEN: [0, 170, 0],
        YELLOW: [170, 85, 0],
        BLUE: [0, 0, 170],
        MAGENTA: [170, 0, 170],
        CYAN: [0, 170, 170],
        GRAY: [170, 170, 170],
        DARKGRAY: [85, 85, 85],
        LIGHTRED: [255, 85, 85],
        LIGHTGREEN: [85, 255, 85],
        LIGHTYELLOW: [255, 255, 85],
        LIGHTBLUE: [85, 85, 255],
        LIGHTMAGENTA: [255, 85, 255],
        LIGHTCYAN: [85, 255, 255],
        WHITE: [255, 255, 255],
    };

    let backend = futures_lite::future::block_on({
        let mut b = Builder::<PostProcessorBuilder>::from_fonts(arg.fallback_fonts)
            .with_width_and_height(Dimensions {
                width: NonZeroU32::new(size.width).expect("non-zero width"),
                height: NonZeroU32::new(size.height).expect("non-zero-height"),
            })
            .with_color_table(colors)
            .with_bg_color(arg.bg_color)
            .with_fg_color(arg.fg_color)
            .with_fonts(arg.fonts)
            .with_font_size_px(arg.font_size_px);
        if arg.rapid_blink > 0 {
            b = b.with_rapid_blink_millis(arg.rapid_blink);
        }
        if arg.slow_blink > 0 {
            b = b.with_slow_blink_millis(arg.slow_blink);
        }
        b.build_with_target(arg.window)
    })
    .expect("ratatui-wgpu-backend");

    Terminal::new(backend).expect("ratatui-terminal")
}
