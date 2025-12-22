pub mod convert_crossterm;
pub mod convert_winit;

///
/// Event-type converter from winit-events to something else.
///
pub trait ConvertEvent<Event> {
    /// Window size changed.
    fn set_window_size(&mut self, window_size: ratatui::backend::WindowSize);
    /// Update some states.
    fn update_state(&mut self, event: &winit::event::WindowEvent);
    /// Query the current state.
    fn state(&self) -> &WinitEventState;

    /// Convert winit event.
    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event>;
}

/// Winit event with extra tracked modifier-state and window-size.
#[derive(Debug, Clone)]
pub struct CompositeWinitEvent {
    pub event: winit::event::WindowEvent,
    pub state: WinitEventState,
}

#[derive(Debug, Default, Clone)]
pub struct WinitEventState {
    /// Modifiers.
    pub m_shift: bool,
    /// Modifiers
    pub m_alt: bool,
    /// Modifiers
    pub m_ctrl: bool,
    /// Modifiers
    pub m_super: bool,
    /// Pending dead key
    pub dead_key_press: Option<char>,
    /// Pending dead key
    pub dead_key_release: Option<char>,
    /// Window sizes.
    pub window_size_px: ratatui::layout::Size,
    /// Window sizes in rendered cells.
    pub window_size: ratatui::layout::Size,
    /// Rendered text cell width.
    pub cell_width_px: u16,
    /// Rendered text cell height.
    pub cell_height_px: u16,
    /// Mouse cursor.
    pub x: u16,
    /// Mouse cursor.
    pub y: u16,
    /// Mouse cursor.
    pub x_px: f64,
    /// Mouse cursor.
    pub y_px: f64,
    /// Mouse button state.
    pub left_pressed: bool,
    /// Mouse button state.
    pub middle_pressed: bool,
    /// Mouse button state.
    pub right_pressed: bool,
    /// Mouse button state
    pub back_pressed: bool,
    /// Mouse button state
    pub forward_pressed: bool,
}

impl WinitEventState {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set_window_size(&mut self, window_size: ratatui::backend::WindowSize) {
        self.window_size = window_size.columns_rows;
        self.window_size_px = window_size.pixels;
        self.cell_width_px = window_size.pixels.width / window_size.columns_rows.width;
        self.cell_height_px = window_size.pixels.height / window_size.columns_rows.height;
    }

    pub(crate) fn update_state(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.m_shift = modifiers.state().shift_key();
                self.m_alt = modifiers.state().alt_key();
                self.m_ctrl = modifiers.state().control_key();
                self.m_super = modifiers.state().super_key();
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                if self.cell_width_px == 0 || self.cell_height_px == 0 {
                    return;
                }
                self.x_px = position.x;
                self.y_px = position.y;
                self.x = (position.x / self.cell_width_px as f64) as u16;
                self.y = (position.y / self.cell_height_px as f64) as u16;
            }
            winit::event::WindowEvent::CursorEntered { .. } => {}
            winit::event::WindowEvent::CursorLeft { .. } => {
                self.x_px = 0.0;
                self.y_px = 0.0;
                self.x = 0;
                self.y = 0;
            }
            winit::event::WindowEvent::MouseWheel { .. } => {}
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let pressed = match state {
                    winit::event::ElementState::Pressed => true,
                    winit::event::ElementState::Released => false,
                };
                match button {
                    winit::event::MouseButton::Left => {
                        self.left_pressed = pressed;
                    }
                    winit::event::MouseButton::Right => {
                        self.right_pressed = pressed;
                    }
                    winit::event::MouseButton::Middle => {
                        self.middle_pressed = pressed;
                    }
                    winit::event::MouseButton::Back => {
                        self.back_pressed = pressed;
                    }
                    winit::event::MouseButton::Forward => {
                        self.forward_pressed = pressed;
                    }
                    winit::event::MouseButton::Other(_) => {
                        // noop
                    }
                }
            }
            _ => {}
        }
    }
}
