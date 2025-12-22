use crate::event_type::{CompositeWinitEvent, ConvertEvent, WinitEventState};
use std::sync::{Arc, RwLock};

/// Does a noop conversion to CompositWinitEvent, that
/// only adds the tracked modifier-state and window-size.
#[derive(Debug, Default)]
pub struct ConvertWinit {
    state: Arc<RwLock<WinitEventState>>,
}

impl<Event> ConvertEvent<Event> for ConvertWinit
where
    Event: 'static + From<CompositeWinitEvent>,
{
    fn set_window_size(&mut self, window_size: ratatui::backend::WindowSize) {
        self.state
            .write()
            .expect("rw-lock write")
            .set_window_size(window_size);
    }

    fn update_state(&mut self, event: &winit::event::WindowEvent) {
        self.state
            .write()
            .expect("rw-lock write")
            .update_state(event)
    }

    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event> {
        Some(
            CompositeWinitEvent {
                event,
                state: Arc::clone(&self.state),
            }
            .into(),
        )
    }
}

impl ConvertWinit {
    pub fn new() -> Self {
        Self::default()
    }
}
