use rat_event::{ConsumedEvent, HandleEvent, Outcome, Regular};
use rat_focus::Focus;
use ratatui::Terminal;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use winit::window::Window;

mod framework;
mod run_config;
pub mod crossterm;

pub use framework::*;
use ratatui_wgpu::WgpuBackend;
use ratatui_wgpu::shaders::AspectPreservingDefaultPostProcessor;
pub use run_config::*;

pub mod mock {
    //! Provides dummy implementations for some functions.

    /// Empty placeholder for [run_tui](crate::run_tui).
    pub fn init<State, Global, Error>(
        _state: &mut State, //
        _ctx: &mut Global,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Empty placeholder for [run_tui](crate::run_tui).
    pub fn error<Global, State, Event, Error>(
        _error: Error,
        _state: &mut State,
        _ctx: &mut Global,
    ) -> Result<crate::Control<Event>, Error> {
        Ok(crate::Control::Continue)
    }
}

/// Result enum for event handling.
///
/// The result of an event is processed immediately after the
/// function returns, before polling new events. This way an action
/// can trigger another action which triggers the repaint without
/// other events intervening.
///
/// If you ever need to return more than one result from event-handling,
/// you can hand it to AppContext/RenderContext::queue(). Events
/// in the queue are processed in order, and the return value of
/// the event-handler comes last. If an error is returned, everything
/// send to the queue will be executed nonetheless.
///
/// __See__
///
/// - [flow!](rat_event::flow)
/// - [try_flow!](rat_event::try_flow)
/// - [ConsumedEvent]
#[derive(Debug, Clone, Copy)]
#[must_use]
#[non_exhaustive]
pub enum Control<Event> {
    /// Continue with event-handling.
    /// In the event-loop this waits for the next event.
    Continue,
    /// Break event-handling without repaint.
    /// In the event-loop this waits for the next event.
    Unchanged,
    /// Break event-handling and repaints/renders the application.
    /// In the event-loop this calls `render`.
    Changed,
    /// Eventhandling can cause secondary application specific events.
    /// One common way is to return this `Control::Message(my_event)`
    /// to reenter the event-loop with your own secondary event.
    ///
    /// This acts quite like a message-queue to communicate between
    /// disconnected parts of your application. And indeed there is
    /// a hidden message-queue as part of the event-loop.
    ///
    /// The other way is to call [SalsaAppContext::queue] to initiate such
    /// events.
    Event(Event),
    /// A dialog close event. In the main loop it will be handled
    /// just like [Control::Event]. But the DialogStack can react
    /// separately and close the window.
    #[cfg(feature = "dialog")]
    Close(Event),
    /// Quit the application.
    Quit,
}

impl<Event> Eq for Control<Event> {}

impl<Event> PartialEq for Control<Event> {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}

impl<Event> Ord for Control<Event> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            Control::Continue => match other {
                Control::Continue => Ordering::Equal,
                Control::Unchanged => Ordering::Less,
                Control::Changed => Ordering::Less,
                Control::Event(_) => Ordering::Less,
                #[cfg(feature = "dialog")]
                Control::Close(_) => Ordering::Less,
                Control::Quit => Ordering::Less,
            },
            Control::Unchanged => match other {
                Control::Continue => Ordering::Greater,
                Control::Unchanged => Ordering::Equal,
                Control::Changed => Ordering::Less,
                Control::Event(_) => Ordering::Less,
                #[cfg(feature = "dialog")]
                Control::Close(_) => Ordering::Less,
                Control::Quit => Ordering::Less,
            },
            Control::Changed => match other {
                Control::Continue => Ordering::Greater,
                Control::Unchanged => Ordering::Greater,
                Control::Changed => Ordering::Equal,
                Control::Event(_) => Ordering::Less,
                #[cfg(feature = "dialog")]
                Control::Close(_) => Ordering::Less,
                Control::Quit => Ordering::Less,
            },
            Control::Event(_) => match other {
                Control::Continue => Ordering::Greater,
                Control::Unchanged => Ordering::Greater,
                Control::Changed => Ordering::Greater,
                Control::Event(_) => Ordering::Equal,
                #[cfg(feature = "dialog")]
                Control::Close(_) => Ordering::Less,
                Control::Quit => Ordering::Less,
            },
            #[cfg(feature = "dialog")]
            Control::Close(_) => match other {
                Control::Continue => Ordering::Greater,
                Control::Unchanged => Ordering::Greater,
                Control::Changed => Ordering::Greater,
                Control::Event(_) => Ordering::Greater,
                Control::Close(_) => Ordering::Equal,
                Control::Quit => Ordering::Less,
            },
            Control::Quit => match other {
                Control::Continue => Ordering::Greater,
                Control::Unchanged => Ordering::Greater,
                Control::Changed => Ordering::Greater,
                Control::Event(_) => Ordering::Greater,
                #[cfg(feature = "dialog")]
                Control::Close(_) => Ordering::Greater,
                Control::Quit => Ordering::Equal,
            },
        }
    }
}

impl<Event> PartialOrd for Control<Event> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Event> ConsumedEvent for Control<Event> {
    fn is_consumed(&self) -> bool {
        !matches!(self, Control::Continue)
    }
}

impl<Event, T: Into<Outcome>> From<T> for Control<Event> {
    fn from(value: T) -> Self {
        let r = value.into();
        match r {
            Outcome::Continue => Control::Continue,
            Outcome::Unchanged => Control::Unchanged,
            Outcome::Changed => Control::Changed,
        }
    }
}

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

    /// Set the cursor, if the given value is something,
    /// hides it otherwise.
    ///
    /// This should only be set during rendering.
    fn set_screen_cursor(&self, cursor: Option<(u16, u16)>) {
        if let Some(c) = cursor {
            self.salsa_ctx().cursor.set(Some(c));
        }
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

    /// Access the window.
    #[inline]
    fn window(&self) -> Arc<Window> {
        self.salsa_ctx().window.clone().expect("window")
    }

    /// Access the terminal.
    #[inline]
    fn terminal(
        &self,
    ) -> Rc<RefCell<Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>>>>
    {
        self.salsa_ctx().term.clone().expect("terminal")
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
    pub(crate) term: Option<
        Rc<RefCell<Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>>>>,
    >,
    /// Window
    pub(crate) window: Option<Arc<Window>>,
    /// Last render time.
    pub(crate) last_render: Cell<Duration>,
    /// Last event time.
    pub(crate) last_event: Cell<Duration>,
    /// Queue foreground tasks.
    pub(crate) queue: control_queue::ControlQueue<Event, Error>,
}

impl<Event, Error> Debug for SalsaAppContext<Event, Error> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut ff = f.debug_struct("AppContext");
        ff.field("focus", &self.focus)
            .field("count", &self.count)
            .field("cursor", &self.cursor)
            // .field("clear_terminal", &self.clear_terminal)
            // .field("insert_before", &"n/a")
            // .field("timers", &self.timers)
            // .field("tasks", &self.tasks)
            .field("queue", &self.queue);
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
            // clear_terminal: Default::default(),
            // insert_before: Default::default(),
            window: Default::default(),
            last_render: Default::default(),
            last_event: Default::default(),
            // timers: Default::default(),
            // tasks: Default::default(),
            // #[cfg(feature = "async")]
            // tokio: Default::default(),
            queue: Default::default(),
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
