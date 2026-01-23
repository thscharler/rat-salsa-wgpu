#![allow(text_direction_codepoint_in_literal)]

use anyhow::Error;
use image::ImageReader;
use log::error;
use rat_event::{HandleEvent, Regular, ct_event, event_flow};
use rat_salsa_wgpu::event_type::CompositeWinitEvent;
use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrossterm;
use rat_salsa_wgpu::poll::PollBlink;
use rat_salsa_wgpu::timer::TimeOut;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, create_salsa_theme};
use rat_wgpu::font::FontData;
use rat_wgpu::image::{ImageArg, ImageBuffer, ImageFit, ImageHandle};
use rat_widget::view::{View, ViewState};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
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
    pub view: ViewState,

    pub fit: ImageFit,
    pub img1: ImageHandle,
    pub img2: ImageHandle,
}

pub fn init(state: &mut Minimal, ctx: &mut Global) -> Result<(), Error> {
    let image = ImageReader::open("rat-salsa-wgpu/utils/tumblr_ms5hgqXaNF1r1k4q7o1_r1_500.gif")?;
    let image = image.decode()?;
    let rgba = image.to_rgba8();
    let rgba = rgba.into_flat_samples();

    let (_c, w, h) = rgba.extents();

    state.img1 =
        ctx.terminal()
            .borrow_mut()
            .backend_mut()
            .add_image(&rgba.samples, w as u32, h as u32);

    let image = ImageReader::open("rat-salsa-wgpu/utils/marv.jpg")?;
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
    let ibuf = ctx.image_buffer();
    let mut ibuf = ibuf.lock().expect("lock");

    buf.set_style(area, ctx.theme.style_style(Style::CONTAINER_BASE));
    ctx.set_bg_color(ctx.theme.style_style(Style::CONTAINER_BASE).bg.expect("bg"));

    render_view(buf, &mut ibuf, state, ctx);

    Ok(())
}

fn render_view(buf: &mut Buffer, ibuf: &mut ImageBuffer, state: &mut Minimal, ctx: &mut Global) {
    let mut view_img = ibuf.derive(Rect::new(0, 0, 40, 100));
    let mut view = View::new()
        .layout(Rect::new(0, 0, 40, 100))
        .style(ctx.theme.p.red(0))
        .into_buffer(Rect::new(3, 3, 20, 10), &mut state.view);

    let area = Rect::new(0, 0, 10, 10);
    view_img.render(&state.img1, area, ImageArg::default().fit(state.fit));
    view.render_widget(Block::bordered(), area);

    let area = Rect::new(10, 0, 30, 10);
    view_img.render(&state.img1, area, ImageArg::default().fit(state.fit));
    view.render_widget(Block::bordered(), area);

    let area = Rect::new(0, 10, 10, 10);
    view_img.render(&state.img2, area, ImageArg::default().fit(state.fit));
    view.render_widget(Block::bordered(), area);

    let area = Rect::new(10, 10, 30, 10);
    view_img.render(&state.img2, area, ImageArg::default().fit(state.fit));
    view.render_widget(Block::bordered(), area);

    //
    ibuf.append(view_img, view.shift(), Rect::new(3, 3, 20, 10));
    view.finish(buf, &mut state.view);
}

#[allow(dead_code)]
fn render_basic(buf: &mut Buffer, ibuf: &mut ImageBuffer, state: &mut Minimal, _ctx: &mut Global) {
    let area = Rect::new(0, 0, 10, 10);
    ibuf.render(&state.img1, area, ImageArg::default().fit(state.fit));
    Block::bordered().render(area, buf);

    let area = Rect::new(10, 0, 30, 10);
    ibuf.render(&state.img1, area, ImageArg::default().fit(state.fit));
    Block::bordered().render(area, buf);

    let area = Rect::new(0, 10, 10, 10);
    ibuf.render(&state.img2, area, ImageArg::default().fit(state.fit));
    Block::bordered().render(area, buf);

    let area = Rect::new(10, 10, 30, 10);
    ibuf.render(&state.img2, area, ImageArg::default().fit(state.fit));
    Block::bordered().render(area, buf);
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::CtEvent(event) = event {
        match &event {
            ct_event!(resized) => event_flow!(Control::Changed),
            ct_event!(key press CONTROL-'q') => event_flow!(Control::Quit),
            ct_event!(key press '4') => event_flow!({
                state.fit = match state.fit {
                    ImageFit::Fill => ImageFit::FitStart,
                    ImageFit::FitStart => ImageFit::FitCenter,
                    ImageFit::FitCenter => ImageFit::FitEnd,
                    ImageFit::FitEnd => ImageFit::HorizontalStart,
                    ImageFit::HorizontalStart => ImageFit::HorizontalCenter,
                    ImageFit::HorizontalCenter => ImageFit::HorizontalEnd,
                    ImageFit::HorizontalEnd => ImageFit::FitVerticalStart,
                    ImageFit::FitVerticalStart => ImageFit::FitVerticalCenter,
                    ImageFit::FitVerticalCenter => ImageFit::FitVerticalEnd,
                    ImageFit::FitVerticalEnd => ImageFit::Fill,
                };
                Control::Changed
            }),
            ct_event!(key press '5') => event_flow!({
                state.fit = match state.fit {
                    ImageFit::Fill => ImageFit::FitVerticalEnd,
                    ImageFit::FitStart => ImageFit::Fill,
                    ImageFit::FitCenter => ImageFit::FitStart,
                    ImageFit::FitEnd => ImageFit::FitCenter,
                    ImageFit::HorizontalStart => ImageFit::FitEnd,
                    ImageFit::HorizontalCenter => ImageFit::HorizontalStart,
                    ImageFit::HorizontalEnd => ImageFit::HorizontalCenter,
                    ImageFit::FitVerticalStart => ImageFit::HorizontalEnd,
                    ImageFit::FitVerticalCenter => ImageFit::FitVerticalStart,
                    ImageFit::FitVerticalEnd => ImageFit::FitVerticalCenter,
                };
                Control::Changed
            }),

            _ => {}
        }

        event_flow!(state.view.handle(event, Regular));
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
