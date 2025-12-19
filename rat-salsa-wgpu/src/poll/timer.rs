use crate::poll::PollEvents;
use crate::timer::{TimeOut, Timers};
use crate::Control;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

/// Processes timers.
#[derive(Debug, Default)]
pub struct PollTimers {
    timers: Arc<Timers>,
}

impl PollTimers {
    pub fn new() -> Self {
        Self {
            timers: Arc::new(Timers::default()),
        }
    }

    pub(crate) fn get_timers(&self) -> Arc<Timers> {
        self.timers.clone()
    }
}

impl<Event, Error> PollEvents<Event, Error> for PollTimers
where
    Event: 'static + From<TimeOut>,
    Error: 'static + From<std::io::Error>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn sleep_time(&self) -> Option<Duration> {
        self.timers.sleep_time()
    }

    fn poll(&mut self) -> Result<bool, Error> {
        Ok(self.timers.poll())
    }

    fn read(&mut self) -> Result<Control<Event>, Error> {
        Ok(self
            .timers
            .read()
            .map(|v| Control::Event(v.0.into()))
            .unwrap_or(Control::Continue))
    }
}
