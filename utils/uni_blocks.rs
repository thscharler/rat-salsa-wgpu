use crate::ex3_data::BLOCKS;
use crate::glyph_info::{GlyphInfo, GlyphInfoState};
use crate::glyphs::{Glyphs, GlyphsState};
use anyhow::Error;
use log::{debug, error};
use rat_event::{HandleEvent, Outcome, Popup, Regular, ct_event, event_flow};
use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
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
use rat_theme4::{StyleName, WidgetStyle, create_salsa_theme};
use rat_widget::checkbox::{Checkbox, CheckboxState};
use rat_widget::choice::{Choice, ChoiceState};
use rat_widget::event::{ChoiceOutcome, SliderOutcome, TextOutcome};
use rat_widget::paired::{Paired, PairedWidget};
use rat_widget::popup::Placement;
use rat_widget::scrolled::Scroll;
use rat_widget::slider::{Slider, SliderState};
use rat_widget::text_input_mask::{MaskedInput, MaskedInputState};
use rat_widget::view::{View, ViewState};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use std::fs;
use std::path::PathBuf;

mod uni_blocks_data;

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Nord");
    let mut global = Global::new(config, theme);
    let mut state = Minimal::new();

    run_tui(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::new(ConvertCrossterm::new())?
            .window_title("uni-blocks")
            .window_position(winit::dpi::PhysicalPosition::new(30, 30))
            .font_family("Overpass Mono")
            .font_size(23.)
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
    pub blocks: &'static [&'static str],
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
            blocks: BLOCKS,
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

#[derive(Debug)]
pub struct Minimal {
    pub fonts: ChoiceState<usize>,
    pub font_size: SliderState<usize>,
    pub blocks: ChoiceState<usize>,
    pub underline: CheckboxState,
    pub combining_base: MaskedInputState,

    pub view: ViewState,
    pub glyphs: GlyphsState,
    pub glyphinfo: GlyphInfoState,
}

impl Minimal {
    pub fn new() -> Self {
        Self {
            fonts: Default::default(),
            font_size: Default::default(),
            blocks: Default::default(),
            underline: Default::default(),
            combining_base: MaskedInputState::new().with_mask("_").expect("valid mask"),
            view: Default::default(),
            glyphs: Default::default(),
            glyphinfo: Default::default(),
        }
    }
}

impl HasFocus for Minimal {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.widget(&self.fonts);
        builder.widget(&self.font_size);
        builder.widget(&self.blocks);
        builder.widget(&self.underline);
        builder.widget(&self.combining_base);
        builder.widget(&self.glyphs);
    }

    fn focus(&self) -> FocusFlag {
        unimplemented!()
    }

    fn area(&self) -> Rect {
        unimplemented!()
    }
}

pub fn init(state: &mut Minimal, ctx: &mut Global) -> Result<(), Error> {
    ctx.set_focus(FocusBuilder::build_for(state));
    ctx.focus().focus(&state.glyphs);

    if let Some(font_idx) = ctx
        .fonts
        .iter()
        .position(|v| v.as_str() == &ctx.font_family())
    {
        state.fonts.set_value(font_idx);
    }
    state.combining_base.set_value("y");
    state.glyphinfo.set_font_family(&ctx.font_family());

    Ok(())
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<(), Error> {
    let vlayout = Layout::vertical([
        Constraint::Length(5), //
        Constraint::Fill(1),   //
    ])
    .split(area);
    let hlayout = Layout::horizontal([
        Constraint::Percentage(61), //
        Constraint::Percentage(39),
    ])
    .spacing(1)
    .split(vlayout[1]);

    buf.set_style(area, ctx.theme.style_style(Style::CONTAINER_BASE));

    Span::from(" :: ").render(Rect::new(area.x, area.y, 4, 1), buf);

    let font_area = Rect::new(area.x + 6, area.y, 40, 1);
    let (font, font_popup) = Choice::new()
        .items(ctx.fonts.iter().enumerate().map(|(n, v)| (n, v.as_str())))
        .popup_len(4)
        .popup_placement(Placement::Right)
        .styles(ctx.theme.style(WidgetStyle::CHOICE))
        .into_widgets();
    font.render(font_area, buf, &mut state.fonts);

    let fontsize_area = Rect::new(area.x + 47, area.y, 20, 1);
    Slider::new()
        .range((7, 59))
        .step(1)
        .upper_bound(ctx.font_size().to_string())
        .styles(ctx.theme.style(WidgetStyle::SLIDER))
        .render(fontsize_area, buf, &mut state.font_size);

    let blocks_area = Rect::new(area.x + 6, area.y + 1, 40, 1);
    let (blocks, blocks_popup) = Choice::new()
        .items(ctx.blocks.iter().enumerate().map(|(n, v)| (n, *v)))
        .popup_len(4)
        .popup_placement(Placement::Right)
        .popup_y_offset(-2)
        .styles(ctx.theme.style(WidgetStyle::CHOICE))
        .into_widgets();
    blocks.render(blocks_area, buf, &mut state.blocks);

    let underline_area = Rect::new(area.x + 6, area.y + 2, 15, 1);
    Checkbox::new()
        .text("underline")
        .styles(ctx.theme.style(WidgetStyle::CHECKBOX))
        .render(underline_area, buf, &mut state.underline);

    let combining_area = Rect::new(area.x + 47, area.y + 2, 15, 1);
    Paired::new_labeled(
        "c-base",
        MaskedInput::new().styles(ctx.theme.style(WidgetStyle::TEXT)),
    )
    .render(combining_area, buf, &mut state.combining_base);

    let block = ctx.blocks[state.blocks.value()];
    let block = unic_ucd::BlockIter::new()
        .find(|v| v.name == block)
        .expect("block");

    let blockrange_area = Rect::new(area.x + 47, area.y + 1, 25, 1);
    Line::from(format!(
        "{:#5x} - {:#5x}",
        block.range.low as u32, block.range.high as u32
    ))
    .render(blockrange_area, buf);

    let glyphs = Glyphs::new()
        .style(ctx.theme.style(Style::DOCUMENT_BASE))
        .codepoint_style(ctx.theme.p.high_bg_style(Colors::Yellow, Colors::Green, 6))
        .combining_base(state.combining_base.text())
        .focus_style(ctx.theme.style(Style::FOCUS))
        .underline(state.underline.value())
        .start(block.range.low)
        .end(block.range.high);

    let mut view_buf = View::new()
        .view_height(glyphs.height())
        .view_width(glyphs.width())
        .vscroll(Scroll::new())
        .hscroll(Scroll::new())
        .styles(ctx.theme.style(WidgetStyle::VIEW))
        .into_buffer(hlayout[0], &mut state.view);

    let glyphs_area = Rect::new(0, 0, glyphs.width(), glyphs.height());
    view_buf.render(glyphs, glyphs_area, &mut state.glyphs);

    view_buf.finish(buf, &mut state.view);

    if let Some(cc) = state.glyphs.codepoint.get(state.glyphs.selected) {
        GlyphInfo::new()
            .style(ctx.theme.style(Style::CONTAINER_BASE))
            .cc(*cc)
            .combining_base(state.combining_base.text())
            .render(hlayout[1], buf, &mut state.glyphinfo);
    }

    // popup
    font_popup.render(font_area, buf, &mut state.fonts);
    blocks_popup.render(blocks_area, buf, &mut state.blocks);

    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::CtEvent(event) = event {
        ctx.set_focus(FocusBuilder::rebuild_for(state, ctx.take_focus()));
        ctx.handle_focus(event);

        match event {
            ct_event!(resized) => event_flow!({
                state.font_size.set_value(ctx.font_size() as usize);
                Control::Changed
            }),
            ct_event!(key press CONTROL-'q') => event_flow!(Control::Quit),

            ct_event!(keycode press F(1)) => event_flow!({
                let v = state.fonts.value();
                if v + 1 < ctx.fonts.len() {
                    state.fonts.set_value(v + 1);
                    change_font_family(state, ctx)?
                } else {
                    Control::Continue
                }
            }),
            ct_event!(keycode press SHIFT-F(1)) => event_flow!({
                let v = state.fonts.value();
                if v > 0 {
                    state.fonts.set_value(v - 1);
                    change_font_family(state, ctx)?
                } else {
                    Control::Continue
                }
            }),

            ct_event!(keycode press F(2)) => event_flow!({
                let v = state.font_size.value();
                if v < state.font_size.range.1 {
                    state.font_size.set_value(v + 1);
                    ctx.set_font_size(v as f64 + 1.0);
                    Control::Changed
                } else {
                    Control::Continue
                }
            }),
            ct_event!(keycode press SHIFT-F(2)) => event_flow!({
                let v = state.font_size.value();
                if v > state.font_size.range.0 {
                    state.font_size.set_value(v - 1);
                    ctx.set_font_size(v as f64 - 1.0);
                    Control::Changed
                } else {
                    Control::Continue
                }
            }),

            ct_event!(keycode press F(3)) => event_flow!({
                let v = state.blocks.value();
                if v + 1 < ctx.blocks.len() {
                    state.blocks.set_value(v + 1);
                }
                Control::Changed
            }),
            ct_event!(keycode press SHIFT-F(3)) => event_flow!({
                let v = state.blocks.value();
                if v > 0 {
                    state.blocks.set_value(v - 1);
                }
                Control::Changed
            }),

            ct_event!(keycode press F(4)) => event_flow!({
                state.underline.flip_checked();
                Control::Changed
            }),

            _ => {}
        }

        event_flow!(match state.fonts.handle(event, Popup) {
            ChoiceOutcome::Value => {
                change_font_family(state, ctx)?
            }
            r => r.into(),
        });
        event_flow!(state.blocks.handle(event, Popup));

        event_flow!(match state.font_size.handle(event, Regular) {
            SliderOutcome::Value => {
                let v = state.font_size.value();
                ctx.set_font_size(v as f64);
                Control::Changed
            }
            r => Control::from(r),
        });
        event_flow!(state.underline.handle(event, Regular));
        event_flow!(state.combining_base.handle(event, Regular));
        event_flow!(state.view.handle(event, Regular));
        event_flow!(match state.glyphs.handle(event, Regular) {
            Outcome::Changed => {
                state.view.show_area(state.glyphs.selected_view());
                Control::Changed
            }
            r => r.into(),
        });

        if state.glyphs.is_focused() {
            match event {
                ct_event!(keycode press Home) => event_flow!({
                    state.blocks.set_value(0);
                    Control::Changed
                }),
                ct_event!(keycode press End) => event_flow!({
                    state.blocks.set_value(ctx.blocks.len().saturating_sub(1));
                    Control::Changed
                }),
                ct_event!(keycode press PageDown) => event_flow!({
                    let v = state.blocks.value();
                    if v + 1 < ctx.blocks.len() {
                        state.blocks.set_value(v + 1);
                    }
                    Control::Changed
                }),
                ct_event!(keycode press PageUp) => event_flow!({
                    let v = state.blocks.value();
                    if v > 0 {
                        state.blocks.set_value(v - 1);
                    }
                    Control::Changed
                }),
                _ => {}
            }
        }
    }

    Ok(Control::Continue)
}

fn change_font_family(state: &mut Minimal, ctx: &mut Global) -> Result<Control<AppEvent>, Error> {
    let font = ctx.fonts[state.fonts.value()].as_str();
    debug!("set_font_family {:?}", font);
    ctx.set_font_family(font);
    state.glyphinfo.set_font_family(font);
    Ok(Control::Changed)
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
    let log_file = log_path.join("../../log.log");
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

mod glyph_info {
    use rat_salsa_wgpu::font_data::FontData;
    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;
    use ratatui_core::style::Style;
    use ratatui_core::text::Text;
    use ratatui_core::widgets::{StatefulWidget, Widget};
    use rustybuzz::ttf_parser::GlyphId;
    use rustybuzz::{Face, ShapePlan, UnicodeBuffer, shape_with_plan, ttf_parser};
    use std::fmt::{Debug, Formatter};
    use std::marker::PhantomData;
    use unic_ucd::{CanonicalCombiningClass, Name};
    use unicode_script::UnicodeScript;

    pub struct GlyphInfo<'a> {
        cc: char,
        combining_base: &'a str,
        style: Style,
        _phantom: PhantomData<&'a ()>,
    }

    #[derive(Default)]
    pub struct GlyphInfoState {
        pub area: Rect,
        pub font: Option<Face<'static>>,
    }

    impl Debug for GlyphInfoState {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("GlyphInfoState").finish()
        }
    }

    impl<'a> GlyphInfo<'a> {
        pub fn new() -> Self {
            Self {
                cc: 'A',
                combining_base: " ",
                style: Default::default(),
                _phantom: Default::default(),
            }
        }

        pub fn style(mut self, style: Style) -> Self {
            self.style = style;
            self
        }

        pub fn combining_base(mut self, cc: &'a str) -> Self {
            self.combining_base = cc;
            self
        }

        pub fn cc(mut self, cc: char) -> Self {
            self.cc = cc;
            self
        }
    }

    impl<'a> StatefulWidget for GlyphInfo<'a> {
        type State = GlyphInfoState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            state.area = area;

            let mut txt = Text::default();
            txt.push_line(format!("codepoint {:05x}", self.cc as u32));
            if let Some(name) = Name::of(self.cc) {
                txt.push_line(format!("name {}", name));
            }
            txt.push_line("");
            txt.push_line(format!("script {:?}", self.cc.script()));
            txt.push_line(format!("script-ext {:?}", self.cc.script_extension()));
            txt.push_line(format!(
                "combining {}",
                CanonicalCombiningClass::of(self.cc).is_reordered()
            ));
            txt.push_line("");

            if let Some(font) = &state.font {
                txt.push_line(format!("font-height {}", font.height(),));
                txt.push_line(format!("ascender {}", font.ascender(),));
                txt.push_line(format!("descender {}", font.descender()));
                txt.push_line("");

                if let Some(gid) = font.glyph_index(self.cc) {
                    txt.push_line(format!("glyph {:?}", gid.0,));

                    let bb = font.glyph_bounding_box(gid).unwrap_or(ttf_parser::Rect {
                        x_min: 0,
                        y_min: 0,
                        x_max: 0,
                        y_max: 0,
                    });
                    txt.push_line(format!(
                        "bounding_box {:?} {:?}; {:?} {:?}",
                        bb.x_min, bb.x_max, bb.y_min, bb.y_max,
                    ));
                    txt.push_line(format!(
                        "advance h:{:?} v:{:?}",
                        font.glyph_hor_advance(gid).unwrap_or_default(),
                        font.glyph_ver_advance(gid).unwrap_or_default()
                    ));
                }
                txt.push_line("");

                let mut buffer = UnicodeBuffer::new();
                if CanonicalCombiningClass::of(self.cc).is_reordered() {
                    buffer.push_str(self.combining_base);
                }
                buffer.add(self.cc, 0);
                buffer.guess_segment_properties();

                let plan_cache = ShapePlan::new(
                    font,
                    buffer.direction(),
                    Some(buffer.script()),
                    buffer.language().as_ref(),
                    &[],
                );

                let glyph_buffer = shape_with_plan(font, &plan_cache, buffer);

                for (n, (info, position)) in glyph_buffer
                    .glyph_infos()
                    .iter()
                    .zip(glyph_buffer.glyph_positions().iter())
                    .enumerate()
                {
                    txt.push_line(format!(
                        "{:?}: {:?} {:?}",
                        n,
                        info.glyph_id,
                        font.glyph_name(GlyphId(info.glyph_id as _))
                            .unwrap_or("???")
                    ));

                    txt.push_line(format!(
                        "  : {:?} {:?}; {:?} {:?}",
                        position.x_offset,
                        position.x_advance,
                        position.y_offset,
                        position.y_advance
                    ));
                }
            } else {
                txt.push_line("no font");
            }

            buf.set_style(area, self.style);
            txt.render(area, buf);
        }
    }

    impl GlyphInfoState {
        pub fn new() -> Self {
            Self {
                area: Default::default(),
                font: None,
            }
        }

        pub fn set_font_family(&mut self, family: &str) {
            let font_ids = FontData
                .font_db()
                .faces()
                .filter_map(|info| {
                    if info.style != fontdb::Style::Normal || info.weight != fontdb::Weight::NORMAL
                    {
                        return None;
                    }
                    for (v, _) in &info.families {
                        if v.as_str() == family {
                            return Some(info.id);
                        }
                    }
                    None
                })
                .collect::<Vec<_>>();
            if let Some(fid) = font_ids.first() {
                if let Some(bytes) = FontData.load_font_bytes(*fid) {
                    self.font = Face::from_slice(bytes, 0);
                } else {
                    self.font = None;
                }
            } else {
                self.font = None;
            }
        }
    }
}

mod glyphs {
    use crossterm::event::Event;
    use log::debug;
    use rat_event::{FromBool, HandleEvent, Outcome, Regular, ct_event, event_flow};
    use rat_focus::{FocusBuilder, FocusFlag, HasFocus};
    use rat_widget::reloc::{RelocatableState, relocate_area};
    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;
    use ratatui_core::style::Style;
    use ratatui_core::text::Span;
    use ratatui_core::widgets::{StatefulWidget, Widget};
    use std::marker::PhantomData;
    use unic_ucd::CanonicalCombiningClass;

    const CLUSTER: u32 = 16;

    pub struct Glyphs<'a> {
        style: Style,
        codepoint_style: Style,
        focus_style: Style,
        start: char,
        end: char,
        underline: bool,
        combining_base: &'a str,
        _phantom: PhantomData<&'a ()>,
    }

    #[derive(Debug, Default)]
    pub struct GlyphsState {
        pub area: Rect,

        // selected. may not correspond with the vec's below.
        pub selected: usize,

        // codepoints displayed
        pub codepoint: Vec<char>,
        // areas for each codepoint in display coord.
        pub areas: Vec<Rect>,
        // areas for each codepoint in rendered coord.
        pub rendered: Vec<Rect>,

        pub focus: FocusFlag,
    }

    impl<'a> Glyphs<'a> {
        pub fn new() -> Self {
            Self {
                style: Default::default(),
                codepoint_style: Default::default(),
                focus_style: Default::default(),
                start: '\u{0000}',
                end: '\u{0000}',
                underline: true,
                combining_base: " ",
                _phantom: Default::default(),
            }
        }

        pub fn style(mut self, style: Style) -> Self {
            self.style = style;
            self
        }

        pub fn combining_base(mut self, cc: &'a str) -> Self {
            self.combining_base = cc;
            self
        }

        pub fn codepoint_style(mut self, style: Style) -> Self {
            self.codepoint_style = style;
            self
        }

        pub fn focus_style(mut self, style: Style) -> Self {
            self.focus_style = style;
            self
        }

        pub fn start(mut self, cc: char) -> Self {
            self.start = cc;
            self
        }

        pub fn end(mut self, cc: char) -> Self {
            self.end = cc;
            self
        }

        pub fn underline(mut self, underline: bool) -> Self {
            self.underline = underline;
            self
        }

        pub fn width(&self) -> u16 {
            9 + CLUSTER as u16 * 2
        }

        pub fn height(&self) -> u16 {
            let rows = (self.end as u32 - self.start as u32) / CLUSTER + 1;
            rows as u16 * 2
        }
    }

    impl RelocatableState for GlyphsState {
        fn relocate(&mut self, shift: (i16, i16), clip: Rect) {
            self.area.relocate(shift, clip);
            for (rendered, area) in self.rendered.iter().zip(self.areas.iter_mut()) {
                *area = relocate_area(*rendered, shift, clip);
            }
        }
    }

    impl HasFocus for GlyphsState {
        fn build(&self, builder: &mut FocusBuilder) {
            builder.leaf_widget(self);
        }

        fn focus(&self) -> FocusFlag {
            self.focus.clone()
        }

        fn area(&self) -> Rect {
            self.area
        }
    }

    impl<'a> StatefulWidget for Glyphs<'a> {
        type State = GlyphsState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            state.area = area;

            state.codepoint.clear();
            state.areas.clear();
            state.rendered.clear();

            buf.set_style(area, self.style);

            let glyph_style = if self.underline {
                self.codepoint_style.underlined()
            } else {
                self.codepoint_style
            };

            let mut tmp = String::new();
            for cc in self.start..=self.end {
                let off = cc as u32 - self.start as u32;

                let row = off / CLUSTER;
                let col = off % CLUSTER;

                if col == 0 {
                    let byte_span = format!("{:#06x} ", self.start as u32 + off,);
                    let head_area = Rect::new(area.x, area.y + 2 * row as u16, 14, 1);
                    Span::from(byte_span).render(head_area, buf);
                }

                let glyph_style = if state.is_focused() && state.selected == off as usize {
                    glyph_style.patch(self.focus_style)
                } else {
                    glyph_style
                };

                let cp_area = Rect::new(
                    area.x + 9 + 2 * col as u16, //
                    area.y + 2 * row as u16,
                    1,
                    1,
                );

                if let Some(cell) = buf.cell_mut(cp_area.as_position()) {
                    cell.set_style(glyph_style);

                    if cc as u32 >= 32 && cc as u32 != 127 {
                        tmp.clear();

                        if CanonicalCombiningClass::of(cc).is_reordered() {
                            tmp.push_str(self.combining_base);
                        }
                        tmp.push(cc);
                        cell.set_symbol(&tmp);
                    } else {
                        cell.set_symbol("?");
                    }
                }

                state.codepoint.push(cc);
                state.rendered.push(cp_area.intersection(area));
                state.areas.push(cp_area.intersection(area));
            }
        }
    }

    impl GlyphsState {
        pub fn new() -> Self {
            Self {
                area: Default::default(),
                selected: 0,
                codepoint: Default::default(),
                areas: Default::default(),
                rendered: Default::default(),
                focus: Default::default(),
            }
        }

        /// Returns the area for the codepoint in view-coords (rendered coords).
        pub fn selected_view(&self) -> Rect {
            if self.selected < self.codepoint.len() {
                debug!("found area");
                self.rendered[self.selected]
            } else {
                debug!("no area");
                Rect::default()
            }
        }

        pub fn first(&mut self) -> bool {
            let old_idx = self.selected;
            self.selected = 0;
            debug!("first {}", self.selected != old_idx);
            self.selected != old_idx
        }

        pub fn last(&mut self) -> bool {
            let old_idx = self.selected;
            self.selected = self.codepoint.len().saturating_sub(1);
            debug!("last {}", self.selected != old_idx);
            self.selected != old_idx
        }

        pub fn next(&mut self) -> bool {
            let old_idx = self.selected;
            if self.selected + 1 < self.codepoint.len() {
                self.selected += 1;
            } else {
                self.selected = self.codepoint.len().saturating_sub(1);
            }

            self.selected != old_idx
        }

        pub fn prev(&mut self) -> bool {
            let old_idx = self.selected;
            if self.selected > 0 {
                if self.selected < self.codepoint.len() {
                    self.selected -= 1;
                } else {
                    self.selected = self.codepoint.len().saturating_sub(1);
                }
            }

            self.selected != old_idx
        }

        pub fn up(&mut self) -> bool {
            let old_idx = self.selected;
            if self.selected >= CLUSTER as usize {
                if self.selected < self.codepoint.len() {
                    self.selected -= CLUSTER as usize;
                } else {
                    self.selected = self.codepoint.len().saturating_sub(1);
                }
            }

            self.selected != old_idx
        }

        pub fn down(&mut self) -> bool {
            let old_idx = self.selected;
            if (self.selected + CLUSTER as usize) < self.codepoint.len() {
                self.selected += CLUSTER as usize;
            } else if self.selected >= self.codepoint.len() {
                self.selected = self.codepoint.len().saturating_sub(1);
            }

            self.selected != old_idx
        }
    }

    impl HandleEvent<Event, Regular, Outcome> for GlyphsState {
        fn handle(&mut self, event: &Event, _qualifier: Regular) -> Outcome {
            if self.is_focused() {
                event_flow!(
                    return match event {
                        ct_event!(keycode press Home) => self.first().as_changed_continue(),
                        ct_event!(keycode press End) => self.last().as_changed_continue(),
                        ct_event!(keycode press Left) => self.prev().into(),
                        ct_event!(keycode press Right) => self.next().into(),
                        ct_event!(keycode press Up) => self.up().into(),
                        ct_event!(keycode press Down) => self.down().into(),
                        _ => Outcome::Continue,
                    }
                );
            }

            match event {
                ct_event!(mouse down Left for x,y) => event_flow!(
                    return {
                        if let Some(idx) = rat_event::util::item_at(&self.areas, *x, *y) {
                            self.selected = idx;
                            Outcome::Changed
                        } else {
                            Outcome::Continue
                        }
                    }
                ),
                _ => {}
            }

            Outcome::Continue
        }
    }
}
