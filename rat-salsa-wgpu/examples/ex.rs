use anyhow::Error;
use rat_salsa_wgpu::run_wgpu;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub fn main() {
    let mut global = Global {
        ctx: Default::default(),
    };
    let mut state = Minimal::default();

    run_wgpu(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        (),
    )
    .expect("fine");
}

pub struct Global {
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

#[derive(Debug)]
pub enum AppEvent {
    Event(),
}

#[derive(Debug, Default)]
pub struct Minimal {}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<(), Error> {
    Ok(())
}

pub fn init(state: &mut Minimal, ctx: &mut Global) -> Result<(), Error> {
    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    Ok(Control::Continue)
}

pub fn error(
    event: Error,
    state: &mut Minimal,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    Ok(Control::Continue)
}
