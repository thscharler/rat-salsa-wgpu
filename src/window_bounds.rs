use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// Window bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowBounds {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            maximized: false,
        }
    }
}

impl Display for WindowBounds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}+{}+{}{}",
            self.x,
            self.y,
            self.width,
            self.height,
            if self.maximized { "+max" } else { "" }
        )
    }
}

impl FromStr for WindowBounds {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tok = s.split([':', '+']);
        let Some(x) = tok.next() else { return Err(()) };
        let Ok(x) = i32::from_str(x) else {
            return Err(());
        };
        let Some(y) = tok.next() else { return Err(()) };
        let Ok(y) = i32::from_str(y) else {
            return Err(());
        };
        let Some(width) = tok.next() else {
            return Err(());
        };
        let Ok(width) = u32::from_str(width) else {
            return Err(());
        };
        let Some(height) = tok.next() else {
            return Err(());
        };
        let Ok(height) = u32::from_str(height) else {
            return Err(());
        };
        let mut maximized = false;
        if let Some(max) = tok.next() {
            if max == "max" {
                maximized = true;
            }
        }

        Ok(WindowBounds {
            x,
            y,
            width,
            height,
            maximized,
        })
    }
}

impl WindowBounds {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            maximized: false,
        }
    }
}
