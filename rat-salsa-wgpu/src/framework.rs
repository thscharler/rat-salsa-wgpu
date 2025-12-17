use crate::terminal::{Terminal, WgpuTerminal};
use crate::{Control, SalsaContext};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use std::fmt::Debug;
use std::io;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

pub(crate) mod control_queue;
mod poll_queue;

const SLEEP: u64 = 250_000; // µs
const BACKOFF: u64 = 10_000; // µs
const FAST_SLEEP: u64 = 100; // µs

struct WgpuApp<'a, Global, State, Event, Error> {
    init: fn(
        state: &mut State, //
        ctx: &mut Global,
    ) -> Result<(), Error>,
    render:
        fn(area: Rect, buf: &mut Buffer, state: &mut State, ctx: &mut Global) -> Result<(), Error>,
    event: fn(
        event: &Event, //
        state: &mut State,
        ctx: &mut Global,
    ) -> Result<Control<Event>, Error>,
    error: fn(
        error: Error, //
        state: &mut State,
        ctx: &mut Global,
    ) -> Result<Control<Event>, Error>,
    global: &'a mut Global,
    state: &'a mut State,
    cfg: (),

    window: Option<Arc<Window>>,
    terminal: Option<WgpuTerminal>,
}

pub fn run_wgpu<Global, State, Event, Error>(
    init: fn(
        state: &mut State, //
        ctx: &mut Global,
    ) -> Result<(), Error>,
    render: fn(
        area: Rect,
        buf: &mut Buffer,
        state: &mut State,
        ctx: &mut Global,
    ) -> Result<(), Error>,
    event: fn(
        event: &Event, //
        state: &mut State,
        ctx: &mut Global,
    ) -> Result<Control<Event>, Error>,
    error: fn(
        error: Error, //
        state: &mut State,
        ctx: &mut Global,
    ) -> Result<Control<Event>, Error>,
    global: &mut Global,
    state: &mut State,
    cfg: (),
) -> Result<(), Error>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static,
    Error: 'static + Debug + From<winit::error::EventLoopError> + From<io::Error>,
{
    let event_loop = EventLoop::builder().build()?;
    let mut app = WgpuApp {
        init,
        render,
        event,
        error,
        global,
        state,
        cfg,
        window: None,
        terminal: None,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

impl<'a, Global, State, Event, Error> ApplicationHandler
    for WgpuApp<'a, Global, State, Event, Error>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static,
    Error: 'static + Debug + From<io::Error>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        ));
        let window = self.window.as_ref().expect("window");

        self.terminal = Some(WgpuTerminal::new(window.clone()));

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }
        let Some(terminal) = self.terminal.as_mut() else {
            return;
        };
        if let WindowEvent::Resized(size) = event {
            terminal.backend_mut().resize(size.width, size.height);
        }

        terminal
            .render(&mut |f| -> Result<(), Error> {
                f.render_widget(
                    Paragraph::new(Line::from(vec!["Hello World! 🦀🚀".bold().italic()]))
                        .block(Block::bordered()),
                    f.area(),
                );
                Ok(())
            })
            .expect("render");

        self.window.as_ref().expect("window").request_redraw();
    }
}
