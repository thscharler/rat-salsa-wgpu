use crate::_private::NonExhaustive;
use crate::event_type::ConvertEvent;
use crate::font_data::FontData;
use crate::framework::control_queue::ControlQueue;
use crate::framework::poll_queue::PollQueue;
use crate::poll::{PollEvents, PollQuit, PollRendered, PollTasks, PollTimers, PollTokio};
use crate::run_config::TermInit;
use crate::tasks::Cancel;
use crate::thread_pool::ThreadPool;
use crate::timer::Timers;
use crate::tokio_tasks::TokioTasks;
use crate::{Control, RunConfig, SalsaAppContext, SalsaContext};
use log::{debug, info};
use rat_widget::text::cursor::CursorType;
use ratatui_core::backend::{Backend, WindowSize};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Color;
use ratatui_core::terminal::{Frame, Terminal};
use ratatui_wgpu::{CursorStyle, Font, WgpuBackend};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::cmp::min;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use std::{io, mem, thread};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

pub(crate) mod control_queue;
mod poll_queue;

/// Start the UI and run the event-loop.
///
/// This is the same as in rat-salsa, just using a different RunConfig.
///
/// ```rust no_run
/// use anyhow::{anyhow, Error};
/// use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrossterm;
/// use rat_salsa_wgpu::{Control, RunConfig, SalsaAppContext, mock, run_tui};
/// use rat_widget::event::ct_event;
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use ratatui::style::Stylize;
/// use ratatui::text::{Line, Span};
/// use ratatui::widgets::Widget;
///
/// fn main() -> Result<(), Error> {
///     run_tui(
///         mock::init,
///         render,
///         event,
///         error,
///         &mut Global::default(),
///         &mut Ultra,
///         RunConfig::new(ConvertCrossterm::default())?,
///     )
/// }
///
/// #[derive(Debug, Default)]
/// pub struct Global {
///     ctx: SalsaAppContext<UltraEvent, Error>,
///     pub err_cnt: u32,
///     pub err_msg: String,
/// }
///
/// impl SalsaContext<UltraEvent, Error> for Global {
///     fn set_salsa_ctx(&mut self, app_ctx: SalsaAppContext<UltraEvent, Error>) {
///         self.ctx = app_ctx;
///     }
///
///     fn salsa_ctx(&self) -> &SalsaAppContext<UltraEvent, Error> {
///         &self.ctx
///     }
/// }
///
/// #[derive(Debug, PartialEq, Eq, Clone)]
/// pub enum UltraEvent {
///     Event(crossterm::event::Event),
/// }
///
/// impl From<crossterm::event::Event> for UltraEvent {
///     fn from(value: crossterm::event::Event) -> Self {
///         Self::Event(value)
///     }
/// }
///
/// pub struct Ultra;
///
/// fn render(area: Rect, buf: &mut Buffer, _state: &mut Ultra, ctx: &mut Global) -> Result<(), Error> {
///     Line::from_iter([Span::from("'q' to quit, 'e' for error, 'r' for repair")])
///         .render(Rect::new(area.x, area.y, area.width, 1), buf);
///     Line::from_iter([
///         Span::from("Hello world!").green(),
///         Span::from(" Status: "),
///         if ctx.err_cnt > 0 {
///             Span::from(&ctx.err_msg).red().underlined()
///         } else {
///             Span::from(&ctx.err_msg).cyan().underlined()
///         },
///     ])
///     .render(Rect::new(area.x, area.y + 2, area.width, 1), buf);
///     Ok(())
/// }
///
/// fn event(
///     event: &UltraEvent,
///     _state: &mut Ultra,
///     ctx: &mut Global,
/// ) -> Result<Control<UltraEvent>, Error> {
///     match event {
///         UltraEvent::Event(event) => match event {
///             ct_event!(key press 'q') => Ok(Control::Quit),
///             ct_event!(key press 'e') => return Err(anyhow!("An error occured.")),
///             ct_event!(key press 'r') => {
///                 if ctx.err_cnt > 1 {
///                     ctx.err_cnt -= 1;
///                     ctx.err_msg = format!("#{}# One error repaired.", ctx.err_cnt).to_string();
///                 } else if ctx.err_cnt == 1 {
///                     ctx.err_cnt -= 1;
///                     ctx.err_msg = "All within norms.".to_string();
///                 } else {
///                     ctx.err_cnt = 1;
///                     ctx.err_msg = format!("#{}# Over-repaired.", ctx.err_cnt).to_string();
///                 }
///                 Ok(Control::Changed)
///             }
///             _ => Ok(Control::Continue),
///         },
///     }
/// }
///
/// fn error(event: Error, _state: &mut Ultra, ctx: &mut Global) -> Result<Control<UltraEvent>, Error> {
///     ctx.err_cnt += 1;
///     ctx.err_msg = format!("#{}# {}", ctx.err_cnt, event).to_string();
///     Ok(Control::Changed)
/// }
/// ```
///
/// Maybe `templates/minimal.rs` is more useful.
///
pub fn run_tui<Global, State, Event, Error>(
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
    cfg: RunConfig<Event, Error>,
) -> Result<(), Error>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<winit::error::EventLoopError> + From<io::Error>,
{
    // rat_widget::text::cursor::set_cursor_type(CursorType::RenderedCursor);

    let RunConfig {
        event_loop,
        event_type,
        cr_fonts,
        fallback_font,
        font_family,
        font_size,
        symbol_font,
        emoji_font,
        bg_color,
        fg_color,
        cur_style,
        cur_blink,
        cur_color,
        rapid_blink,
        slow_blink,
        win_attr,
        cr_window,
        cr_term,
        poll,
    } = cfg;

    let mut rendered_event = None;
    let mut quit_event = None;
    let mut timers_ctrl = None;
    let mut tasks_ctrl = None;
    let mut tokio_ctrl = None;
    let poll = {
        let mut tmp = Vec::new();
        for v in poll.into_iter() {
            if v.as_ref().type_id() == TypeId::of::<PollRendered>() {
                rendered_event = Some(v);
                continue;
            } else if v.as_ref().type_id() == TypeId::of::<PollQuit>() {
                quit_event = Some(v);
                continue;
            } else if v.as_ref().type_id() == TypeId::of::<PollTimers>() {
                timers_ctrl = v
                    .as_any()
                    .downcast_ref::<PollTimers>()
                    .map(|t| t.get_timers());
            } else if v.as_ref().type_id() == TypeId::of::<PollTasks<Event, Error>>() {
                tasks_ctrl = v
                    .as_any()
                    .downcast_ref::<PollTasks<Event, Error>>()
                    .map(|t| t.get_tasks());
            }
            #[cfg(feature = "async")]
            if v.as_ref().type_id() == TypeId::of::<PollTokio<Event, Error>>() {
                tokio_ctrl = v
                    .as_any()
                    .downcast_ref::<PollTokio<Event, Error>>()
                    .map(|t| t.get_tasks());
            }

            tmp.push(v);
        }
        tmp
    };

    let proxy = event_loop.create_proxy();

    let mut app = WgpuApp::Startup(Startup {
        init,
        render,
        event,
        error,
        global,
        state,
        cr_fonts,
        fallback_font,
        font_family,
        font_size,
        symbol_font,
        emoji_font,
        bg_color,
        fg_color,
        rapid_blink,
        slow_blink,
        cur_style,
        cur_blink,
        cur_color,
        win_attr,
        cr_window,
        cr_term,
        event_type,
        quit_event,
        rendered_event,
        timers_ctrl,
        tasks_ctrl,
        tokio_ctrl,
        poll,
        proxy,
    });

    event_loop.run_app(&mut app)?;

    Ok(())
}

struct Startup<'a, Global, State, Event, Error>
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

    /// font loading callback
    cr_fonts: Box<dyn FnOnce(&fontdb::Database) -> Vec<fontdb::ID> + 'static>,
    fallback_font: Option<(String, Font<'static>)>,
    font_family: Option<String>,
    font_size: Option<f64>,
    symbol_font: Option<Font<'static>>,
    emoji_font: Option<Font<'static>>,
    bg_color: Color,
    fg_color: Color,
    rapid_blink: u8,
    slow_blink: u8,
    /// terminal cursor
    cur_style: CursorStyle,
    cur_blink: u8,
    cur_color: Color,

    /// window callback
    win_attr: WindowAttributes,
    cr_window: Box<dyn FnOnce(&ActiveEventLoop, WindowAttributes) -> Window>,

    /// terminal callback
    cr_term: Box<dyn FnOnce(TermInit) -> Terminal<WgpuBackend<'static, 'static>>>,

    event_type: Box<dyn ConvertEvent<Event>>,
    quit_event: Option<Box<dyn PollEvents<Event, Error> + Send>>,
    rendered_event: Option<Box<dyn PollEvents<Event, Error> + Send>>,

    /// Application timers.
    timers_ctrl: Option<Arc<Timers>>,
    /// Background tasks.
    tasks_ctrl: Option<Arc<ThreadPool<Event, Error>>>,
    /// Background tasks.
    #[cfg(feature = "async")]
    tokio_ctrl: Option<Arc<TokioTasks<Event, Error>>>,

    poll: Vec<Box<dyn PollEvents<Event, Error> + Send>>,

    proxy: EventLoopProxy<Result<Control<Event>, Error>>,
}

struct Running<'a, Global, State, Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    render:
        fn(area: Rect, buf: &mut Buffer, state: &mut State, ctx: &mut Global) -> Result<(), Error>,
    event: fn(event: &Event, state: &mut State, ctx: &mut Global) -> Result<Control<Event>, Error>,
    error: fn(error: Error, state: &mut State, ctx: &mut Global) -> Result<Control<Event>, Error>,

    global: &'a mut Global,
    state: &'a mut State,

    event_type: Box<dyn ConvertEvent<Event>>,
    quit_event: Option<Box<dyn PollEvents<Event, Error> + Send>>,
    rendered_event: Option<Box<dyn PollEvents<Event, Error> + Send>>,

    poll: Poll,

    window: Option<Arc<Window>>,
    window_size: WindowSize,
    terminal: Option<Rc<RefCell<Terminal<WgpuBackend<'static, 'static>>>>>,
}

enum WgpuApp<'a, Global, State, Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    Invalid,
    Startup(Startup<'a, Global, State, Event, Error>),
    Running(Running<'a, Global, State, Event, Error>),
}

impl<'a, Global, State, Event, Error> ApplicationHandler<Result<Control<Event>, Error>>
    for WgpuApp<'a, Global, State, Event, Error>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        initialize_terminal(self, event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Result<Control<Event>, Error>) {
        process_event(self, event_loop, None, Some(event));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        process_event(self, event_loop, Some(event), None);
    }
}

fn initialize_terminal<'a, Global, State, Event, Error>(
    app: &mut WgpuApp<'a, Global, State, Event, Error>,
    event_loop: &ActiveEventLoop,
) where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    if !matches!(app, WgpuApp::Startup(_)) {
        panic!("expected startup state");
    }

    let WgpuApp::Startup(Startup {
        init,
        render,
        event,
        error,
        global,
        state,
        cr_fonts,
        fallback_font,
        font_family,
        font_size,
        symbol_font,
        emoji_font,
        bg_color,
        fg_color,
        rapid_blink,
        slow_blink,
        cur_style,
        cur_blink,
        cur_color,
        win_attr,
        cr_window,
        cr_term,
        mut event_type,
        quit_event,
        rendered_event,
        timers_ctrl,
        tasks_ctrl,
        tokio_ctrl,
        poll,
        proxy,
    }) = mem::replace(app, WgpuApp::Invalid)
    else {
        panic!()
    };

    let (fallback_family, fallback_font) =
        if let Some((fallback_family, fallback_font)) = fallback_font {
            (fallback_family, Some(fallback_font))
        } else {
            (String::default(), None)
        };

    let font_size = font_size.unwrap_or(22.0);
    let font_ids = cr_fonts(FontData.font_db());
    let window = Arc::new(cr_window(event_loop, win_attr));

    let font_size_px = (font_size * window.scale_factor()).round() as u32;
    let font_family = font_family.unwrap_or(fallback_family);

    // setup fonts
    let mut fallback_fonts = Vec::new();
    if let Some(font) = fallback_font.clone() {
        fallback_fonts.push(font);
    }
    if let Some(font) = symbol_font {
        fallback_fonts.push(font);
    }
    if let Some(font) = emoji_font {
        fallback_fonts.push(font);
    }

    let mut fonts = font_ids
        .iter()
        .filter_map(|id| FontData.load_font(*id))
        .collect::<Vec<_>>();
    if fonts.is_empty() {
        if let Some(fallback_font) = fallback_font {
            fonts.push(fallback_font);
        } else {
            panic!("need at least one valid font or a fallback font");
        }
    }

    let terminal = Rc::new(RefCell::new(cr_term(TermInit {
        fallback_fonts: fallback_fonts.clone(),
        fonts,
        font_size_px,
        window: window.clone(),
        bg_color,
        fg_color,
        rapid_blink,
        slow_blink,
        cur_style,
        cur_blink,
        cur_color,
        non_exhaustive: NonExhaustive,
    })));

    // window-size can be determined when we have the fonts installed.
    let window_size = terminal
        .borrow_mut()
        .backend_mut()
        .window_size()
        .expect("window_size");
    event_type.set_window_size(window_size);

    // create other event-polling
    let (poll_start, poll) = create_poll(proxy, poll);

    global.set_salsa_ctx(SalsaAppContext {
        focus: Default::default(),
        count: Default::default(),
        cursor: Default::default(),
        term: RefCell::new(Some(terminal.clone())),
        clear_terminal: Default::default(),
        last_render: Default::default(),
        last_event: Default::default(),
        timers: timers_ctrl,
        tasks: tasks_ctrl,
        tokio: tokio_ctrl,
        queue: ControlQueue::default(),
        window: RefCell::new(Some(window.clone())),
        font_changed: Default::default(),
        font_size_changed: Default::default(),
        font_ids: RefCell::new(font_ids),
        font_family: RefCell::new(font_family),
        font_size: Cell::new(font_size),
    });

    let run_state = Running {
        render,
        event,
        error,
        global,
        state,
        event_type,
        quit_event,
        rendered_event,
        poll,
        window: Some(window),
        window_size,
        terminal: Some(terminal),
    };

    // init state
    init(run_state.state, run_state.global).expect("init");

    // initial render
    run_state.window.as_ref().expect("window").set_visible(true);
    run_state.window.as_ref().expect("window").request_redraw();

    // set up running state.
    *app = WgpuApp::Running(run_state);

    // now start polling
    poll_start.start();
}

fn process_event<'a, Global, State, Event, Error>(
    app: &mut WgpuApp<'a, Global, State, Event, Error>,
    event_loop: &ActiveEventLoop,
    mut event: Option<WindowEvent>,
    user: Option<Result<Control<Event>, Error>>,
) where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    let WgpuApp::Running(app) = app else {
        panic!("not initialized");
    };

    if let Some(WindowEvent::Destroyed) = event {
        info!("window destroyed. exit event-loop.");
        event_loop.exit();
        return;
    }
    if app.terminal.is_none() || app.window.is_none() {
        info!("skip event during shutdown.");
        return;
    }

    if let Some(event) = &event {
        app.event_type.update_state(event);
    }
    if let Some(WindowEvent::CloseRequested) = event {
        app.global.salsa_ctx().queue.push(Ok(Control::Quit));
        event = None;
    }
    if let Some(WindowEvent::RedrawRequested) = event {
        app.global.salsa_ctx().queue.push(Ok(Control::Changed));
        event = None;
    }
    if let Some(WindowEvent::Resized(size)) = event {
        resize(app, size);
        app.terminal
            .as_ref()
            .expect("terminal")
            .borrow_mut()
            .clear()
            .expect("clear terminal");
        if let Some(event) = resized_event(app) {
            app.global.salsa_ctx().queue.push(Ok(Control::Event(event)));
        } else {
            app.global.salsa_ctx().queue.push(Ok(Control::Changed));
        }
        event = None;
    }
    if let Some(WindowEvent::ScaleFactorChanged { .. }) = event {
        app.global.salsa_ctx().font_size_changed.set(true);
        event = None;
    }
    // font scaling
    if app.event_type.state().ctrl_pressed() {
        if let Some(WindowEvent::MouseWheel {
            delta: MouseScrollDelta::LineDelta(_, dy),
            ..
        }) = event
        {
            if dy > 0.0 {
                if app.global.salsa_ctx().font_size.get() > 7.0 {
                    app.global.salsa_ctx().font_size.update(|v| v - 1.0);
                }
            } else {
                app.global.salsa_ctx().font_size.update(|v| v + 1.0);
            }
            app.global.salsa_ctx().font_size_changed.set(true);
            event = None;
        }
    }
    if app.global.salsa_ctx().clear_terminal.get() {
        app.terminal
            .as_ref()
            .expect("terminal")
            .borrow_mut()
            .clear()
            .expect("clear terminal");
        app.global.salsa_ctx().clear_terminal.set(false);
    }
    if app.global.salsa_ctx().font_changed.get() {
        // reload backend font
        reload_fonts(app);
        app.terminal
            .as_ref()
            .expect("terminal")
            .borrow_mut()
            .clear()
            .expect("clear terminal");
        if let Some(event) = resized_event(app) {
            app.global.salsa_ctx().queue.push(Ok(Control::Event(event)));
        } else {
            app.global.salsa_ctx().queue.push(Ok(Control::Changed));
        }
        app.global.salsa_ctx().font_changed.set(false);
        app.global.salsa_ctx().font_size_changed.set(false);
    }
    if app.global.salsa_ctx().font_size_changed.get() {
        // reload backend
        change_font_size(app);
        app.terminal
            .as_ref()
            .expect("terminal")
            .borrow_mut()
            .clear()
            .expect("clear terminal");
        if let Some(event) = resized_event(app) {
            app.global.salsa_ctx().queue.push(Ok(Control::Event(event)));
        } else {
            app.global.salsa_ctx().queue.push(Ok(Control::Changed));
        }
        app.global.salsa_ctx().font_size_changed.set(false);
    }

    if let Some(event) = event {
        if let Some(event) = app.event_type.convert(event) {
            app.global
                .salsa_ctx()
                .queue
                .push(Ok(Control::Event(event.into())));
        } else {
            // noop
        }
    }
    if let Some(user) = user {
        app.global.salsa_ctx().queue.push(user);
    }

    let mut was_changed = false;
    'ui: loop {
        // panic on worker panic
        if let Some(tasks) = &app.global.salsa_ctx().tasks {
            if !tasks.check_liveness() {
                dbg!("worker panicked");
                shutdown(app, event_loop);
                break 'ui;
            }
        }

        // Result of event-handling.
        if let Some(ctrl) = app.global.salsa_ctx().queue.take() {
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
                    let r = (app.error)(e, app.state, app.global);
                    app.global.salsa_ctx().queue.push(r);
                }
                Ok(Control::Continue) => {}
                Ok(Control::Unchanged) => {}
                Ok(Control::Changed) => {
                    render_tui(app);
                }
                #[cfg(feature = "dialog")]
                Ok(Control::Close(a)) => {
                    // close probably demands a repaint.
                    app.global.salsa_ctx().queue.push(Ok(Control::Event(a)));
                    app.global.salsa_ctx().queue.push(Ok(Control::Changed));
                }
                Ok(Control::Event(a)) => {
                    let ttt = SystemTime::now();
                    let r = (app.event)(&a, app.state, app.global);
                    app.global
                        .salsa_ctx()
                        .last_event
                        .set(ttt.elapsed().unwrap_or_default());
                    app.global.salsa_ctx().queue.push(r);
                }
                Ok(Control::Quit) => {
                    if let Some(quit) = &mut app.quit_event {
                        match quit.read() {
                            Ok(Control::Event(a)) => {
                                match (app.event)(&a, app.state, app.global) {
                                    Ok(Control::Quit) => { /* really quit now */ }
                                    v => {
                                        app.global.salsa_ctx().queue.push(v);
                                        continue;
                                    }
                                }
                            }
                            Err(_) => unreachable!(),
                            Ok(_) => unreachable!(),
                        }
                    }
                    shutdown(app, event_loop);
                    break 'ui;
                }
                Ok(Control::Blink) => {
                    debug!("goblink");
                    app.terminal
                        .as_ref()
                        .expect("terminal")
                        .borrow_mut()
                        .backend_mut()
                        .blink();
                }
            }
        }

        if app.global.salsa_ctx().queue.is_empty() {
            break 'ui;
        }
    }
}

fn render_tui<'a, Global, State, Event, Error>(app: &mut Running<'a, Global, State, Event, Error>)
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    let mut r = Ok(());
    app.terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .draw(&mut |frame: &mut Frame| {
            let frame_area = frame.area();
            let ttt = SystemTime::now();

            r = (app.render)(frame_area, frame.buffer_mut(), app.state, app.global);

            app.global
                .salsa_ctx()
                .last_render
                .set(ttt.elapsed().unwrap_or_default());
            if let Some((cursor_x, cursor_y)) = app.global.salsa_ctx().cursor.get() {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
            app.global.salsa_ctx().count.set(frame.count());
            app.global.salsa_ctx().cursor.set(None);
        })
        .expect("draw-frame");

    match r {
        Ok(_) => {
            if let Some(rendered) = &mut app.rendered_event {
                app.global.salsa_ctx().queue.push(rendered.read());
            }
        }
        Err(e) => app.global.salsa_ctx().queue.push(Err(e)),
    }
}

fn reload_fonts<'a, Global, State, Event, Error>(app: &mut Running<'a, Global, State, Event, Error>)
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    let font_vec = app
        .global
        .salsa_ctx()
        .font_ids
        .borrow()
        .iter()
        .filter_map(|id| FontData.load_font(*id))
        .collect::<Vec<_>>();
    app.terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .backend_mut()
        .update_font_vec(font_vec);

    let font_size_px = (app.global.salsa_ctx().font_size.get()
        * app.window.as_ref().expect("window").scale_factor())
    .round() as u32;
    app.terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .backend_mut()
        .update_font_size(font_size_px);

    // only a resize of the backend works.
    // and only a really extreme shrink removes all the artifacts, it seems ...
    // let lsize = app.window_size.pixels;
    // app.terminal //
    //     .borrow_mut()
    //     .backend_mut()
    //     .resize(1, 1);
    // app.terminal
    //     .borrow_mut()
    //     .backend_mut()
    //     .resize(lsize.width as u32, lsize.height as u32);

    app.window_size = app
        .terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .backend_mut()
        .window_size()
        .expect("window_size");
    app.event_type.set_window_size(app.window_size);
}

fn change_font_size<'a, Global, State, Event, Error>(
    app: &mut Running<'a, Global, State, Event, Error>,
) where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    let font_size_px = (app.global.salsa_ctx().font_size.get()
        * app.window.as_ref().expect("window").scale_factor())
    .round() as u32;
    app.terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .backend_mut()
        .update_font_size(font_size_px);

    app.window_size = app
        .terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .backend_mut()
        .window_size()
        .expect("window_size");
    app.event_type.set_window_size(app.window_size);
}

fn resized_event<'a, Global, State, Event, Error>(
    app: &mut Running<'a, Global, State, Event, Error>,
) -> Option<Event>
where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    let size = app.window_size.pixels;
    let size = PhysicalSize::new(size.width as u32, size.height as u32);
    app.event_type.convert(WindowEvent::Resized(size))
}

fn resize<'a, Global, State, Event, Error>(
    app: &mut Running<'a, Global, State, Event, Error>,
    size: PhysicalSize<u32>,
) where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    app.terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .backend_mut()
        .resize(size.width, size.height);

    app.window_size = app
        .terminal
        .as_ref()
        .expect("terminal")
        .borrow_mut()
        .backend_mut()
        .window_size()
        .expect("window_size");

    app.event_type.set_window_size(app.window_size);
}

fn shutdown<'a, Global, State, Event, Error>(
    app: &mut Running<'a, Global, State, Event, Error>,
    _event_loop: &ActiveEventLoop,
) where
    Global: SalsaContext<Event, Error>,
    Event: 'static + Send,
    Error: 'static + Debug + Send + From<io::Error>,
{
    let t = app
        .global
        .salsa_ctx()
        .term
        .borrow_mut()
        .take()
        .expect("terminal");
    drop(t);

    let t = app.terminal.take().expect("terminal");
    if Rc::strong_count(&t) > 1 {
        panic!("Terminal still referenced during shutdown. Can't shutdown cleanly.");
    }
    drop(t);

    let w = app
        .global
        .salsa_ctx()
        .window
        .borrow_mut()
        .take()
        .expect("window");
    drop(w);

    let w = app.window.take().expect("window");
    if Arc::strong_count(&w) > 1 {
        panic!("Window still referenced during shutdown. Can't shutdown cleanly");
    }
    drop(w);

    app.poll.shutdown();
}

struct PollStart {
    can_start: Arc<(Mutex<bool>, Condvar)>,
}

struct Poll {
    cancel: Cancel,
    join_handle: JoinHandle<()>,
}

impl PollStart {
    fn start(&self) {
        let (lock, cvar) = &*self.can_start;
        let mut started = lock.lock().expect("can_start mutex");
        *started = true;

        // We notify the condvar that the value has changed.
        cvar.notify_one();
    }
}

impl Poll {
    fn shutdown(&self) {
        self.cancel.cancel();
        self.join_handle.thread().unpark();
    }
}

const SLEEP: u64 = 250_000; // µs
const BACKOFF: u64 = 10_000; // µs
const FAST_SLEEP: u64 = 100; // µs

fn create_poll<Event, Error>(
    event_loop: EventLoopProxy<Result<Control<Event>, Error>>,
    mut poll: Vec<Box<dyn PollEvents<Event, Error> + Send>>,
) -> (PollStart, Poll)
where
    Event: 'static + Send,
    Error: 'static + Debug + Send,
{
    let cancel = Cancel::new();
    let can_start = Arc::new((Mutex::new(false), Condvar::new()));

    let t_cancel = cancel.clone();
    let t_can_start = Arc::clone(&can_start);
    let join_handle = thread::spawn(move || {
        let poll_queue = PollQueue::default();
        let mut poll_sleep = Duration::from_micros(SLEEP);

        // Wait till we are free to start polling.
        let (lock, cvar) = &*t_can_start;
        let mut can_start = lock.lock().unwrap();
        while !*can_start {
            can_start = cvar.wait(can_start).unwrap();
        }

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

    (
        PollStart { can_start },
        Poll {
            cancel,
            join_handle,
        },
    )
}
