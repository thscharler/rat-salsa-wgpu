use crate::event_type::ConvertEvent;

/// Winit event with extra tracked modifier-state and window-size.
pub struct CompositeWinitEvent {
    modifiers: winit::event::Modifiers,
    window_size: ratatui::backend::WindowSize,
    event: winit::event::WindowEvent,
}

/// Does a noop conversion to CompositWinitEvent, that
/// only adds the tracked modifier-state and window-size.
#[derive(Debug)]
pub struct ConvertWinit {
    modifiers: winit::event::Modifiers,
    window_size: ratatui::backend::WindowSize,
}

impl Default for ConvertWinit {
    fn default() -> Self {
        Self {
            modifiers: Default::default(),
            window_size: ratatui::backend::WindowSize {
                columns_rows: Default::default(),
                pixels: Default::default(),
            },
        }
    }
}

impl<Event> ConvertEvent<Event> for ConvertWinit
where
    Event: 'static + From<CompositeWinitEvent>,
{
    fn set_modifiers(&mut self, modifiers: winit::event::Modifiers) {
        self.modifiers = modifiers;
    }

    fn set_window_size(&mut self, window_size: ratatui::backend::WindowSize) {
        self.window_size = window_size;
    }

    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event> {
        Some(
            CompositeWinitEvent {
                modifiers: self.modifiers,
                window_size: self.window_size,
                event,
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
