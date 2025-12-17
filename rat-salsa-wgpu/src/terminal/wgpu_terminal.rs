use crate::terminal::Terminal;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui_wgpu::shaders::AspectPreservingDefaultPostProcessor;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::{io, mem, ptr};
use winit::window::Window;

pub struct WgpuTerminal {
    window: Arc<Window>,
    term: ratatui::Terminal<WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor>>,
}

impl WgpuTerminal {
    pub fn new(window: Arc<Window>) -> WgpuTerminal {
        let fonts = load_fonts();
        let size = window.inner_size();

        let backend = futures_lite::future::block_on(
            Builder::from_font(
                Font::new(include_bytes!("../CascadiaMono-Regular.ttf")).expect("font"),
            )
            .with_fonts(fonts)
            .with_width_and_height(Dimensions {
                width: NonZeroU32::new(size.width).expect("non-zero width"),
                height: NonZeroU32::new(size.height).expect("non-zero-height"),
            })
            .build_with_target(window.clone()),
        )
        .expect("ratatui-wgpu-backend");

        Self {
            window,
            term: ratatui::Terminal::new(backend).expect("ratatui-terminal"),
        }
    }

    pub fn backend_mut(
        &mut self,
    ) -> &mut WgpuBackend<'static, 'static, AspectPreservingDefaultPostProcessor> {
        self.term.backend_mut()
    }
}

fn load_fonts() -> Vec<ratatui_wgpu::Font<'static>> {
    static mut FONTDB: *const fontdb::Database = ptr::dangling();
    static mut FONTS: *const Vec<Vec<u8>> = ptr::dangling();

    let fontdb = unsafe {
        let mut fontdb = fontdb::Database::new();
        fontdb.load_system_fonts();
        FONTDB = &raw const fontdb;
        mem::forget(fontdb);

        &*FONTDB
    };

    let fonts = unsafe {
        let mut vec = Vec::new();
        FONTS = &raw const vec;
        mem::forget(vec);

        &mut *(FONTS as *mut Vec<Vec<u8>>)
    };

    unsafe {
        for v in fontdb
            .faces()
            .filter_map(|info| {
                if (info.monospaced
                    || info.post_script_name.contains("Emoji")
                    || info.post_script_name.contains("emoji"))
                    && info.index == 0
                {
                    Some(info.id)
                } else {
                    None
                }
            })
            .filter_map(|id| {
                FONTDB
                    .as_ref()
                    .expect("fontdb")
                    .with_face_data(id, |d, _| d.to_vec())
            })
        {
            fonts.push(v);
        }
    }

    fonts
        .iter()
        .filter_map(|d| ratatui_wgpu::Font::new(d))
        .collect::<Vec<_>>()
}

impl<Error> Terminal<Error> for WgpuTerminal
where
    Error: 'static + From<io::Error>,
{
    fn init(&mut self) -> Result<(), io::Error> {
        Ok(())
    }

    fn get_frame(&mut self) -> Frame<'_> {
        self.term.get_frame()
    }

    fn current_buffer_mut(&mut self) -> &mut Buffer {
        self.term.current_buffer_mut()
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.term.flush()
    }

    fn resize(&mut self, area: Rect) -> Result<(), io::Error> {
        self.term.resize(area)
    }

    fn hide_cursor(&mut self) -> Result<(), io::Error> {
        self.term.hide_cursor()
    }

    fn show_cursor(&mut self) -> Result<(), io::Error> {
        self.term.show_cursor()
    }

    fn get_cursor_position(&mut self) -> Result<Position, io::Error> {
        self.term.get_cursor_position()
    }

    fn set_cursor_position(&mut self, position: Position) -> Result<(), io::Error> {
        self.term.set_cursor_position(position)
    }

    fn clear(&mut self) -> Result<(), io::Error> {
        self.term.clear()
    }

    fn swap_buffers(&mut self) {
        self.term.swap_buffers()
    }

    fn size(&self) -> Result<Size, io::Error> {
        self.term.size()
    }

    fn insert_before(
        &mut self,
        _height: u16,
        _draw_fn: Box<dyn FnOnce(&mut Buffer)>,
    ) -> Result<(), io::Error> {
        unimplemented!("insert_before is not supported")
    }

    #[cfg(feature = "scrolling-regions")]
    fn scroll_region_up(&mut self, region: Range<u16>, line_count: u16) -> Result<(), io::Error> {
        unimplemented!("scroll_region_up is not supported")
    }

    #[cfg(feature = "scrolling-regions")]
    fn scroll_region_down(&mut self, region: Range<u16>, line_count: u16) -> Result<(), io::Error> {
        unimplemented!("scroll_region_down is not supported")
    }

    fn shutdown(&mut self) -> Result<(), io::Error> {
        Ok(())
    }

    fn render(
        &mut self,
        f: &mut dyn FnMut(&mut Frame<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut res = Ok(());
        _ = self.term.hide_cursor();
        self.term.draw(|frame| res = f(frame))?;
        res
    }
}
