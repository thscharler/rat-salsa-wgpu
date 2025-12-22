use anyhow::Error;
use log::{debug, error};
use rat_event::{ct_event, try_flow};
use rat_salsa_wgpu::event::{QuitEvent, RenderedEvent};
use rat_salsa_wgpu::event_type::CompositeWinitEvent;
use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrossterm;
use rat_salsa_wgpu::font_data::FontData;
use rat_salsa_wgpu::poll::{PollTasks, PollTimers};
use rat_salsa_wgpu::timer::TimeOut;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::create_salsa_theme;
use rat_theme4::theme::SalsaTheme;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::Widget;
use std::fs;
use std::path::PathBuf;

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Imperial Shell");
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
            .window_position(winit::dpi::PhysicalPosition::new(30, 30))
            // .font_family("JetBrainsMono Nerd Font Mono")
            // .font_family("Courier New")
            .font_size(20.)
            .poll(PollTimers::new())
            .poll(PollTasks::new(2)),
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
        let font_db = FontData.font_db();
        let mut fonts = font_db
            .faces()
            .filter_map(|info| {
                if info.monospaced {
                    if let Some((family, _)) = info.families.first() {
                        if family != "Lucida Console"
                            && family != "NSimSun"
                            && family != "SimSun-ExtB"
                            && family != "SimSun-ExtG"
                        {
                            Some(family.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        fonts.sort();
        fonts.dedup();
        fonts.push("<Fallback>".to_string());

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

impl From<RenderedEvent> for AppEvent {
    fn from(_: RenderedEvent) -> Self {
        AppEvent::Rendered
    }
}

impl From<QuitEvent> for AppEvent {
    fn from(_: QuitEvent) -> Self {
        AppEvent::Quit
    }
}

impl From<TimeOut> for AppEvent {
    fn from(value: TimeOut) -> Self {
        Self::TimeOut(value)
    }
}

#[derive(Debug, Default)]
pub struct Minimal {
    pub font_idx: usize,
}

pub fn init(_state: &mut Minimal, _ctx: &mut Global) -> Result<(), Error> {
    Ok(())
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    _state: &mut Minimal,
    ctx: &mut Global,
) -> Result<(), Error> {
    let layout = Layout::vertical([
        Constraint::Fill(1), //
    ])
    .split(area);

    Text::from_iter([
        Line::from(" ...").style(Style::new()),
        Line::from(" ..").style(Style::new()),
        Line::from(" .").style(Style::new()),
        Line::from(ctx.font_family()).style(Style::new()),
        Line::from(ctx.font_size().to_string()).style(Style::new()),
    ])
    .render(layout[0], buf);

    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    debug!("event {:?}", event);
    if let AppEvent::CtEvent(event) = event {
        try_flow!(match &event {
            ct_event!(resized) => {
                Control::Changed
            }
            ct_event!(key press CONTROL-'q') => Control::Quit,
            ct_event!(keycode press F(1)) => {
                state.font_idx = (state.font_idx + 1) % ctx.fonts.len();
                let font = ctx.fonts[state.font_idx].as_str();
                debug!("set_font_family {:?}", font);
                ctx.set_font_family(font);
                Control::Changed
            }
            _ => Control::Continue,
        });
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
    let log_file = log_path.join("log.log");
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
