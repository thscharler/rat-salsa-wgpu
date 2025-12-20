pub mod convert_winit;
pub mod convert_crossterm;

pub trait ConvertEvent<Event> {
    /// Modifier state changed.
    fn set_modifiers(&mut self, modifiers: winit::event::Modifiers);
    /// Window size changed.
    fn set_window_size(&mut self, window_size: ratatui::backend::WindowSize);

    /// Convert winit event.
    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event>;
}
