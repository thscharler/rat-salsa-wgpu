use anyhow::Error;
use log::{debug, error};
use rat_event::{ct_event, try_flow};
use rat_salsa_wgpu::event::{QuitEvent, RenderedEvent};
use rat_salsa_wgpu::event_type::CompositeWinitEvent;
use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrossterm;
use rat_salsa_wgpu::font_data::FontData;
use rat_salsa_wgpu::poll::{PollTasks, PollTimers};
use rat_salsa_wgpu::timer::TimeOut;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::palette::Colors;
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, create_salsa_theme};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::{Color, Style, Stylize};
use ratatui_core::text::{Line, Span, Text};
use ratatui_core::widgets::Widget;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Nord");
    let mut global = Global::new(config, theme);
    let mut state = Minimal::default();

    run_tui(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::new(ConvertCrossterm::new())?
            .window_position(winit::dpi::PhysicalPosition::new(30, 30))
            .font_family("IBM Plex Mono")
            .font_size(35.)
            .poll(PollTimers::new())
            .poll(PollTasks::new(2)),
    )?;

    Ok(())
}

/// Globally accessible data/state.
pub struct Global {
    // the salsa machinery
    ctx: SalsaAppContext<AppEvent, Error>,

    pub cfg: Config,
    pub theme: SalsaTheme,
    pub fonts: Vec<String>,
}

impl SalsaContext<AppEvent, Error> for Global {
    fn set_salsa_ctx(&mut self, app_ctx: SalsaAppContext<AppEvent, Error>) {
        self.ctx = app_ctx;
    }

    fn salsa_ctx(&self) -> &SalsaAppContext<AppEvent, Error> {
        &self.ctx
    }
}

impl Global {
    pub fn new(cfg: Config, theme: SalsaTheme) -> Self {
        let mut fonts = FontData.installed_fonts().clone();
        fonts.insert(0, "<Fallback>".to_string());
        Self {
            ctx: Default::default(),
            cfg,
            theme,
            fonts,
        }
    }
}

/// Configuration.
#[derive(Debug, Default)]
pub struct Config {}

#[derive(Debug)]
pub enum AppEvent {
    NoOp,
    Event(CompositeWinitEvent),
    CtEvent(crossterm::event::Event),
    TimeOut(TimeOut),
    Quit,
    Rendered,
}

impl From<crossterm::event::Event> for AppEvent {
    fn from(value: crossterm::event::Event) -> Self {
        AppEvent::CtEvent(value)
    }
}

impl From<CompositeWinitEvent> for AppEvent {
    fn from(value: CompositeWinitEvent) -> Self {
        AppEvent::Event(value)
    }
}

impl From<RenderedEvent> for AppEvent {
    fn from(_: RenderedEvent) -> Self {
        AppEvent::Rendered
    }
}

impl From<QuitEvent> for AppEvent {
    fn from(_: QuitEvent) -> Self {
        AppEvent::Quit
    }
}

impl From<TimeOut> for AppEvent {
    fn from(value: TimeOut) -> Self {
        Self::TimeOut(value)
    }
}

#[derive(Debug, Default)]
pub struct Minimal {
    pub font_idx: usize,
    pub range_idx: usize,
}

pub fn init(_state: &mut Minimal, _ctx: &mut Global) -> Result<(), Error> {
    Ok(())
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<(), Error> {
    let layout = Layout::vertical([
        Constraint::Fill(1), //
    ])
    .split(area);

    buf.set_style(area, ctx.theme.style_style(Style::CONTAINER_BASE));

    let (block_start, block_end, block) = BLOCKS[state.range_idx];

    let mut txt = Text::default();
    txt.push_line(Line::from(format!("  :: {}", ctx.font_family())).bold());
    txt.push_line(Line::from(format!("  :: {}", ctx.font_size())).italic());
    txt.push_line(format!("  :: {}", block));
    txt.push_line(format!(
        "  :: {:#5x} - {:#5x}",
        block_start as u32, block_end as u32
    ));

    const CLUSTER: u32 = 16;

    // let mut tmp1 = Line::default();
    let mut tmp2 = Line::default();

    let byte_span = format!(
        "{:#5x} - {:#5x} ",
        block_start as u32,
        block_start as u32 + CLUSTER
    );
    tmp2.push_span(Span::from(byte_span));

    for cc in block_start..=block_end {
        let off = cc as u32 - block_start as u32;

        if off != 0 && off % CLUSTER == 0 {
            txt.push_line(tmp2);
            txt.push_line(Line::default());

            let byte_span = format!(
                "{:#5x} - {:#5x} ",
                block_start as u32 + off,
                block_start as u32 + off + CLUSTER
            );
            tmp2 = Line::default();
            tmp2.push_span(Span::from(byte_span));
        }

        tmp2.push_span(" ");

        let mut glyph_style = ctx.theme.p.high_bg_style(Colors::Yellow, Colors::Green, 5);
        glyph_style = glyph_style.underlined();

        tmp2.push_span(Span::from(cc.to_string()).style(glyph_style));
    }

    txt.push_line(tmp2);
    txt.render(layout[0], buf);

    Ok(())
}

static BLOCKS: &'static [(char, char, &'static str)] = &[
    ('\u{0000}', '\u{007F}', "Lateinisch, Basis"),
    ('\u{0080}', '\u{00FF}', "Lateinisch, Ergänzung"),
    ('\u{0100}', '\u{017F}', "Lateinisch, Erweiterung A"),
    ('\u{0180}', '\u{024F}', "Lateinisch, Erweiterung B"),
    ('\u{0250}', '\u{02AF}', "IPA-Erweiterungen"),
    ('\u{02B0}', '\u{02FF}', "Modifikationszeichen mit Vorschub"),
    // ('\u{0300}', '\u{036F}', "Kombinierende diakritische Zeichen"),
    ('\u{0370}', '\u{03FF}', "Griechisch und Koptisch"),
    ('\u{0400}', '\u{04FF}', "Kyrillisch"),
    ('\u{0500}', '\u{052F}', "Kyrillisch, Ergänzung"),
    ('\u{0530}', '\u{058F}', "Armenisch"),
    ('\u{0590}', '\u{05FF}', "Hebräisch"),
    ('\u{0600}', '\u{06FF}', "Arabisch"),
    ('\u{0700}', '\u{074F}', "Syrisch"),
    ('\u{0750}', '\u{077F}', "Arabisch, Ergänzung"),
    // ('\u{0780}', '\u{07BF}', "Thaana"),
    // ('\u{07C0}', '\u{07FF}', "N’Ko"),
    // ('\u{0800}', '\u{083F}', "Samaritanisch"),
    // ('\u{0840}', '\u{085F}', "Mandäisch"),
    // ('\u{0860}', '\u{086F}', "Syrisch, Ergänzung"),
    // ('\u{08A0}', '\u{08FF}', "Arabisch, Erweiterung A"),
    // ('\u{0900}', '\u{097F}', "Devanagari"),
    // ('\u{0980}', '\u{09FF}', "Bengalisch"),
    // ('\u{0A00}', '\u{0A7F}', "Gurmukhi"),
    // ('\u{0A80}', '\u{0AFF}', "Gujarati"),
    // ('\u{0B00}', '\u{0B7F}', "Oriya"),
    // ('\u{0B80}', '\u{0BFF}', "Tamilisch"),
    // ('\u{0C00}', '\u{0C7F}', "Telugu"),
    // ('\u{0C80}', '\u{0CFF}', "Kannada"),
    // ('\u{0D00}', '\u{0D7F}', "Malayalam"),
    // ('\u{0D80}', '\u{0DFF}', "Singhalesisch"),
    // ('\u{0E00}', '\u{0E7F}', "Thailändisch"),
    // ('\u{0E80}', '\u{0EFF}', "Laotisch"),
    // ('\u{0F00}', '\u{0FFF}', "Tibetisch"),
    // ('\u{1000}', '\u{109F}', "Birmanisch"),
    // ('\u{10A0}', '\u{10FF}', "Georgisch"),
    // ('\u{1100}', '\u{11FF}', "Hangeul-Jamo"),
    // ('\u{1200}', '\u{137F}', "Äthiopisch"),
    // ('\u{1380}', '\u{139F}', "Äthiopisch, Zusatz"),
    // ('\u{13A0}', '\u{13FF}', "Cherokee"),
    // (
    //     '\u{1400}',
    //     '\u{167F}',
    //     "Vereinheitlichte Silbenzeichen kanadischer Ureinwohner",
    // ),
    // ('\u{1680}', '\u{169F}', "Ogam"),
    // ('\u{16A0}', '\u{16FF}', "Runen"),
    // ('\u{1700}', '\u{171F}', "Tagalog"),
    // ('\u{1720}', '\u{173F}', "Hanunóo"),
    // ('\u{1740}', '\u{175F}', "Buid"),
    // ('\u{1760}', '\u{177F}', "Tagbanuwa"),
    // ('\u{1780}', '\u{17FF}', "Khmer"),
    // ('\u{1800}', '\u{18AF}', "Mongolisch"),
    // (
    //     '\u{18B0}',
    //     '\u{18FF}',
    //     "Vereinheitlichte Silbenzeichen kanadischer Ureinwohner, Erweiterung",
    // ),
    // ('\u{1900}', '\u{194F}', "Limbu"),
    // ('\u{1950}', '\u{197F}', "Tai Le"),
    // ('\u{1980}', '\u{19DF}', "Neu-Tai-Lue"),
    // ('\u{19E0}', '\u{19FF}', "Khmer-Symbole"),
    // ('\u{1A00}', '\u{1A1F}', "Buginesisch"),
    // ('\u{1A20}', '\u{1AAF}', "Lanna"),
    // (
    //     '\u{1AB0}',
    //     '\u{1AFF}',
    //     "Kombinierte diakritische Zeichen, Erweiterung",
    // ),
    // ('\u{1B00}', '\u{1B7F}', "Balinesisch"),
    // ('\u{1B80}', '\u{1BBF}', "Sundanesisch"),
    // ('\u{1BC0}', '\u{1BFF}', "Batak"),
    // ('\u{1C00}', '\u{1C4F}', "Lepcha"),
    // ('\u{1C50}', '\u{1C7F}', "Ol Chiki"),
    ('\u{1C80}', '\u{1C8F}', "Kyrillisch, Erweiterung C"),
    ('\u{1C90}', '\u{1CBF}', "Georgian Extended"),
    // ('\u{1CC0}', '\u{1CCF}', "Sundanesisch, Ergänzung"),
    // ('\u{1CD0}', '\u{1CFF}', "Vedisch-Erweiterungen"),
    ('\u{1D00}', '\u{1D7F}', "Phonetische Erweiterungen"),
    (
        '\u{1D80}',
        '\u{1DBF}',
        "Phonetische Erweiterungen, Ergänzung",
    ),
    // (
    //     '\u{1DC0}',
    //     '\u{1DFF}',
    //     "Kombinierte diakritische Zeichen, Ergänzung",
    // ),
    ('\u{1E00}', '\u{1EFF}', "Lateinisch, weiterter Zusatz"),
    ('\u{1F00}', '\u{1FFF}', "Griechisch-Erweiterungen"),
    ('\u{2000}', '\u{206F}', "Allgemeine Interpunktionen"),
    ('\u{2070}', '\u{209F}', "Hoch- und tiefgestellte Zeichen"),
    ('\u{20A0}', '\u{20CF}', "Währungszeichen"),
    // (
    //     '\u{20D0}',
    //     '\u{20FF}',
    //     "Kombinierte diakritische Zeichen für Symbole",
    // ),
    ('\u{2100}', '\u{214F}', "Buchstabenähnliche Symbole"),
    ('\u{2150}', '\u{218F}', "Zahlzeichen"),
    ('\u{2190}', '\u{21FF}', "Pfeile"),
    ('\u{2200}', '\u{22FF}', "Mathematische Operatoren"),
    ('\u{2300}', '\u{23FF}', "Verschiedene technische Zeichen"),
    ('\u{2400}', '\u{243F}', "Symbole für Steuerzeichen"),
    ('\u{2440}', '\u{245F}', "Optische Zeichenerkennung"),
    (
        '\u{2460}',
        '\u{24FF}',
        "Umschlossene alphanumerische Zeichen",
    ),
    ('\u{2500}', '\u{257F}', "Rahmenzeichnung"),
    ('\u{2580}', '\u{259F}', "Blockelemente"),
    ('\u{25A0}', '\u{25FF}', "Geometrische Formen"),
    ('\u{2600}', '\u{26FF}', "Verschiedene Symbole"),
    ('\u{2700}', '\u{27BF}', "Dingbats"),
    (
        '\u{27C0}',
        '\u{27EF}',
        "Verschiedene mathematische Symbole A",
    ),
    ('\u{27F0}', '\u{27FF}', "Pfeile, Zusatz A"),
    ('\u{2800}', '\u{28FF}', "Braille-Zeichen"),
    ('\u{2900}', '\u{297F}', "Pfeile, Zusatz B"),
    (
        '\u{2980}',
        '\u{29FF}',
        "Verschiedene mathematische Symbole B",
    ),
    ('\u{2A00}', '\u{2AFF}', "Mathematische Operatoren, Zusatz"),
    ('\u{2B00}', '\u{2BFF}', "Verschiedene Symbole und Pfeile"),
    ('\u{2C00}', '\u{2C5F}', "Glagolitisch"),
    ('\u{2C60}', '\u{2C7F}', "Lateinisch, Erweiterung C"),
    ('\u{2C80}', '\u{2CFF}', "Koptisch"),
    ('\u{2D00}', '\u{2D2F}', "Georgisch, Ergänzung"),
    // ('\u{2D30}', '\u{2D7F}', "Tifinagh"),
    // ('\u{2D80}', '\u{2DDF}', "Äthiopisch-Erweiterungen"),
    ('\u{2DE0}', '\u{2DFF}', "Kyrillisch, Erweiterung A"),
    ('\u{2E00}', '\u{2E7F}', "Interpunktionen, Zusatz"),
    // ('\u{2E80}', '\u{2EFF}', "CJK-Radikale, Ergänzung"),
    // ('\u{2F00}', '\u{2FDF}', "Kangxi-Radikale"),
    // (
    //     '\u{2FF0}',
    //     '\u{2FFF}',
    //     "Ideographische Beschreibungszeichen",
    // ),
    // ('\u{3000}', '\u{303F}', "CJK-Symbole und -Interpunktionen"),
    // ('\u{3040}', '\u{309F}', "Hiragana"),
    // ('\u{30A0}', '\u{30FF}', "Katakana"),
    // ('\u{3100}', '\u{312F}', "Bopomofo"),
    // ('\u{3130}', '\u{318F}', "Hangeul-Jamo, Kompatibilität"),
    // ('\u{3190}', '\u{319F}', "Kanbun"),
    // ('\u{31A0}', '\u{31BF}', "Bopomofo-Erweiterungen"),
    // ('\u{31C0}', '\u{31EF}', "CJK-Striche"),
    // ('\u{31F0}', '\u{31FF}', "Phonetisch-Katakana-Erweiterungen"),
    // (
    //     '\u{3200}',
    //     '\u{32FF}',
    //     "Umschlossene CJK-Zeichen und -Monate",
    // ),
    // ('\u{3300}', '\u{33FF}', "CJK-Kompatibilität"),
    // (
    //     '\u{3400}',
    //     '\u{4DBF}',
    //     "Vereinheitlichte CJK-Ideogramme, Erweiterung A",
    // ),
    // ('\u{4DC0}', '\u{4DFF}', "I-Ging-Hexagramme"),
    // ('\u{4E00}', '\u{9FFF}', "Vereinheitlichte CJK-Ideogramme"),
    // ('\u{A000}', '\u{A48F}', "Yi-Silbenzeichen"),
    // ('\u{A490}', '\u{A4CF}', "Yi-Radikale"),
    // ('\u{A4D0}', '\u{A4FF}', "Lisu"),
    // ('\u{A500}', '\u{A63F}', "Vai"),
    ('\u{A640}', '\u{A69F}', "Kyrillisch, Erweiterung B"),
    // ('\u{A6A0}', '\u{A6FF}', "Bamum"),
    ('\u{A700}', '\u{A71F}', "Modifizierende Tonzeichen"),
    ('\u{A720}', '\u{A7FF}', "Lateinisch, Erweiterung D"),
    // ('\u{A800}', '\u{A82F}', "Syloti Nagri"),
    // ('\u{A830}', '\u{A83F}', "Allgemeine indische Ziffern"),
    // ('\u{A840}', '\u{A87F}', "Phagspa"),
    // ('\u{A880}', '\u{A8DF}', "Saurashtra"),
    // ('\u{A8E0}', '\u{A8FF}', "Devanagari-Erweiterungen"),
    // ('\u{A900}', '\u{A92F}', "Kayah Li"),
    // ('\u{A930}', '\u{A95F}', "Rejang"),
    // ('\u{A960}', '\u{A97F}', "Hangeul-Jamo, Erweiterung A"),
    // ('\u{A980}', '\u{A9DF}', "Javanisch"),
    // ('\u{A9E0}', '\u{A9FF}', "Birmanisch, Erweiterung B"),
    // ('\u{AA00}', '\u{AA5F}', "Cham"),
    // ('\u{AA60}', '\u{AA7F}', "Birmanisch, Erweiterung A"),
    // ('\u{AA80}', '\u{AADF}', "Tai Viet"),
    // ('\u{AAE0}', '\u{AAFF}', "Meitei-Mayek-Erweiterungen"),
    // ('\u{AB00}', '\u{AB2F}', "Äthiopisch, Erweiterung A"),
    ('\u{AB30}', '\u{AB6F}', "Lateinisch, Erweiterung E"),
    // ('\u{AB70}', '\u{ABBF}', "Cherokee-Erweiterungen"),
    // ('\u{ABC0}', '\u{ABFF}', "Meitei-Mayek"),
    // ('\u{AC00}', '\u{D7AF}', "Hangeul-Silbenzeichen"),
    // ('\u{D7B0}', '\u{D7FF}', "Hangeul-Jamo, Erweiterung B"),
    // ('\u{D800}', '\u{DB7F}', "Obere Surrogate"),
    // ('\u{DB80}', '\u{DBFF}', "Obere Surrogate zur privaten Nutzung"),
    // ('\u{DC00}', '\u{DFFF}', "Untere Surrogate"),
    ('\u{E000}', '\u{F8FF}', "Bereich zur privaten Nutzung"),
    // ('\u{F900}', '\u{FAFF}', "CJK-Kompatibilitätsideogramme"),
    ('\u{FB00}', '\u{FB4F}', "Alphabetische Präsentationsformen"),
    // ('\u{FB50}', '\u{FDFF}', "Arabische Präsentationsformen A"),
    // ('\u{FE00}', '\u{FE0F}', "Varianten-Selektoren"),
    ('\u{FE10}', '\u{FE1F}', "Vertikale Formen"),
    // (
    //     '\u{FE20}',
    //     '\u{FE2F}',
    //     "Kombinierende halbe diakritische Zeichen",
    // ),
    // ('\u{FE30}', '\u{FE4F}', "CJK-Kompatibilitätsformen"),
    ('\u{FE50}', '\u{FE6F}', "Kleine Formvarianten"),
    // ('\u{FE70}', '\u{FEFF}', "Arabische Präsentationsformen B"),
    ('\u{FF00}', '\u{FFEF}', "Halbbreite und vollbreite Formen"),
    ('\u{FFF0}', '\u{FFFF}', "Spezielles"),
    // ('\u{10000}', '\u{1007F}', "Linear-B-Silbenzeichen"),
    // ('\u{10080}', '\u{100FF}', "Linear-B-Ideogramme"),
    // ('\u{10100}', '\u{1013F}', "Ägäische Zahlzeichen"),
    // ('\u{10140}', '\u{1018F}', "Altgriechische Zahlzeichen"),
    ('\u{10190}', '\u{101CF}', "Alte Symbole"),
    // ('\u{101D0}', '\u{101FF}', "Diskos von Phaistos"),
    // ('\u{10280}', '\u{1029F}', "Lykisch"),
    // ('\u{102A0}', '\u{102DF}', "Karisch"),
    // ('\u{102E0}', '\u{102FF}', "Koptische Zahlzeichen"),
    // ('\u{10300}', '\u{1032F}', "Altitalisch"),
    // ('\u{10330}', '\u{1034F}', "Gotisch"),
    // ('\u{10350}', '\u{1037F}', "Altpermisch"),
    // ('\u{10380}', '\u{1039F}', "Ugaritisch"),
    // ('\u{103A0}', '\u{103DF}', "Altpersisch"),
    // ('\u{10400}', '\u{1044F}', "Mormonenalphabet"),
    // ('\u{10450}', '\u{1047F}', "Shaw-Alphabet"),
    // ('\u{10480}', '\u{104AF}', "Osmaniya"),
    // ('\u{104B0}', '\u{104FF}', "Osage"),
    // ('\u{10500}', '\u{1052F}', "Albanisch"),
    // ('\u{10530}', '\u{1056F}', "Alwanisch"),
    // ('\u{10600}', '\u{1077F}', "Linear A"),
    // ('\u{10800}', '\u{1083F}', "Kyprische Schrift"),
    // ('\u{10840}', '\u{1085F}', "Aramäisch"),
    // ('\u{10860}', '\u{1087F}', "Palmyrenisch"),
    // ('\u{10880}', '\u{108AF}', "Nabatäisch"),
    // ('\u{108E0}', '\u{108FF}', "Hatran"),
    ('\u{10900}', '\u{1091F}', "Phönizisch"),
    ('\u{10920}', '\u{1093F}', "Lydisch"),
    ('\u{10980}', '\u{1099F}', "Meroitische Hieroglyphen"),
    ('\u{109A0}', '\u{109FF}', "Meroitisch-demotisch"),
    // ('\u{10A00}', '\u{10A5F}', "Kharoshthi"),
    // ('\u{10A60}', '\u{10A7F}', "Altsüdarabisch"),
    // ('\u{10A80}', '\u{10A9F}', "Altnordarabisch"),
    // ('\u{10AC0}', '\u{10AFF}', "Manichäisch"),
    // ('\u{10B00}', '\u{10B3F}', "Avestisch"),
    // ('\u{10B40}', '\u{10B5F}', "Parthisch"),
    // ('\u{10B60}', '\u{10B7F}', "Inschriften-Pahlavi"),
    // ('\u{10B80}', '\u{10BAF}', "Psalter-Pahlavi"),
    // ('\u{10C00}', '\u{10C4F}', "Alttürkisch"),
    // ('\u{10C80}', '\u{10CFF}', "Altungarisch"),
    // ('\u{10D00}', '\u{10D3F}', "Hanifi Rohingya"),
    // ('\u{10E60}', '\u{10E7F}', "Rumi-Ziffern"),
    // ('\u{10E80}', '\u{10EBF}', "Yezidi"),
    // ('\u{10F00}', '\u{10F2F}', "Old Sogdian"),
    // ('\u{10F30}', '\u{10F6F}', "Sogdian"),
    // ('\u{10FB0}', '\u{10FDF}', "Chorasmian"),
    // ('\u{10FE0}', '\u{10FFF}', "Elymaic"),
    // ('\u{11000}', '\u{1107F}', "Brahmi"),
    // ('\u{11080}', '\u{110CF}', "Kaithi"),
    // ('\u{110D0}', '\u{110FF}', "Sorang-Sompeng"),
    // ('\u{11100}', '\u{1114F}', "Chakma"),
    // ('\u{11150}', '\u{1117F}', "Mahajani"),
    // ('\u{11180}', '\u{111DF}', "Sharada"),
    // ('\u{111E0}', '\u{111FF}', "Singhalesische alte Zahlzeichen"),
    // ('\u{11200}', '\u{1124F}', "Khojki"),
    // ('\u{11280}', '\u{112AF}', "Multani"),
    // ('\u{112B0}', '\u{112FF}', "Khudabadi"),
    // ('\u{11300}', '\u{1137F}', "Grantha"),
    // ('\u{11400}', '\u{1147F}', "Newa"),
    // ('\u{11480}', '\u{114DF}', "Tirhuta"),
    // ('\u{11580}', '\u{115FF}', "Siddham"),
    // ('\u{11600}', '\u{1165F}', "Modi"),
    // ('\u{11660}', '\u{1167F}', "Mongolisch, Ergänzung"),
    // ('\u{11680}', '\u{116CF}', "Takri"),
    // ('\u{11700}', '\u{1173F}', "Ahom"),
    // ('\u{11800}', '\u{1184F}', "Dogra"),
    // ('\u{118A0}', '\u{118FF}', "Varang Kshiti"),
    // ('\u{11900}', '\u{1195F}', "Dives Akuru"),
    // ('\u{119A0}', '\u{119FF}', "Nandinagari"),
    // ('\u{11A00}', '\u{11A4F}', "Dsanabadsar-Quadratschrift"),
    // ('\u{11A50}', '\u{11AAF}', "Sojombo-Schrift"),
    // ('\u{11AC0}', '\u{11AFF}', "Pau Cin Hau"),
    // ('\u{11C00}', '\u{11C6F}', "Bhaiksuki"),
    // ('\u{11C70}', '\u{11CBF}', "Marchen"),
    // ('\u{11D00}', '\u{11D5F}', "Masaram Gondi"),
    // ('\u{11D60}', '\u{11DAF}', "Gunjala Gondi"),
    // ('\u{11EE0}', '\u{11EFF}', "Makasar"),
    // ('\u{11FB0}', '\u{11FBF}', "Lisu Supplement"),
    // ('\u{11FC0}', '\u{11FFF}', "Tamil Supplement"),
    ('\u{12000}', '\u{123FF}', "Keilschrift"),
    (
        '\u{12400}',
        '\u{1247F}',
        "Keilschrift-Zahlzeichen und -Interpunktionen",
    ),
    ('\u{12480}', '\u{1254F}', "Frühe Keilschrift"),
    ('\u{13000}', '\u{1342F}', "Ägyptische Hieroglyphen"),
    (
        '\u{13430}',
        '\u{1343F}',
        "Egyptian Hieroglyph Format Controls",
    ),
    ('\u{14400}', '\u{1467F}', "Anatolische Hieroglyphen"),
    // ('\u{16800}', '\u{16A3F}', "Bamum, Ergänzung"),
    // ('\u{16A40}', '\u{16A6F}', "Mro"),
    // ('\u{16AD0}', '\u{16AFF}', "Bassa Vah"),
    // ('\u{16B00}', '\u{16B8F}', "Pahawh Hmong"),
    // ('\u{16E40}', '\u{16E9F}', "Medefaidrin"),
    // ('\u{16F00}', '\u{16F9F}', "Pollard-Schrift"),
    // (
    //     '\u{16FE0}',
    //     '\u{16FFF}',
    //     "Ideographische Symbole und Interpunktionen",
    // ),
    // ('\u{17000}', '\u{187FF}', "Tangut"),
    // ('\u{18800}', '\u{18AFF}', "Tangut-Komponenten"),
    // ('\u{18B00}', '\u{18CFF}', "Khitan Small Script"),
    // ('\u{18D00}', '\u{18D8F}', "Tangut Supplement"),
    // ('\u{1B000}', '\u{1B0FF}', "Kana, Ergänzung"),
    // ('\u{1B100}', '\u{1B12F}', "Kana-Erweiterungen"),
    // ('\u{1B130}', '\u{1B16F}', "Small Kana Extension"),
    // ('\u{1B170}', '\u{1B2FF}', "Nuschu-Schrift"),
    // ('\u{1BC00}', '\u{1BC9F}', "Duployé-Kurzschrift"),
    // ('\u{1BCA0}', '\u{1BCAF}', "Kurzschrift-Steuerzeichen"),
    // (
    //     '\u{1D000}',
    //     '\u{1D0FF}',
    //     "Byzantinische Notenschriftzeichen",
    // ),
    // ('\u{1D100}', '\u{1D1FF}', "Notenschriftzeichen"),
    // (
    //     '\u{1D200}',
    //     '\u{1D24F}',
    //     "Altgriechische Notenschriftzeichen",
    // ),
    // ('\u{1D2E0}', '\u{1D2FF}', "Mayan Numerals"),
    // ('\u{1D300}', '\u{1D35F}', "Tai-Xuan-Jing-Symbole"),
    // ('\u{1D360}', '\u{1D37F}', "Zählstabziffern"),
    // (
    //     '\u{1D400}',
    //     '\u{1D7FF}',
    //     "Mathematische alphanumerische Symbole",
    // ),
    // ('\u{1D800}', '\u{1DAAF}', "Sutton-Zeichenschrift"),
    // ('\u{1E000}', '\u{1E02F}', "Glagolitisch, Ergänzung"),
    // ('\u{1E100}', '\u{1E14F}', "Nyiakeng Puachue Hmong"),
    // ('\u{1E2C0}', '\u{1E2FF}', "Wancho"),
    // ('\u{1E800}', '\u{1E8DF}', "Mende-Schrift"),
    // ('\u{1E900}', '\u{1E95F}', "Adlam"),
    // ('\u{1EC70}', '\u{1ECBF}', "Indic Siyaq Numbers"),
    // ('\u{1ED00}', '\u{1ED4F}', "Ottoman Siyaq Numbers"),
    // (
    //     '\u{1EE00}',
    //     '\u{1EEFF}',
    //     "Arabische mathemathische alphanumerische Symbole",
    // ),
    ('\u{1F000}', '\u{1F02F}', "Mah-Jongg-Steine"),
    ('\u{1F030}', '\u{1F09F}', "Dominosteine"),
    ('\u{1F0A0}', '\u{1F0FF}', "Spielkarten"),
    (
        '\u{1F100}',
        '\u{1F1FF}',
        "Umschlossene alphanumerische Zeichen, Zusatz",
    ),
    // ('\u{1F200}', '\u{1F2FF}', "Umschlossene CJK-Zeichen, Zusatz"),
    (
        '\u{1F300}',
        '\u{1F5FF}',
        "Verschiedene piktografische Symbole",
    ),
    ('\u{1F600}', '\u{1F64F}', "Smileys"),
    ('\u{1F650}', '\u{1F67F}', "Ziersymbole"),
    ('\u{1F680}', '\u{1F6FF}', "Verkehrs- und Kartensymbole"),
    ('\u{1F700}', '\u{1F77F}', "Alchemistische Symbole"),
    ('\u{1F780}', '\u{1F7FF}', "Geometrische Formen, Erweiterung"),
    ('\u{1F800}', '\u{1F8FF}', "Pfeile, Zusatz C"),
    ('\u{1F900}', '\u{1F9FF}', "Symbole und Piktogramme, Zusatz"),
    ('\u{1FA00}', '\u{1FA6F}', "Chess Symbols"),
    (
        '\u{1FA70}',
        '\u{1FAFF}',
        "Symbols and Pictographs Extended-A",
    ),
    ('\u{1FB00}', '\u{1FBFF}', "Symbols for Legacy Computing"),
    // (
    //     '\u{20000}',
    //     '\u{2A6DF}',
    //     "Vereinheitlichte CJK-Ideogramme, Erweiterung B",
    // ),
    // (
    //     '\u{2A700}',
    //     '\u{2B73F}',
    //     "Vereinheitlichte CJK-Ideogramme, Erweiterung C",
    // ),
    // (
    //     '\u{2B740}',
    //     '\u{2B81F}',
    //     "Vereinheitlichte CJK-Ideogramme, Erweiterung D",
    // ),
    // (
    //     '\u{2B820}',
    //     '\u{2CEAF}',
    //     "Vereinheitlichte CJK-Ideogramme, Erweiterung E",
    // ),
    // (
    //     '\u{2CEB0}',
    //     '\u{2EBEF}',
    //     "Vereinheitlichte CJK-Ideogramme, Erweiterung F",
    // ),
    // (
    //     '\u{2F800}',
    //     '\u{2FA1F}',
    //     "CJK-Kompatibilitätsideogramme, Ergänzung",
    // ),
    // (
    //     '\u{30000}',
    //     '\u{3134F}',
    //     "CJK Unified Ideographs Extension G",
    // ),
    // ('\u{E0000}', '\u{E007F}', "Tags"),
    // ('\u{E0100}', '\u{E01EF}', "Variantenselektoren, Ergänzung"),
    (
        '\u{F0000}',
        '\u{FFFFF}',
        "Bereich zur privaten Nutzung, Ergänzung A",
    ),
    (
        '\u{100000}',
        '\u{10FFFF}',
        "Bereich zur privaten Nutzung, Ergänzung B",
    ),
];

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::CtEvent(event) = event {
        try_flow!(match &event {
            ct_event!(resized) => {
                Control::Changed
            }
            ct_event!(key press CONTROL-'q') => Control::Quit,

            ct_event!(keycode press F(1)) => {
                state.font_idx = (state.font_idx + 1) % ctx.fonts.len();
                let font = ctx.fonts[state.font_idx].as_str();
                debug!("set_font_family {:?}", font);
                ctx.set_font_family(font);
                Control::Changed
            }
            ct_event!(keycode press SHIFT-F(1)) => {
                state.font_idx = (state.font_idx.saturating_sub(1)) % ctx.fonts.len();
                let font = ctx.fonts[state.font_idx].as_str();
                debug!("set_font_family {:?}", font);
                ctx.set_font_family(font);
                Control::Changed
            }
            ct_event!(scroll down) | ct_event!(keycode press PageDown) => {
                if state.range_idx + 1 < BLOCKS.len() {
                    state.range_idx += 1;
                }
                Control::Changed
            }
            ct_event!(scroll up) | ct_event!(keycode press PageUp) => {
                if state.range_idx > 0 {
                    state.range_idx -= 1;
                }
                Control::Changed
            }
            ct_event!(keycode press Home) => {
                state.range_idx = 0;
                Control::Changed
            }
            ct_event!(keycode press End) => {
                state.range_idx = BLOCKS.len() - 1;
                Control::Changed
            }

            _ => Control::Continue,
        });
    }

    Ok(Control::Continue)
}

pub fn error(
    event: Error,
    _state: &mut Minimal,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    error!("{:?}", event);
    Ok(Control::Changed)
}

fn setup_logging() -> Result<(), Error> {
    let log_path = PathBuf::from("");
    let log_file = log_path.join("log.log");
    _ = fs::remove_file(&log_file);
    fern::Dispatch::new()
        .format(|out, message, record| {
            if record.target() == "rat_salsa_wgpu::framework" {
                out.finish(format_args!("{}", message)) //
            }
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file(&log_file)?)
        .apply()?;
    Ok(())
}
