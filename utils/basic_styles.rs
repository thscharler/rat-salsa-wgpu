use anyhow::Error;
use crossterm::event::MouseEventKind;
use log::{debug, error};
use rat_event::{HandleEvent, Regular, ct_event, event_flow};
use rat_salsa_wgpu::events::CompositeWinitEvent;
use rat_salsa_wgpu::events::ConvertCrosstermEx;
use rat_salsa_wgpu::poll::PollBlink;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::palette::Colors;
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, WidgetStyle, create_salsa_theme};
use rat_widget::paragraph::{Paragraph, ParagraphState};
use rat_widget::statusline_stacked::StatusLineStacked;
use rat_widget::tabbed::{TabPlacement, TabType, Tabbed, TabbedState};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::{Modifier, Style, Stylize};
use ratatui_core::text::{Line, Span, Text};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use std::fs;
use std::path::PathBuf;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key, SmolStr};
use rat_wgpu::font::FontData;

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Imperial Shell");
    let mut global = Global::new(config, theme);
    let mut state = Minimal::default();

    run_tui(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::new(ConvertCrosstermEx::new())?
            .window_position(30, 30)
            .font_size(22.)
            .rapid_blink(1)
            .slow_blink(4)
            .poll(PollBlink::new(0, 250)),
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
        fonts.push("Fairfax".into());

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
    WEvent(CompositeWinitEvent),
    CtEvent(crossterm::event::Event),
}

impl From<crossterm::event::Event> for AppEvent {
    fn from(value: crossterm::event::Event) -> Self {
        AppEvent::CtEvent(value)
    }
}

impl From<CompositeWinitEvent> for AppEvent {
    fn from(value: CompositeWinitEvent) -> Self {
        AppEvent::WEvent(value)
    }
}

#[derive(Debug, Default)]
pub struct Minimal {
    pub mouse_event: Option<crossterm::event::MouseEvent>,
    pub font_idx: usize,
    pub tabbed: TabbedState,
    pub para: ParagraphState,
}

pub fn init(state: &mut Minimal, _ctx: &mut Global) -> Result<(), Error> {
    state.tabbed.select(Some(0));
    state.tabbed.focus.set(true);
    Ok(())
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<(), Error> {
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1), //
        Constraint::Length(1),
    ])
    .split(area);

    buf.set_style(area, ctx.theme.style_style(Style::CONTAINER_BASE));

    Text::from_iter([
        Line::from(""),
        Line::from(format!("** {} **", ctx.font_family())),
        Line::from(""),
    ])
    .render(layout[0], buf);

    Tabbed::new()
        .block(Block::bordered())
        .styles(ctx.theme.style(WidgetStyle::TABBED))
        .placement(TabPlacement::Left)
        .tab_type(TabType::Attached)
        .tabs([
            "Current",
            "Basic",
            "Blink",
            "Extra",
            "Ligature",
            "A-Z",
            "Combining",
            "Arabic",
            "Other",
            "Arabic 2",
            "Arabic 3",
        ])
        .render(layout[1], buf, &mut state.tabbed);

    match state.tabbed.selected().unwrap_or(0) {
        1 => Text::from_iter([
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("bold").style(Style::new().add_modifier(Modifier::BOLD)),
                Span::from("]"),
            ]),
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("italic").style(Style::new().add_modifier(Modifier::ITALIC)),
                Span::from("]"),
            ]),
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("underlined").style(Style::new().add_modifier(Modifier::UNDERLINED)),
                Span::from("]"),
            ]),
        ])
        .render(state.tabbed.widget_area, buf),

        2 => Text::from_iter([
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("slow_blink").style(Style::new().add_modifier(Modifier::SLOW_BLINK)),
                Span::from("]"),
            ]),
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("rapid_blink").style(Style::new().add_modifier(Modifier::RAPID_BLINK)),
                Span::from("]"),
            ]),
        ])
        .render(state.tabbed.widget_area, buf),

        3 => Text::from_iter([
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("dim").style(Style::new().add_modifier(Modifier::DIM)),
                Span::from("]"),
            ]),
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("reversed").style(Style::new().add_modifier(Modifier::REVERSED)),
                Span::from("]"),
            ]),
            Line::from(""),
            Line::from_iter([
                Span::from(" hidden:["),
                Span::from("hidden").style(Style::new().add_modifier(Modifier::HIDDEN)),
                Span::from("]"),
            ]),
            Line::from(""),
            Line::from_iter([
                Span::from(" ["),
                Span::from("crossed_out").style(Style::new().add_modifier(Modifier::CROSSED_OUT)),
                Span::from("]"),
            ]),
        ])
        .render(state.tabbed.widget_area, buf),

        4 => Text::from_iter([
            Line::from(""),
            Line::from_iter([
                Span::from(" ligature test dots: 3:[...] "),
                Span::from("2:[..] "),
                Span::from("1:[.] "),
            ]),
            Line::from(""),
            Line::from_iter([
                Span::from(" ligature test slash: 3:[///] "),
                Span::from("2:[//] "),
                Span::from("1:[/] "),
            ]),
        ])
        .render(state.tabbed.widget_area, buf),

        5 => Text::from_iter([
            Line::from(""), //
            Line::from(" ABCDEFGHIJKLMNOPQRSTUVWXYZ "),
        ])
        .render(state.tabbed.widget_area, buf),

        6 => Text::from_iter([
            Line::from(""), //
            Line::from(""), //
            Line::from(""), //
            Line::from(""), //
            Line::from(""), //
            Line::from(" H̴̢͕̠͖͇̻͓̙̞͔͕͓̰͋͛͂̃̌͂͆͜͠ "),
        ])
        .render(state.tabbed.widget_area, buf),

        0 | 7 => Text::from_iter([
            Line::from(""), //
            Line::from("\u{2068}مرحبا بالعالم\u{2069}"),
        ])
        .render(state.tabbed.widget_area, buf),

        8 => Text::from_iter([
            Line::from(""), //
            Line::from("Ｈｅｌｌｏ, ｗｏｒｌｄ!"),
        ])
        .render(state.tabbed.widget_area, buf),

        9 => Text::from_iter([
            Line::from(""),
            Line::from("with isolate"),
            Line::from("\u{2068}Hello World! مرحبا بالعالم 0123456789000000000\u{2069}"),
            Line::from(""),
            Line::from("without isolate"),
            Line::from("Hello World! مرحبا بالعالم 0123456789000000000"),
            Line::from("forced ltr"),
            Line::from("\u{202D}Hello World! مرحبا بالعالم 0123456789000000000"),
            Line::from("forced rtl"),
            Line::from("\u{202E}Hello World! مرحبا بالعالم 0123456789000000000\u{202D}"),
            Line::from("rtl embedding"),
            Line::from("Hello World!\u{202B}مرحبا بالعالم  \u{202C} 0123456789000000000 "),
        ])
        .render(state.tabbed.widget_area, buf),

        10 => Paragraph::new(Text::from_iter([
            Line::from(""),
            Line::from("with isolate"),
            Line::from(vec![
                "\u{2068}Hello World!".green(),
                "مرحبا بالعالم".blue(),
                "0123456789\u{2069}".dim(),
            ]),
            Line::from(""),
            Line::from("without isolate"),
            Line::from(vec![
                "Hello World!".green(),
                "مرحبا بالعالم".blue(),
                "0123456789".dim(),
            ]),
        ]))
        .render(state.tabbed.widget_area, buf, &mut state.para),

        _ => {}
    }

    // Status
    let status_color_0 = ctx.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 3);
    let status_area = layout[2];
    StatusLineStacked::new()
        .style(ctx.theme.style(Style::STATUS_BASE))
        .center_margin(1)
        .end(
            if let Some(mouse_event) = &state.mouse_event {
                Span::from(format!(
                    "{}|{}: {:?}",
                    mouse_event.column, mouse_event.row, mouse_event.kind
                ))
            } else {
                Span::from("no event")
            }
            .style(status_color_0),
            Span::from(" "),
        )
        .render(status_area, buf);

    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::WEvent(event) = event {
        event_flow!(match &event.event {
            WindowEvent::Resized(_) => {
                Control::Changed
            }
            WindowEvent::KeyboardInput {
                event: winevent, ..
            } => {
                if winevent.state == ElementState::Pressed
                    && event.state.ctrl_pressed()
                    && winevent.logical_key == Key::Character(SmolStr::new_static("q"))
                {
                    Control::Quit
                } else {
                    Control::Continue
                }
            }
            _ => Control::Continue,
        });
    }

    if let AppEvent::CtEvent(event) = event {
        event_flow!(match &event {
            ct_event!(resized) => {
                Control::Changed
            }
            ct_event!(key press CONTROL-'q') => Control::Quit,
            ct_event!(keycode press F(1)) => {
                next_font(state, ctx)
            }
            ct_event!(keycode press SHIFT-F(1)) => {
                prev_font(state, ctx)
            }
            ct_event!(keycode press F(2)) => {
                incr_font(state, ctx)
            }
            ct_event!(keycode press SHIFT-F(2)) => {
                decr_font(state, ctx)
            }
            _ => Control::Continue,
        });

        event_flow!(state.tabbed.handle(event, Regular));

        if let crossterm::event::Event::Mouse(m) = event {
            if m.kind != MouseEventKind::Moved {
                state.mouse_event = Some(m.clone());
                ctx.queue(Control::Changed)
            }
        }
    }

    Ok(Control::Continue)
}

fn incr_font(_state: &mut Minimal, ctx: &mut Global) -> Control<AppEvent> {
    ctx.set_font_size(ctx.font_size() + 1.0);
    Control::Changed
}

fn decr_font(_state: &mut Minimal, ctx: &mut Global) -> Control<AppEvent> {
    ctx.set_font_size(ctx.font_size() - 1.0);
    Control::Changed
}

fn next_font(state: &mut Minimal, ctx: &mut Global) -> Control<AppEvent> {
    if state.font_idx + 1 < ctx.fonts.len() {
        state.font_idx += 1;
    } else {
        state.font_idx = 0;
    }
    let font = ctx.fonts[state.font_idx].as_str();
    debug!("set_font {:?}", font);
    ctx.set_font_family(font);
    Control::Changed
}

fn prev_font(state: &mut Minimal, ctx: &mut Global) -> Control<AppEvent> {
    if state.font_idx > 0 {
        state.font_idx -= 1;
    } else {
        state.font_idx = ctx.fonts.len().saturating_sub(1);
    }
    let font = ctx.fonts[state.font_idx].as_str();
    debug!("set_font {:?}", font);
    ctx.set_font_family(font);
    Control::Changed
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
    let log_file = log_path.join("basic_styles.log");
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
