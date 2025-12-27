use anyhow::Error;
use log::error;
use rat_event::event_flow;
use rat_salsa_wgpu::event_type::CompositeWinitEvent;
use rat_salsa_wgpu::event_type::convert_winit::ConvertWinit;
use rat_salsa_wgpu::{Control, RunConfig, SalsaAppContext, SalsaContext, mock, run_tui};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Span;
use ratatui_core::widgets::Widget;
use std::fs;
use std::path::PathBuf;
use winit::event::WindowEvent;

fn main() -> Result<(), Error> {
    setup_logging()?;

    let mut global = Global::new();
    let mut state = State {
        journal: Default::default(),
    };

    run_tui(
        mock::init,
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::new(ConvertWinit::new())?
            .window_position(winit::dpi::PhysicalPosition::new(30, 30))
            .font_family("Courier New")
            .font_size(20.),
    )
}

struct Global {
    ctx: SalsaAppContext<AppEvent, Error>,
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
    pub fn new() -> Self {
        Self {
            ctx: Default::default(),
        }
    }
}

#[allow(unused)]
enum AppEvent {
    NoOp,
    WEvent(CompositeWinitEvent),
    CtEvent(crossterm::event::Event),
}

impl From<crossterm::event::Event> for AppEvent {
    fn from(value: crossterm::event::Event) -> Self {
        AppEvent::CtEvent(value)
    }
}

impl From<CompositeWinitEvent> for AppEvent {
    fn from(value: CompositeWinitEvent) -> Self {
        AppEvent::WEvent(value)
    }
}

struct State {
    journal: Vec<WindowEvent>,
}

fn render(area: Rect, buf: &mut Buffer, state: &mut State, _ctx: &mut Global) -> Result<(), Error> {
    if state.journal.len() > 0 {
        let mut r = 0;
        for event in state.journal.iter().rev() {
            if let Some(msg) = match event {
                WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            logical_key,
                            text,
                            location,
                            state,
                            repeat,
                            ..
                        },
                    ..
                } => Some(format!(
                    "key {:?} {:?} {:?} {:?} {:?}",
                    logical_key, text, location, state, repeat
                )),
                WindowEvent::ModifiersChanged(v) => Some(format!("modifiers {:?}", v)),
                WindowEvent::Ime(v) => Some(format!("ime {:?}", v)),
                _ => None,
            } {
                let row_area = Rect::new(area.x, area.y + r, area.width, 1);
                r += 1;

                Span::from(msg).render(row_area, buf);

                if area.y + r >= area.height {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn event(
    event: &AppEvent,
    state: &mut State,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    event_flow!(match event {
        AppEvent::WEvent(k) => {
            state.journal.push(k.event.clone());
            Control::Changed
        }
        _ => Control::Continue,
    });

    Ok(Control::Continue)
}

fn error(event: Error, _state: &mut State, _ctx: &mut Global) -> Result<Control<AppEvent>, Error> {
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
