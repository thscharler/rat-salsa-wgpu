/// Timer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerEvent(pub crate::timer::TimeOut);

/// Event sent immediately after rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedEvent;

/// Event sent immediately before quitting the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuitEvent;

pub use winit::event::WindowEvent;
pub use rat_wgpu::event::{CompositeWindowEvent, WindowEventState, winit_to_crossterm_event};
