//!
//! Triggers a blink switch timer.
//!
use crate::Control;
use crate::poll::PollEvents;
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// Triggers a render after some duration has passed.
pub struct PollBlink {
    rate: Arc<AtomicU64>,
    next: SystemTime,
}

impl PollBlink {
    /// New blink trigger.
    ///
    /// - start_lag: initial wait in milliseconds.
    /// - rate: wait between blink switches in milliseconds.
    ///
    /// __Returns__
    ///
    /// Returns the Poll and an Arc to the configured duration.
    pub fn new(start_lag: u64, rate: u64) -> Self {
        Self {
            rate: Arc::new(AtomicU64::new(rate)),
            next: SystemTime::now() + Duration::from_millis(start_lag),
        }
    }
}

impl<Event, Error> PollEvents<Event, Error> for PollBlink
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
            let rate = self.rate.load(Ordering::Acquire);
            self.next += Duration::from_millis(rate);
            Ok(Control::Blink)
        } else {
            Ok(Control::Continue)
        }
    }
}
