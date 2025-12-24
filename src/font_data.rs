use append_only_vec::AppendOnlyVec;
use std::sync::OnceLock;

/// Some fallback font data.
static FALLBACK_DATA: &[u8] = include_bytes!("CascadiaMono-Regular.ttf");
static FALLBACK_FONT: OnceLock<ratatui_wgpu::Font<'static>> = OnceLock::new();
#[cfg(feature = "fallback_symbol_font")]
static SYMBOL_DATA: &[u8] = include_bytes!("NotoSansSymbols2-Regular.ttf");
#[cfg(feature = "fallback_symbol_font")]
static SYMBOL_FONT: OnceLock<ratatui_wgpu::Font<'static>> = OnceLock::new();
#[cfg(feature = "fallback_emoji_font")]
static EMOJI_DATA: &[u8] = include_bytes!("OpenMoji-black-glyf.ttf");
#[cfg(feature = "fallback_emoji_font")]
static EMOJI_FONT: OnceLock<ratatui_wgpu::Font<'static>> = OnceLock::new();

static FONTDB: OnceLock<fontdb::Database> = OnceLock::new();
static FONT_DATA: AppendOnlyVec<(fontdb::ID, Box<[u8]>)> = AppendOnlyVec::new();
static FONTS: AppendOnlyVec<(fontdb::ID, ratatui_wgpu::Font<'static>)> = AppendOnlyVec::new();

pub struct FontData;

impl FontData {
    pub fn fallback_font(self) -> ratatui_wgpu::Font<'static> {
        FALLBACK_FONT
            .get_or_init(|| ratatui_wgpu::Font::new(FALLBACK_DATA).expect("valid_font"))
            .clone()
    }

    #[cfg(feature = "fallback_emoji_font")]
    pub fn fallback_emoji_font(self) -> Option<ratatui_wgpu::Font<'static>> {
        Some(
            EMOJI_FONT
                .get_or_init(|| ratatui_wgpu::Font::new(EMOJI_DATA).expect("valid_font"))
                .clone(),
        )
    }

    #[cfg(not(feature = "fallback_emoji_font"))]
    pub fn fallback_emoji_font(self) -> Option<ratatui_wgpu::Font<'static>> {
        None
    }

    #[cfg(feature = "fallback_symbol_font")]
    pub fn fallback_symbol_font(self) -> Option<ratatui_wgpu::Font<'static>> {
        Some(
            SYMBOL_FONT
                .get_or_init(|| ratatui_wgpu::Font::new(SYMBOL_DATA).expect("valid_font"))
                .clone(),
        )
    }

    #[cfg(not(feature = "fallback_symbol_font"))]
    pub fn fallback_symbol_font(self) -> Option<ratatui_wgpu::Font<'static>> {
        None
    }

    pub fn font_db(self) -> &'static fontdb::Database {
        FONTDB.get_or_init(|| {
            let mut font_db = fontdb::Database::new();
            font_db.load_system_fonts();
            font_db
        })
    }

    /// Font already cached?
    pub fn have_font(self, id: fontdb::ID) -> bool {
        for (font_id, _) in FONTS.iter() {
            if id == *font_id {
                return true;
            }
        }
        false
    }

    /// Create a Font and cache the underlying data.
    pub fn load_font(self, id: fontdb::ID) -> Option<ratatui_wgpu::Font<'static>> {
        for (font_id, font) in FONTS.iter() {
            if id == *font_id {
                return Some(font.clone());
            }
        }

        let data = self
            .font_db()
            .with_face_data(id, |d, _| d.to_vec())
            .expect("font_data");
        let idx = FONT_DATA.push((id, data.into_boxed_slice()));
        let (_, data) = &FONT_DATA[idx];

        let font = ratatui_wgpu::Font::new(data).expect("valid-font");
        FONTS.push((id, font.clone()));

        Some(font)
    }
}
