use crossbeam::channel::{SendError, Sender};
use log::debug;
#[allow(dead_code)]
use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::Focus;
use ratatui::Terminal;
use ratatui_wgpu::WgpuBackend;
use ratatui_wgpu::shaders::{ DefaultPostProcessorBuilder};
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::AbortHandle;
use winit::window::Window;

#[cfg(not(feature = "preserve_aspect_ratio"))]
pub(crate) type PostProcessorBuilder = DefaultPostProcessorBuilder<false>;
#[cfg(feature = "preserve_aspect_ratio")]
pub(crate) type PostProcessorBuilder = DefaultPostProcessorBuilder<true>;

mod control;
mod framework;
mod run_config;
mod thread_pool;
#[cfg(feature = "async")]
mod tokio_tasks;

use crate::font_data::FontData;
use crate::tasks::{Cancel, Liveness};
use crate::thread_pool::ThreadPool;
use crate::timer::{TimerDef, TimerHandle, Timers};
use crate::tokio_tasks::TokioTasks;
pub use control::Control;
pub use framework::run_tui;
pub use run_config::{RunConfig, TermInit};

#[cfg(feature = "dialog")]
pub mod dialog_stack;
/// Event types.
pub mod event;
/// Support for different event-types.
pub mod event_type;
/// Support for fonts.
pub mod font_data;
/// Provides dummy implementations for some functions.
pub mod mock;
/// Event sources.
pub mod poll;
/// Types used for both future tasks and thread tasks.
pub mod tasks;
/// Support for timers.
pub mod timer;

/// This trait gives access to all facilities built into rat-salsa.
///
/// Your global state struct has to implement this trait. This allows
/// rat-salsa to add its facilities to it.
///
/// [run_tui] sets it during initialization, it will be up and
/// running by the time init() is called.
///
pub trait SalsaContext<Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    /// The AppContext struct holds all the data for the rat-salsa
    /// functionality. [run_tui] calls this to set the initialized
    /// struct.
    fn set_salsa_ctx(&mut self, app_ctx: SalsaAppContext<Event, Error>);

    /// Access the AppContext previously set.
    fn salsa_ctx(&self) -> &SalsaAppContext<Event, Error>;

    /// Get the current frame/render-count.
    fn count(&self) -> usize {
        self.salsa_ctx().count.get()
    }

    /// Get the last render timing.
    fn last_render(&self) -> Duration {
        self.salsa_ctx().last_render.get()
    }

    /// Get the last event-handling timing.
    fn last_event(&self) -> Duration {
        self.salsa_ctx().last_event.get()
    }

    /// Set a window title.
    fn set_window_title(&self, title: String) {
        self.window().set_title(title.as_str());
    }

    /// Set the cursor, if the given value is something,
    /// hides it otherwise.
    ///
    /// This should only be set during rendering.
    fn set_screen_cursor(&self, cursor: Option<(u16, u16)>) {
        if let Some(c) = cursor {
            self.salsa_ctx().cursor.set(Some(c));
        }
    }

    /// Add a timer.
    ///
    /// __Panic__
    ///
    /// Panics if no timer support is configured.
    #[inline]
    fn add_timer(&self, t: TimerDef) -> TimerHandle {
        self.salsa_ctx()
            .timers
            .as_ref()
            .expect("No timers configured. In main() add RunConfig::default()?.poll(PollTimers)")
            .add(t)
    }

    /// Remove a timer.
    ///
    /// __Panic__
    ///
    /// Panics if no timer support is configured.
    #[inline]
    fn remove_timer(&self, tag: TimerHandle) {
        self.salsa_ctx()
            .timers
            .as_ref()
            .expect("No timers configured. In main() add RunConfig::default()?.poll(PollTimers)")
            .remove(tag);
    }

    /// Replace a timer.
    /// Remove the old timer and create a new one.
    /// If the old timer no longer exists it just creates the new one.
    ///
    /// __Panic__
    ///
    /// Panics if no timer support is configured.
    #[inline]
    fn replace_timer(&self, h: Option<TimerHandle>, t: TimerDef) -> TimerHandle {
        if let Some(h) = h {
            self.remove_timer(h);
        }
        self.add_timer(t)
    }

    /// Add a background worker task.
    ///
    /// ```rust ignore
    /// let cancel = ctx.spawn(|cancel, send| {
    ///     // ... do stuff
    ///     if cancel.is_canceled() {
    ///         return; // return early
    ///     }
    ///     Ok(Control::Continue)
    /// });
    /// ```
    ///
    /// - Cancel token
    ///
    /// The cancel token can be used by the application to signal an early
    /// cancellation of a long-running task. This cancellation is cooperative,
    /// the background task must regularly check for cancellation and quit
    /// if needed.
    ///
    /// - Liveness token
    ///
    /// This token is set whenever the given task has finished, be it
    /// regularly or by panicking.
    ///
    /// __Panic__
    ///
    /// Panics if no worker-thread support is configured.
    #[inline]
    fn spawn_ext(
        &self,
        task: impl FnOnce(
            Cancel,
            &Sender<Result<Control<Event>, Error>>,
        ) -> Result<Control<Event>, Error>
        + Send
        + 'static,
    ) -> Result<(Cancel, Liveness), SendError<()>>
    where
        Event: 'static + Send,
        Error: 'static + Send,
    {
        self.salsa_ctx()
            .tasks
            .as_ref()
            .expect(
                "No thread-pool configured. In main() add RunConfig::default()?.poll(PollTasks)",
            )
            .spawn(Box::new(task))
    }

    /// Add a background worker task.
    ///
    /// ```rust ignore
    /// let cancel = ctx.spawn(|| {
    ///     // ...
    ///     Ok(Control::Continue)
    /// });
    /// ```
    ///
    /// __Panic__
    ///
    /// Panics if no worker-thread support is configured.
    #[inline]
    fn spawn(
        &self,
        task: impl FnOnce() -> Result<Control<Event>, Error> + Send + 'static,
    ) -> Result<(), SendError<()>>
    where
        Event: 'static + Send,
        Error: 'static + Send,
    {
        _ = self
            .salsa_ctx()
            .tasks
            .as_ref()
            .expect(
                "No thread-pool configured. In main() add RunConfig::default()?.poll(PollTasks)",
            )
            .spawn(Box::new(|_, _| task()))?;
        Ok(())
    }

    /// Spawn a future in the executor.
    ///
    /// Panic
    ///
    /// Panics if tokio is not configured.
    #[inline]
    #[cfg(feature = "async")]
    fn spawn_async<F>(&self, future: F)
    where
        F: Future<Output = Result<Control<Event>, Error>> + Send + 'static,
        Event: 'static + Send,
        Error: 'static + Send,
    {
        _ = self.salsa_ctx() //
            .tokio
            .as_ref()
            .expect("No tokio runtime is configured. In main() add RunConfig::default()?.poll(PollTokio::new(rt))")
            .spawn(Box::new(future));
    }

    /// Spawn a future in the executor.
    /// You get an extra channel to send back more than one result.
    ///
    /// - AbortHandle
    ///
    /// The tokio AbortHandle to abort a spawned task.
    ///
    /// - Liveness
    ///
    /// This token is set whenever the given task has finished, be it
    /// regularly or by panicking.
    ///
    /// Panic
    ///
    /// Panics if tokio is not configured.
    #[inline]
    #[cfg(feature = "async")]
    fn spawn_async_ext<C, F>(&self, cr_future: C) -> (AbortHandle, Liveness)
    where
        C: FnOnce(tokio::sync::mpsc::Sender<Result<Control<Event>, Error>>) -> F,
        F: Future<Output = Result<Control<Event>, Error>> + Send + 'static,
        Event: 'static + Send,
        Error: 'static + Send,
    {
        let rt = self
            .salsa_ctx() //
            .tokio
            .as_ref()
            .expect("No tokio runtime is configured. In main() add RunConfig::default()?.poll(PollTokio::new(rt))");
        let future = cr_future(rt.sender());
        rt.spawn(Box::new(future))
    }

    /// Queue an application event.
    #[inline]
    fn queue_event(&self, event: Event) {
        self.salsa_ctx().queue.push(Ok(Control::Event(event)));
    }

    /// Queue additional results.
    #[inline]
    fn queue(&self, ctrl: impl Into<Control<Event>>) {
        self.salsa_ctx().queue.push(Ok(ctrl.into()));
    }

    /// Queue an error.
    #[inline]
    fn queue_err(&self, err: Error) {
        self.salsa_ctx().queue.push(Err(err));
    }

    /// Set the `Focus`.
    #[inline]
    fn set_focus(&self, focus: Focus) {
        self.salsa_ctx().focus.replace(Some(focus));
    }

    /// Take the `Focus` back from the Context.
    #[inline]
    fn take_focus(&self) -> Option<Focus> {
        self.salsa_ctx().focus.take()
    }

    /// Clear the `Focus`.
    #[inline]
    fn clear_focus(&self) {
        self.salsa_ctx().focus.replace(None);
    }

    /// Access the `Focus`.
    ///
    /// __Panic__
    ///
    /// Panics if no focus has been set.
    #[inline]
    fn focus<'a>(&'a self) -> Ref<'a, Focus> {
        let borrow = self.salsa_ctx().focus.borrow();
        Ref::map(borrow, |v| v.as_ref().expect("focus"))
    }

    /// Mutably access the focus-field.
    ///
    /// __Panic__
    ///
    /// Panics if no focus has been set.
    #[inline]
    fn focus_mut<'a>(&'a mut self) -> RefMut<'a, Focus> {
        let borrow = self.salsa_ctx().focus.borrow_mut();
        RefMut::map(borrow, |v| v.as_mut().expect("focus"))
    }

    /// Handle the focus-event and automatically queue the result.
    ///
    /// __Panic__
    ///
    /// Panics if no focus has been set.
    #[inline]
    fn handle_focus<E>(&mut self, event: &E) -> Outcome
    where
        Focus: HandleEvent<E, Regular, Outcome>,
    {
        let mut borrow = self.salsa_ctx().focus.borrow_mut();
        let focus = borrow.as_mut().expect("focus");
        let r = focus.handle(event, Regular);
        if r.is_consumed() {
            self.queue(r);
        }
        r
    }

    /// Access the terminal.
    #[inline]
    fn terminal(&self) -> Rc<RefCell<Terminal<WgpuBackend<'static, 'static>>>> {
        self.salsa_ctx().term.clone().expect("terminal")
    }

    /// Clear the terminal and do a full redraw before the next draw.
    #[inline]
    fn clear_terminal(&mut self) {
        self.salsa_ctx().clear_terminal.set(true);
        self.salsa_ctx().window().request_redraw();
    }

    /// Access the window.
    #[inline]
    fn window(&self) -> Arc<Window> {
        self.salsa_ctx().window.clone().expect("window")
    }

    /// Change the font-size
    fn set_font_size(&self, size: f64) {
        self.salsa_ctx().font_size.set(size);
        self.salsa_ctx().font_size_changed.set(true);
        self.salsa_ctx().window().request_redraw();
    }

    fn font_size(&self) -> f64 {
        self.salsa_ctx().font_size.get()
    }

    /// Change the font-family
    fn set_font_family(&self, family: &str) {
        *self.salsa_ctx().font_family.borrow_mut() = family.to_string();
        let font_ids = FontData
            .font_db()
            .faces()
            .filter_map(|info| {
                for (v, _) in &info.families {
                    if v.as_str() == family {
                        debug!("use font {}", info.post_script_name);
                        return Some(info.id);
                    }
                }
                None
            })
            .collect::<Vec<_>>();
        self.salsa_ctx().font_ids.replace(font_ids);
        self.salsa_ctx().font_changed.set(true);
        self.salsa_ctx().window().request_redraw();
    }

    fn font_family(&self) -> String {
        self.salsa_ctx().font_family.borrow().clone()
    }
}

///
/// Application context for event handling.
///
/// Add this to your global state and implement [SalsaContext] to
/// access the facilities of rat-salsa. You can Default::default()
/// initialize this field with some dummy values. It will
/// be set correctly when calling [run_tui].
///
pub struct SalsaAppContext<Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    /// Can be set to hold a Focus, if needed.
    pub(crate) focus: RefCell<Option<Focus>>,
    /// Last frame count rendered.
    pub(crate) count: Cell<usize>,
    /// Output cursor position. Set to Frame after rendering is complete.
    pub(crate) cursor: Cell<Option<(u16, u16)>>,
    /// Terminal area
    pub(crate) term: Option<Rc<RefCell<Terminal<WgpuBackend<'static, 'static>>>>>,
    /// Clear terminal before next draw.
    pub(crate) clear_terminal: Cell<bool>,
    /// Last render time.
    pub(crate) last_render: Cell<Duration>,
    /// Last event time.
    pub(crate) last_event: Cell<Duration>,

    /// Application timers.
    pub(crate) timers: Option<Arc<Timers>>,
    /// Background tasks.
    pub(crate) tasks: Option<Arc<ThreadPool<Event, Error>>>,
    /// Background tasks.
    #[cfg(feature = "async")]
    pub(crate) tokio: Option<Arc<TokioTasks<Event, Error>>>,
    /// Queue foreground tasks.
    pub(crate) queue: framework::control_queue::ControlQueue<Event, Error>,

    /// Window
    pub(crate) window: Option<Arc<Window>>,

    pub(crate) font_changed: Cell<bool>,
    pub(crate) font_size_changed: Cell<bool>,
    pub(crate) font_ids: RefCell<Vec<fontdb::ID>>,
    pub(crate) font_family: RefCell<String>,
    pub(crate) font_size: Cell<f64>,
}

impl<Event, Error> Debug for SalsaAppContext<Event, Error> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut ff = f.debug_struct("AppContext");
        ff.field("focus", &self.focus)
            .field("count", &self.count)
            .field("cursor", &self.cursor)
            .field("clear_terminal", &self.clear_terminal.get())
            .field("last_render", &self.last_render.get())
            .field("last_event", &self.last_event.get())
            .field("timers", &self.timers)
            .field("tasks", &self.tasks)
            .field("queue", &self.queue)
            .field("font_changed", &self.font_changed.get())
            .field("font_size_changed", &self.font_size_changed.get())
            .field("font_ids", &self.font_ids.borrow())
            .field("font_family", &self.font_family.borrow())
            .field("font_size", &self.font_size.get());
        #[cfg(feature = "async")]
        {
            ff.field("tokio", &self.tokio);
        }

        ff.finish()
    }
}

impl<Event, Error> Default for SalsaAppContext<Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    fn default() -> Self {
        Self {
            focus: Default::default(),
            count: Default::default(),
            cursor: Default::default(),
            term: Default::default(),
            clear_terminal: Cell::new(false),
            window: Default::default(),
            font_changed: Default::default(),
            font_size_changed: Default::default(),
            font_ids: Default::default(),
            last_render: Default::default(),
            last_event: Default::default(),
            timers: Default::default(),
            tasks: Default::default(),
            #[cfg(feature = "async")]
            tokio: Default::default(),
            queue: Default::default(),
            font_size: Default::default(),
            font_family: Default::default(),
        }
    }
}

impl<Event, Error> SalsaContext<Event, Error> for SalsaAppContext<Event, Error>
where
    Event: 'static,
    Error: 'static,
{
    #[inline]
    fn set_salsa_ctx(&mut self, app_ctx: SalsaAppContext<Event, Error>) {
        *self = app_ctx;
    }

    #[inline]
    fn salsa_ctx(&self) -> &SalsaAppContext<Event, Error> {
        self
    }
}

mod _private {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NonExhaustive;
}
