#![allow(text_direction_codepoint_in_literal)]

use anyhow::Error;
use log::{debug, error};
use rat_event::{ct_event, event_flow};
use rat_salsa_wgpu::event_type::CompositeWinitEvent;
use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrossterm;
use rat_salsa_wgpu::font_data::FontData;
use rat_salsa_wgpu::poll::PollBlink;
use rat_salsa_wgpu::timer::TimeOut;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::palette::Colors;
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, create_salsa_theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Style};
use ratatui_core::text::Span;
use ratatui_core::widgets::Widget;
use ratatui_wgpu::CursorStyle;
use std::fs;
use std::path::PathBuf;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

static SAMPLES: &[&str] = &["a\u{1f90c}a"];

// const FONT: &str = "Geist Mono";
const FONT: &str = "Overpass Mono";
// const FONT: &str = "MS Gothic";

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Nord");
    let mut global = Global::new(config, theme, SAMPLES);
    let mut state = Minimal::default();

    run_tui(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::new(ConvertCrossterm::new())?
            .window_title("one span")
            .window_position(winit::dpi::LogicalPosition::new(1050, 30))
            .window_size(winit::dpi::LogicalSize::new(200, 200))
            .font_family(FONT)
            .font_size(35.)
            .cursor_color(Color::Red)
            .poll(PollBlink::new(0, 200)),
    )?;

    Ok(())
}

/// Globally accessible data/state.
pub struct Global {
    // the salsa machinery
    ctx: SalsaAppContext<AppEvent, Error>,

    pub cfg: Config,
    pub theme: SalsaTheme,
    pub fonts: Vec<String>,
    pub samples: &'static [&'static str],
}

impl SalsaContext<AppEvent, Error> for Global {
    fn set_salsa_ctx(&mut self, app_ctx: SalsaAppContext<AppEvent, Error>) {
        self.ctx = app_ctx;
    }

    fn salsa_ctx(&self) -> &SalsaAppContext<AppEvent, Error> {
        &self.ctx
    }
}

impl Global {
    pub fn new(cfg: Config, theme: SalsaTheme, samples: &'static [&'static str]) -> Self {
        let mut fonts = FontData.installed_fonts().clone();
        fonts.insert(0, "<Fallback>".to_string());
        Self {
            ctx: Default::default(),
            cfg,
            theme,
            fonts,
            samples,
        }
    }
}

/// Configuration.
#[derive(Debug, Default)]
pub struct Config {}

#[derive(Debug)]
pub enum AppEvent {
    NoOp,
    Event(CompositeWinitEvent),
    CtEvent(crossterm::event::Event),
    TimeOut(TimeOut),
    Quit,
    Rendered,
}

impl From<crossterm::event::Event> for AppEvent {
    fn from(value: crossterm::event::Event) -> Self {
        AppEvent::CtEvent(value)
    }
}

impl From<CompositeWinitEvent> for AppEvent {
    fn from(value: CompositeWinitEvent) -> Self {
        AppEvent::Event(value)
    }
}

#[derive(Debug, Default)]
pub struct Minimal {
    pub font_idx: usize,
    pub sample_idx: usize,
    pub underline: bool,
    pub cursor: usize,
}

pub fn init(_state: &mut Minimal, _ctx: &mut Global) -> Result<(), Error> {
    Ok(())
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<(), Error> {
    buf.set_style(area, ctx.theme.style_style(Style::CONTAINER_BASE));

    let mut glyph_style = ctx.theme.p.high_bg_style(Colors::Yellow, Colors::Green, 5);
    if state.underline {
        glyph_style = glyph_style.underlined();
    }
    let bg_style = ctx.theme.p.high_bg_style(Colors::Yellow, Colors::Red, 5);

    let gl_area = Rect::new(0, 1, area.width, 1);
    buf.set_style(gl_area, bg_style);

    let gl_span_area = Rect::new(2, 1, area.width.saturating_sub(2), 1);
    Span::from(ctx.samples[state.sample_idx])
        .style(glyph_style)
        .render(gl_span_area, buf);

    let cx = ctx.samples[state.sample_idx]
        .graphemes(true)
        .map(|v| v.width() as u16)
        .take(state.cursor)
        .sum::<u16>();
    ctx.set_screen_cursor(Some((2 + cx, 1)));

    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::CtEvent(event) = event {
        match &event {
            ct_event!(resized) => event_flow!(Control::Changed),
            ct_event!(key press CONTROL-'q') => event_flow!(Control::Quit),

            ct_event!(keycode press F(1)) => event_flow!({
                state.font_idx = (state.font_idx + 1) % ctx.fonts.len();
                let font = ctx.fonts[state.font_idx].as_str();
                debug!("set_font_family {:?}", font);
                ctx.set_font_family(font);
                Control::Changed
            }),
            ct_event!(keycode press SHIFT-F(1)) => event_flow!({
                state.font_idx = (state.font_idx.saturating_sub(1)) % ctx.fonts.len();
                let font = ctx.fonts[state.font_idx].as_str();
                debug!("set_font_family {:?}", font);
                ctx.set_font_family(font);
                Control::Changed
            }),

            ct_event!(keycode press F(2)) => event_flow!({
                let v = ctx.font_size();
                if v < 60.0 {
                    ctx.set_font_size(v + 1.0);
                    Control::Changed
                } else {
                    Control::Continue
                }
            }),
            ct_event!(keycode press SHIFT-F(2)) => event_flow!({
                let v = ctx.font_size();
                if v > 7.0 {
                    ctx.set_font_size(v - 1.0);
                    Control::Changed
                } else {
                    Control::Continue
                }
            }),

            ct_event!(keycode press F(3)) => event_flow!({
                if state.sample_idx + 1 < ctx.samples.len() {
                    state.sample_idx += 1;
                    Control::Changed
                } else {
                    Control::Continue
                }
            }),
            ct_event!(keycode press SHIFT-F(3)) => event_flow!({
                if state.sample_idx > 0 {
                    state.sample_idx -= 1;
                    Control::Changed
                } else {
                    Control::Continue
                }
            }),

            ct_event!(keycode press F(4)) => event_flow!({
                state.underline = !state.underline;
                Control::Changed
            }),

            ct_event!(keycode press F(5)) => event_flow!({
                let n = match ctx.terminal().borrow().backend().cursor_style() {
                    CursorStyle::Block => CursorStyle::Underscore,
                    CursorStyle::Underscore => CursorStyle::BoldUnderscore,
                    CursorStyle::BoldUnderscore => CursorStyle::Bar,
                    CursorStyle::Bar => CursorStyle::BoldBar,
                    CursorStyle::BoldBar => CursorStyle::Block,
                };
                ctx.terminal()
                    .borrow_mut()
                    .backend_mut()
                    .set_cursor_style(n);
                Control::Blink
            }),

            ct_event!(keycode press Left) => event_flow!({
                if state.cursor > 0 {
                    state.cursor -= 1;
                }
                Control::Changed
            }),
            ct_event!(keycode press Right) => event_flow!({
                let w = ctx.samples[state.sample_idx].graphemes(true).count();
                if state.cursor + 1 < w {
                    state.cursor += 1;
                }
                Control::Changed
            }),

            _ => {}
        }
    }

    Ok(Control::Continue)
}

pub fn error(
    event: Error,
    _state: &mut Minimal,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    error!("{:?}", event);
    Ok(Control::Changed)
}

fn setup_logging() -> Result<(), Error> {
    let log_path = PathBuf::from("");
    let log_file = log_path.join("one_glyph.log");
    _ = fs::remove_file(&log_file);
    fern::Dispatch::new()
        .format(|out, message, record| {
            if record.target() == "rat_salsa_wgpu::framework" {
                out.finish(format_args!("{}", message)) //
            }
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file(&log_file)?)
        .apply()?;
    Ok(())
}
