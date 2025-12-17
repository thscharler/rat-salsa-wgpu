use crate::framework::control_queue::ControlQueue;
use crate::framework::poll_queue::PollQueue;
use crate::terminal::{Terminal, WgpuTerminal};
use crate::{Control, SalsaAppContext, SalsaContext};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use std::cell::RefCell;
use std::fmt::Debug;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use winit::application::ApplicationHandler;
use winit::event::{Modifiers, WindowEvent};
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
    modifiers: Modifiers,
    terminal: Option<Rc<RefCell<WgpuTerminal>>>,
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
    Event: 'static + From<(WindowEvent, Modifiers)>,
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
        window: Default::default(),
        modifiers: Default::default(),
        terminal: Default::default(),
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

impl<'a, Global, State, Event, Error> ApplicationHandler
    for WgpuApp<'a, Global, State, Event, Error>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + From<(WindowEvent, Modifiers)>,
    Error: 'static + Debug + From<io::Error>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .expect("create window"),
        );
        let terminal = Rc::new(RefCell::new(
            WgpuTerminal::new(window.clone()), //
        ));

        self.global.set_salsa_ctx(SalsaAppContext {
            focus: Default::default(),
            count: Default::default(),
            cursor: Default::default(),
            term: Some(terminal.clone()),
            // clear_terminal: Default::default(),
            // insert_before: Default::default(),
            last_render: Default::default(),
            last_event: Default::default(),
            // timers,
            // tasks,
            // #[cfg(feature = "async")]
            // tokio,
            queue: ControlQueue::default(),
        });

        // init state
        (self.init)(self.state, self.global).expect("init");

        // initial render
        terminal
            .borrow_mut()
            .render(&mut |frame| -> Result<(), Error> {
                let frame_area = frame.area();
                let ttt = SystemTime::now();
                (self.render)(frame_area, frame.buffer_mut(), self.state, self.global)?;
                self.global
                    .salsa_ctx()
                    .last_render
                    .set(ttt.elapsed().unwrap_or_default());
                if let Some((cursor_x, cursor_y)) = self.global.salsa_ctx().cursor.get() {
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
                self.global.salsa_ctx().count.set(frame.count());
                self.global.salsa_ctx().cursor.set(None);
                Ok(())
            })
            .expect("initial render");

        window.request_redraw();

        self.window = Some(window);
        self.terminal = Some(terminal);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(term) = self.terminal.as_mut() else {
            return;
        };
        let Some(window) = self.window.as_mut() else {
            return;
        };

        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }
        if let WindowEvent::ModifiersChanged(modifiers) = event {
            self.modifiers = modifiers;
        }
        if let WindowEvent::Resized(size) = event {
            term.borrow_mut()
                .backend_mut()
                .resize(size.width, size.height);
        }

        let mut was_changed = false;

        self.global
            .salsa_ctx()
            .queue
            .push(Ok(Control::Event((event, self.modifiers).into())));

        'ui: loop {
            // Result of event-handling.
            if let Some(ctrl) = self.global.salsa_ctx().queue.take() {
                // filter out double Changed events.
                // no need to render twice in a row.
                if matches!(ctrl, Ok(Control::Changed)) {
                    if was_changed {
                        continue;
                    }
                    was_changed = true;
                } else {
                    was_changed = false;
                }

                match ctrl {
                    Err(e) => {
                        let r = (self.error)(e, self.state, self.global);
                        self.global.salsa_ctx().queue.push(r);
                    }
                    Ok(Control::Continue) => {}
                    Ok(Control::Unchanged) => {}
                    Ok(Control::Changed) => {
                        // if self.global.salsa_ctx().clear_terminal.get() {
                        //     self.global.salsa_ctx().clear_terminal.set(false);
                        //     if let Err(e) = term.borrow_mut().clear() {
                        //         self.global.salsa_ctx().queue.push(Err(e.into()));
                        //     }
                        // }
                        // let ib = self.global.salsa_ctx().insert_before.take();
                        // if ib.height > 0 {
                        //     term.borrow_mut().insert_before(ib.height, ib.draw_fn)?;
                        // }
                        let r = term.borrow_mut().render(&mut |frame| {
                            let frame_area = frame.area();
                            let ttt = SystemTime::now();
                            (self.render)(frame_area, frame.buffer_mut(), self.state, self.global)?;
                            self.global
                                .salsa_ctx()
                                .last_render
                                .set(ttt.elapsed().unwrap_or_default());
                            if let Some((cursor_x, cursor_y)) = self.global.salsa_ctx().cursor.get()
                            {
                                frame.set_cursor_position((cursor_x, cursor_y));
                            }
                            self.global.salsa_ctx().count.set(frame.count());
                            self.global.salsa_ctx().cursor.set(None);
                            Ok(())
                        });
                        self.window.as_ref().expect("window").request_redraw();

                        match r {
                            Ok(_) => {
                                // if let Some(h) = rendered_event {
                                //     global.salsa_ctx().queue.push(poll[h].read());
                                // }
                            }
                            Err(e) => self.global.salsa_ctx().queue.push(Err(e)),
                        }
                    }
                    #[cfg(feature = "dialog")]
                    Ok(Control::Close(a)) => {
                        // close probably demands a repaint.
                        global.salsa_ctx().queue.push(Ok(Control::Changed));
                        // forward event.
                        let ttt = SystemTime::now();
                        let r = event(&a, state, global);
                        global
                            .salsa_ctx()
                            .last_event
                            .set(ttt.elapsed().unwrap_or_default());
                        global.salsa_ctx().queue.push(r);
                    }
                    Ok(Control::Event(a)) => {
                        let ttt = SystemTime::now();
                        let r = (self.event)(&a, self.state, self.global);
                        self.global
                            .salsa_ctx()
                            .last_event
                            .set(ttt.elapsed().unwrap_or_default());
                        self.global.salsa_ctx().queue.push(r);
                    }
                    Ok(Control::Quit) => {
                        // if let Some(quit) = quit {
                        //     let Control::Event(a) = poll[quit].read().unwrap_or(Control::Quit)
                        //     else {
                        //         unreachable!();
                        //     };
                        //     match event(&a, state, global) {
                        //         Ok(Control::Quit) => { /* really quit now */ }
                        //         Ok(v) => global.salsa_ctx().queue.push(Ok(v)),
                        //         Err(e) => global.salsa_ctx().queue.push(Err(e)),
                        //     };
                        // }
                        event_loop.exit();
                        break 'ui;
                    }
                }
            }

            if self.global.salsa_ctx().queue.is_empty() {
                break 'ui;
            }
        }
    }
}
