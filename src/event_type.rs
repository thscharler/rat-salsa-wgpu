use std::sync::{Arc, RwLock};

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

    /// Convert winit event.
    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event>;
}

/// Winit event with extra tracked modifier-state and window-size.
#[derive(Debug, Clone)]
pub struct CompositeWinitEvent {
    pub event: winit::event::WindowEvent,
    pub state: Arc<RwLock<WinitEventState>>,
}

impl CompositeWinitEvent {
    pub fn shift_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").m_shift
    }

    pub fn alt_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").m_shift
    }

    pub fn ctrl_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").m_shift
    }

    pub fn super_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").m_shift
    }

    pub fn window_size(&self) -> ratatui::layout::Size {
        self.state.read().expect("rw-lock read").window_size
    }

    pub fn window_size_px(&self) -> ratatui::layout::Size {
        self.state.read().expect("rw-lock read").window_size_px
    }

    pub fn cell_width_px(&self) -> u16 {
        self.state.read().expect("rw-lock read").cell_width_px
    }

    pub fn cell_height_px(&self) -> u16 {
        self.state.read().expect("rw-lock read").cell_height_px
    }

    pub fn x(&self) -> u16 {
        self.state.read().expect("rw-lock read").x
    }

    pub fn y(&self) -> u16 {
        self.state.read().expect("rw-lock read").y
    }

    pub fn x_px(&self) -> f64 {
        self.state.read().expect("rw-lock read").x_px
    }

    pub fn y_px(&self) -> f64 {
        self.state.read().expect("rw-lock read").y_px
    }

    pub fn left_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").left_pressed
    }

    pub fn right_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").right_pressed
    }

    pub fn middle_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").middle_pressed
    }

    pub fn back_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").back_pressed
    }

    pub fn forward_pressed(&self) -> bool {
        self.state.read().expect("rw-lock read").forward_pressed
    }

    pub fn other_pressed(&self, n: usize) -> bool {
        let v = self
            .state
            .read()
            .expect("rw-lock read")
            .other_pressed
            .get(n)
            .cloned();
        v.unwrap_or(false)
    }
}

#[derive(Debug, Default)]
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
    /// Mouse button state
    pub other_pressed: Vec<bool>,
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
                    winit::event::MouseButton::Other(n) => {
                        while self.other_pressed.len() <= (n + 1) as usize {
                            self.other_pressed.push(false);
                        }
                        self.other_pressed[*n as usize] = pressed;
                    }
                }
            }
            _ => {}
        }
    }
}
