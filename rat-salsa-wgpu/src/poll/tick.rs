use crate::Control;
use crate::poll::PollEvents;
use std::any::Any;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

/// Triggers a render after some duration has passed.
pub struct PollTick {
    rate: Arc<RwLock<Duration>>,
    next: SystemTime,
}

impl PollTick {
    /// New FPS trigger.
    ///
    /// __Returns__
    ///
    /// Returns the Poll and an Arc to the configured duration.
    pub fn new(start_lag: Duration, rate: Duration) -> (Self, Arc<RwLock<Duration>>) {
        let tick = Self {
            rate: Arc::new(RwLock::new(rate)),
            next: SystemTime::now() + start_lag,
        };
        let tick_cfg = tick.rate.clone();
        (tick, tick_cfg)
    }
}

impl<Event, Error> PollEvents<Event, Error> for PollTick
where
    Event: 'static,
    Error: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn poll(&mut self) -> Result<bool, Error> {
        Ok(self.next <= SystemTime::now())
    }

    fn read(&mut self) -> Result<Control<Event>, Error> {
        if self.next <= SystemTime::now() {
            let rate = self.rate.read().expect("rw-lock read");
            self.next += *rate;
            Ok(Control::Changed)
        } else {
            Ok(Control::Continue)
        }
    }
}
