use crate::event_type::{CompositeWinitEvent, ConvertEvent, WinitEventState};

/// Does a noop conversion to CompositWinitEvent, that
/// only adds the tracked modifier-state and window-size.
#[derive(Debug, Default)]
pub struct ConvertWinit {
    state: WinitEventState,
}

impl<Event> ConvertEvent<Event> for ConvertWinit
where
    Event: 'static + From<CompositeWinitEvent>,
{
    fn set_window_size(&mut self, window_size: ratatui_core::backend::WindowSize) {
        self.state.set_window_size(window_size);
    }

    fn update_state(&mut self, event: &winit::event::WindowEvent) {
        self.state.update_state(event)
    }

    fn state(&self) -> &WinitEventState {
        &self.state
    }

    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event> {
        Some(
            CompositeWinitEvent {
                event,
                state: self.state.clone(),
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
