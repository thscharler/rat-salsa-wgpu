#![allow(text_direction_codepoint_in_literal)]

use anyhow::Error;
use image::ImageReader;
use log::{debug, error};
use rat_event::{ct_event, event_flow};
use rat_salsa_wgpu::event_type::CompositeWinitEvent;
use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrossterm;
use rat_salsa_wgpu::font_data::FontData;
use rat_salsa_wgpu::poll::PollBlink;
use rat_salsa_wgpu::timer::TimeOut;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, create_salsa_theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use std::fs;
use std::path::PathBuf;

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Nord");
    let mut global = Global::new(config, theme);
    let mut state = Minimal::default();

    run_tui(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::new(ConvertCrossterm::new())?
            .window_title("one img")
            .font_size(22.)
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
    pub fn new(cfg: Config, theme: SalsaTheme) -> Self {
        let mut fonts = FontData.installed_fonts().clone();
        fonts.insert(0, "<Fallback>".to_string());
        Self {
            ctx: Default::default(),
            cfg,
            theme,
            fonts,
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
    pub img1: usize,
    pub img2: usize,
}

pub fn init(state: &mut Minimal, ctx: &mut Global) -> Result<(), Error> {
    let image = ImageReader::open("rat-salsa-wgpu/utils/m45.jpg")?;
    let image = image.decode()?;
    let rgba = image.to_rgba8();
    let rgba = rgba.into_flat_samples();

    let (_c, w, h) = rgba.extents();

    state.img1 =
        ctx.terminal()
            .borrow_mut()
            .backend_mut()
            .add_image(&rgba.samples, w as u32, h as u32);

    let image = ImageReader::open("rat-salsa-wgpu/utils/hknc009.jpg")?;
    let image = image.decode()?;
    let rgba = image.to_rgba8();
    let rgba = rgba.into_flat_samples();

    let (_c, w, h) = rgba.extents();

    state.img2 =
        ctx.terminal()
            .borrow_mut()
            .backend_mut()
            .add_image(&rgba.samples, w as u32, h as u32);

    Ok(())
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<(), Error> {
    buf.set_style(area, ctx.theme.style_style(Style::CONTAINER_BASE));

    let img_buf = ctx.image_buffer();

    img_buf.render_image(state.img1, Rect::new(0, 0, 10, 10));
    img_buf.render_image(state.img2, Rect::new(10, 10, 10, 10));

    Ok(())
}

pub fn event(
    event: &AppEvent,
    _state: &mut Minimal,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::CtEvent(event) = event {
        match &event {
            ct_event!(resized) => event_flow!(Control::Changed),
            ct_event!(key press CONTROL-'q') => event_flow!(Control::Quit),
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
    let log_file = log_path.join("one_img.log");
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
