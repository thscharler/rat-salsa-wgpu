use crate::framework::control_queue::ControlQueue;
use crate::framework::poll_queue::PollQueue;
use crate::poll::{PollEvents, PollQuit, PollRendered, PollTasks, PollTimers, PollTokio};
use crate::tasks::Cancel;
use crate::thread_pool::ThreadPool;
use crate::timer::Timers;
use crate::tokio_tasks::TokioTasks;
use crate::{Control, RunConfig, SalsaAppContext, SalsaContext};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{Frame, Terminal};
use ratatui_wgpu::shaders::AspectPreservingDefaultPostProcessor;
use ratatui_wgpu::{Font, WgpuBackend};
use std::any::TypeId;
use std::cell::RefCell;
use std::cmp::min;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use std::{io, thread};
use winit::application::ApplicationHandler;
use winit::event::{Modifiers, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

pub(crate) mod control_queue;
mod crossterm;
mod poll_queue;

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
    mut cfg: RunConfig<Event, Error>,
) -> Result<(), Error>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send + From<(WindowEvent, Modifiers)>,
    Error: 'static + Debug + Send + From<winit::error::EventLoopError> + From<io::Error>,
{
    let event_loop = cfg.event_loop.take().expect("event-loop");

    let mut rendered = None;
    let mut quit = None;
    let mut timers = None;
    let mut tasks = None;
    let mut tokio = None;
    let mut poll = Vec::new();
    for v in cfg.poll.take().into_iter().flatten() {
        if v.as_ref().type_id() == TypeId::of::<PollRendered>() {
            rendered = Some(v);
            continue;
        } else if v.as_ref().type_id() == TypeId::of::<PollQuit>() {
            quit = Some(v);
            continue;
        } else if v.as_ref().type_id() == TypeId::of::<PollTimers>() {
            timers = v
                .as_any()
                .downcast_ref::<PollTimers>()
                .map(|t| t.get_timers());
        } else if v.as_ref().type_id() == TypeId::of::<PollTasks<Event, Error>>() {
            tasks = v
                .as_any()
                .downcast_ref::<PollTasks<Event, Error>>()
                .map(|t| t.get_tasks());
        }
        #[cfg(feature = "async")]
        if v.as_ref().type_id() == TypeId::of::<PollTokio<Event, Error>>() {
            tokio = v
                .as_any()
                .downcast_ref::<PollTokio<Event, Error>>()
                .map(|t| t.get_tasks());
        }

        poll.push(v);
    }
    let poll = Some(poll);
    let proxy = Some(event_loop.create_proxy());

    let mut app = WgpuApp {
        init,
        render,
        event,
        error,
        global,
        state,
        cfg,
        quit,
        rendered,
        timers,
        tasks,
        tokio,
        poll,
        proxy,
        poll_run: Default::default(),
        window: Default::default(),
        modifiers: Default::default(),
        terminal: Default::default(),
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

struct WgpuApp<'a, Global, State, Event, Error>
where
    Event: 'static,
    Error: 'static,
{
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

    cfg: RunConfig<Event, Error>,

    quit: Option<Box<dyn PollEvents<Event, Error> + Send>>,
    rendered: Option<Box<dyn PollEvents<Event, Error> + Send>>,
    /// Application timers.
    timers: Option<Rc<Timers>>,
    /// Background tasks.
    tasks: Option<Rc<ThreadPool<Event, Error>>>,
    /// Background tasks.
    #[cfg(feature = "async")]
    tokio: Option<Rc<TokioTasks<Event, Error>>>,

    poll: Option<Vec<Box<dyn PollEvents<Event, Error> + Send>>>,
    proxy: Option<EventLoopProxy<Result<Control<Event>, Error>>>,
    poll_run: Option<Poll>,

    window: Option<Arc<Window>>,
    modifiers: Modifiers,
    terminal: Option<
        Rc<RefCell<Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>>>>,
    >,
}

impl<'a, Global, State, Event, Error> ApplicationHandler<Result<Control<Event>, Error>>
    for WgpuApp<'a, Global, State, Event, Error>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send + From<(WindowEvent, Modifiers)>,
    Error: 'static + Debug + Send + From<io::Error>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // todo: depends on resumed() being called at most once.
        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();
        let cr_font = self.cfg.fonts.take().expect("font-loader");
        let font_ids = cr_font(&font_db);

        static FONT_DATA: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
        FONT_DATA.get_or_init(|| {
            font_ids
                .into_iter()
                .filter_map(|id| font_db.with_face_data(id, |d, _| d.to_vec()))
                .collect::<Vec<_>>()
        });
        let fonts = FONT_DATA
            .get()
            .expect("font-data")
            .iter()
            .filter_map(|d| Font::new(d))
            .collect::<Vec<_>>();

        let cr_window = self.cfg.window.take().expect("window-constructor");
        let window = Arc::new(cr_window(event_loop));

        let cr_terminal = self.cfg.term.take().expect("terminal-constructor");
        let terminal = Rc::new(RefCell::new(cr_terminal(
            window.clone(),
            fonts,
            self.cfg.font_size,
            self.cfg.bg_color,
            self.cfg.fg_color,
        )));

        self.global.set_salsa_ctx(SalsaAppContext {
            focus: Default::default(),
            count: Default::default(),
            cursor: Default::default(),
            term: Some(terminal.clone()),
            window: Some(window.clone()),
            last_render: Default::default(),
            last_event: Default::default(),
            timers: self.timers.take(),
            tasks: self.tasks.take(),
            #[cfg(feature = "async")]
            tokio: self.tokio.take(),
            queue: ControlQueue::default(),
        });

        // init state
        (self.init)(self.state, self.global).expect("init");

        // initial render
        terminal
            .borrow_mut()
            .draw(&mut |frame: &mut Frame| {
                let frame_area = frame.area();
                let ttt = SystemTime::now();

                (self.render)(frame_area, frame.buffer_mut(), self.state, self.global)
                    .expect("initial render");

                self.global
                    .salsa_ctx()
                    .last_render
                    .set(ttt.elapsed().unwrap_or_default());
                if let Some((cursor_x, cursor_y)) = self.global.salsa_ctx().cursor.get() {
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
                self.global.salsa_ctx().count.set(frame.count());
                self.global.salsa_ctx().cursor.set(None);
            })
            .expect("initial render");

        window.request_redraw();
        window.set_visible(true);

        // start poll
        let proxy = self.proxy.take().expect("proxy event-loop");
        let poll = self.poll.take().expect("poll list");
        self.poll_run = Some(start_poll(proxy, poll));

        self.window = Some(window);
        self.terminal = Some(terminal);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Result<Control<Event>, Error>) {
        process_event(self, event_loop, None, Some(event));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        process_event(self, event_loop, Some(event), None);
    }
}

fn process_event<'a, Global, State, Event, Error>(
    app: &mut WgpuApp<'a, Global, State, Event, Error>,
    event_loop: &ActiveEventLoop,
    event: Option<WindowEvent>,
    user: Option<Result<Control<Event>, Error>>,
) where
    Global: SalsaContext<Event, Error>,
    Event: 'static + From<(WindowEvent, Modifiers)>,
    Error: 'static + Debug + From<io::Error>,
{
    let Some(term) = app.terminal.as_mut() else {
        return;
    };
    let Some(window) = app.window.as_mut() else {
        return;
    };

    if let Some(WindowEvent::CloseRequested) = event {
        shutdown(app, event_loop);
        return;
    }
    if let Some(WindowEvent::ModifiersChanged(modifiers)) = event {
        app.modifiers = modifiers;
    }
    if let Some(WindowEvent::Resized(size)) = event {
        term.borrow_mut()
            .backend_mut()
            .resize(size.width, size.height);
    }

    let mut was_changed = false;
    let global = &mut *app.global;
    let state = &mut *app.state;

    if let Some(event) = event {
        let v = Ok(Control::Event((event, app.modifiers).into()));
        global.salsa_ctx().queue.push(v);
    }
    if let Some(user) = user {
        global.salsa_ctx().queue.push(user);
    }

    'ui: loop {
        // Result of event-handling.
        if let Some(ctrl) = global.salsa_ctx().queue.take() {
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
                    let r = (app.error)(e, state, global);
                    global.salsa_ctx().queue.push(r);
                }
                Ok(Control::Continue) => {}
                Ok(Control::Unchanged) => {}
                Ok(Control::Changed) => {
                    let mut r = Ok(());
                    term.borrow_mut()
                        .draw(&mut |frame: &mut Frame| {
                            let frame_area = frame.area();
                            let ttt = SystemTime::now();

                            r = (app.render)(frame_area, frame.buffer_mut(), state, global);

                            global
                                .salsa_ctx()
                                .last_render
                                .set(ttt.elapsed().unwrap_or_default());
                            if let Some((cursor_x, cursor_y)) = global.salsa_ctx().cursor.get() {
                                frame.set_cursor_position((cursor_x, cursor_y));
                            }
                            global.salsa_ctx().count.set(frame.count());
                            global.salsa_ctx().cursor.set(None);
                        })
                        .expect("draw-frame");
                    window.request_redraw();

                    match r {
                        Ok(_) => {
                            if let Some(rendered) = &mut app.rendered {
                                global.salsa_ctx().queue.push(rendered.read());
                            }
                        }
                        Err(e) => global.salsa_ctx().queue.push(Err(e)),
                    }
                }
                #[cfg(feature = "dialog")]
                Ok(Control::Close(a)) => {
                    // close probably demands a repaint.
                    global.salsa_ctx().queue.push(Ok(Control::Changed));
                    // forward event.
                    let ttt = SystemTime::now();

                    let r = (app.event)(&a, state, global);

                    global
                        .salsa_ctx()
                        .last_event
                        .set(ttt.elapsed().unwrap_or_default());
                    global.salsa_ctx().queue.push(r);
                }
                Ok(Control::Event(a)) => {
                    let ttt = SystemTime::now();
                    let r = (app.event)(&a, state, global);
                    global
                        .salsa_ctx()
                        .last_event
                        .set(ttt.elapsed().unwrap_or_default());
                    global.salsa_ctx().queue.push(r);
                }
                Ok(Control::Quit) => {
                    if let Some(quit) = &mut app.quit {
                        let Control::Event(a) = quit.read().unwrap_or(Control::Quit) else {
                            unreachable!();
                        };
                        match (app.event)(&a, state, global) {
                            Ok(Control::Quit) => { /* really quit now */ }
                            Ok(v) => global.salsa_ctx().queue.push(Ok(v)),
                            Err(e) => global.salsa_ctx().queue.push(Err(e)),
                        };
                    }
                    shutdown(app, event_loop);
                    break 'ui;
                }
            }
        }

        if global.salsa_ctx().queue.is_empty() {
            break 'ui;
        }
    }
}

fn shutdown<'a, Global, State, Event, Error>(
    app: &mut WgpuApp<'a, Global, State, Event, Error>,
    event_loop: &ActiveEventLoop,
) where
    Global: SalsaContext<Event, Error>,
    Event: 'static + From<(WindowEvent, Modifiers)>,
    Error: 'static + Debug + From<io::Error>,
{
    app.poll_run.as_mut().expect("poll_run").shutdown();
    event_loop.exit();
}

struct Poll {
    cancel: Cancel,
    join_handle: JoinHandle<()>,
}

impl Poll {
    fn shutdown(&mut self) {
        self.cancel.cancel();
        self.join_handle.thread().unpark();
    }
}

const SLEEP: u64 = 250_000; // µs
const BACKOFF: u64 = 10_000; // µs
const FAST_SLEEP: u64 = 100; // µs

fn start_poll<Event, Error>(
    event_loop: EventLoopProxy<Result<Control<Event>, Error>>,
    mut poll: Vec<Box<dyn PollEvents<Event, Error> + Send>>,
) -> Poll
where
    Event: 'static + Send,
    Error: 'static + Debug + Send,
{
    let cancel = Cancel::new();

    let t_cancel = cancel.clone();
    let join_handle = thread::spawn(move || {
        let poll_queue = PollQueue::default();
        let mut poll_sleep = Duration::from_micros(SLEEP);

        'l: loop {
            if t_cancel.is_canceled() {
                break 'l;
            }

            // The events are not processed immediately, but all
            // notifies are queued in the poll_queue.
            if poll_queue.is_empty() {
                for (n, p) in poll.iter_mut().enumerate() {
                    match p.poll() {
                        Ok(true) => {
                            poll_queue.push(n);
                        }
                        Ok(false) => {}
                        Err(e) => {
                            if event_loop.send_event(Err(e)).is_err() {
                                break 'l;
                            }
                        }
                    }
                }
            }

            // Sleep regime.
            if poll_queue.is_empty() {
                let mut t = poll_sleep;
                for p in poll.iter_mut() {
                    if let Some(timer_sleep) = p.sleep_time() {
                        t = min(timer_sleep, t);
                    }
                }
                thread::park_timeout(t);
                if poll_sleep < Duration::from_micros(SLEEP) {
                    // Back off slowly.
                    poll_sleep += Duration::from_micros(BACKOFF);
                }
            } else {
                // Shorter sleep immediately after an event.
                poll_sleep = Duration::from_micros(FAST_SLEEP);

                while let Some(p_idx) = poll_queue.take() {
                    let event = poll[p_idx].read().expect("poll fine");
                    if event_loop.send_event(Ok(event)).is_err() {
                        break 'l;
                    }
                }
            }
        }
    });

    Poll {
        cancel,
        join_handle,
    }
}
