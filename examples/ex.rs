use anyhow::Error;
use log::{debug, error};
use rat_event::{Dialog, HandleEvent, Regular, ct_event, event_flow, try_flow};
use rat_focus::{FocusBuilder, impl_has_focus};
use rat_salsa_wgpu::event::{QuitEvent, RenderedEvent};
use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrossterm;
use rat_salsa_wgpu::poll::{PollQuit, PollRendered, PollTasks, PollTick, PollTimers, PollTokio};
use rat_salsa_wgpu::timer::{TimeOut, TimerDef};
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::palette::Colors;
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, WidgetStyle, create_salsa_theme};
use rat_widget::event::MenuOutcome;
use rat_widget::menu::{MenuLine, MenuLineState};
use rat_widget::msgdialog::{MsgDialog, MsgDialogState};
use rat_widget::statusline_stacked::StatusLineStacked;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{StatefulWidget, Widget};
use ratatui_wgpu::ColorTable;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use winit::event::{ElementState, Modifiers, WindowEvent};
use winit::keyboard::{Key, SmolStr};

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Shell");
    debug!("{:?}", theme);
    let mut global = Global::new(config, theme);
    let mut state = Minimal::default();

    let rt = tokio::runtime::Runtime::new()?;

    run_tui(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::new(ConvertCrossterm::new())?
            .window_position(winit::dpi::PhysicalPosition::new(30, 30))
            .font_family("Courier New")
            .font_size(20.)
            .poll(PollTick::new(0, 500))
            .poll(PollTimers::new())
            .poll(PollQuit)
            .poll(PollRendered)
            .poll(PollTasks::new(2))
            .poll(PollTokio::new(rt)),
    )?;

    Ok(())
}

/// Globally accessible data/state.
pub struct Global {
    // the salsa machinery
    ctx: SalsaAppContext<AppEvent, Error>,

    pub cfg: Config,
    pub theme: SalsaTheme,
    pub status: String,
    pub upsec: u64,
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
        Self {
            ctx: Default::default(),
            cfg,
            theme,
            status: Default::default(),
            upsec: Default::default(),
        }
    }
}

/// Configuration.
#[derive(Debug, Default)]
pub struct Config {}

#[derive(Debug)]
pub enum AppEvent {
    NoOp,
    Event((WindowEvent, Modifiers)),
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

impl From<(WindowEvent, Modifiers)> for AppEvent {
    fn from(value: (WindowEvent, Modifiers)) -> Self {
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
    pub menu: MenuLineState,
    pub mouse_event: Option<crossterm::event::MouseEvent>,
    pub font_idx: usize,
    pub color_idx: usize,
    pub error_dlg: MsgDialogState,
}

impl_has_focus!(menu for Minimal);

pub fn init(state: &mut Minimal, ctx: &mut Global) -> Result<(), Error> {
    ctx.set_focus(FocusBuilder::build_for(state));
    ctx.focus().first();

    ctx.add_timer(
        TimerDef::new()
            .repeat_forever()
            .timer(Duration::from_secs(1)),
    );

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
        Constraint::Length(1),
    ])
    .split(area);

    let status_layout = Layout::horizontal([
        Constraint::Fill(11), //
        Constraint::Fill(89),
    ])
    .split(layout[1]);

    Text::from_iter([
        if let Some(mouse_event) = &state.mouse_event {
            Line::from(format!(
                "{}|{}: {:?}",
                mouse_event.column, mouse_event.row, mouse_event.kind
            ))
        } else {
            Line::from("no event")
        },
        Line::from(""),
        Line::from("bold").style(Style::new().add_modifier(Modifier::BOLD)),
        Line::from("italic").style(Style::new().add_modifier(Modifier::ITALIC)),
        Line::from("dim").style(Style::new().add_modifier(Modifier::DIM)),
        Line::from("underlined").style(Style::new().add_modifier(Modifier::UNDERLINED)),
        Line::from("slow_blink").style(Style::new().add_modifier(Modifier::SLOW_BLINK)),
        Line::from("rapid_blink").style(Style::new().add_modifier(Modifier::RAPID_BLINK)),
        Line::from("reversed").style(Style::new().add_modifier(Modifier::REVERSED)),
        Line::from("hidden").style(Style::new().add_modifier(Modifier::HIDDEN)),
        Line::from("crossed_out").style(Style::new().add_modifier(Modifier::CROSSED_OUT)),
    ])
    .render(layout[0], buf);

    MenuLine::new()
        .styles(ctx.theme.style(WidgetStyle::MENU))
        .title("-!-")
        .item_parsed("_Quit")
        .render(status_layout[0], buf, &mut state.menu);

    if state.error_dlg.active() {
        MsgDialog::new()
            .styles(ctx.theme.style(WidgetStyle::MSG_DIALOG))
            .render(layout[0], buf, &mut state.error_dlg);
    }

    // Status
    let status_color_0 = ctx.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 3);
    let status_color_1 = ctx.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 2);
    let status_color_2 = ctx.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 1);

    StatusLineStacked::new()
        .style(ctx.theme.style(Style::STATUS_BASE))
        .center_margin(1)
        .center(Line::from(ctx.status.as_str()))
        .end(
            Span::from(ctx.upsec.to_string()).style(status_color_0),
            Span::from(" "),
        )
        .end(
            Span::from(format!(
                " R({:03}){:05} ",
                ctx.count(),
                format!("{:.0?}", ctx.last_render())
            ))
            .style(status_color_1),
            Span::from(" "),
        )
        .end_bare(
            Span::from(format!(" E{:05} ", format!("{:.0?}", ctx.last_event())))
                .style(status_color_2),
        )
        .render(status_layout[1], buf);

    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::Event(event) = event {
        try_flow!(match &event {
            (WindowEvent::Resized(_), _) => {
                Control::Changed
            }
            (WindowEvent::KeyboardInput { event, .. }, modifiers) => {
                if event.state == ElementState::Pressed
                    && modifiers.state().control_key()
                    && event.logical_key == Key::Character(SmolStr::new_static("q"))
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
        try_flow!(match &event {
            ct_event!(resized) => {
                debug!(">> resized");
                Control::Changed
            }
            ct_event!(key press CONTROL-'q') => Control::Quit,
            ct_event!(keycode press F(1)) => {
                static FONTS: &[&'static str] = &[
                    "Courier New", //
                    "Unknown",
                    "Cascadia Code",
                    "Cascadia Mono",
                    "Consolas",
                    "DejaVu Sans Mono",
                    "FiraCode Nerd Font Mono",
                    "JetBrainsMono Nerd Font Mono",
                    "Liberation Mono",
                    // "Lucida Console",
                    "Lucida Sans Typewriter",
                    "MS Gothic",
                    // "NSimSun",
                    // "SimSun-ExtB",
                    // "SimSun-ExtG",
                    "Source Code Pro",
                ];

                state.font_idx = (state.font_idx + 1) % FONTS.len();
                let font = FONTS[state.font_idx];
                ctx.status = format!("font {:?}", font);
                debug!("set_font {:?}", font);
                ctx.set_font_family(font);
                Control::Changed
            }
            ct_event!(keycode press F(2)) => {
                state.color_idx = (state.color_idx + 1) % 2;
                ctx.status = format!("color {}", state.color_idx);
                debug!("set_colors {:?}", state.color_idx);
                let t = match state.color_idx {
                    0 => COLORS1,
                    1 => COLORS2,
                    _ => unreachable!(),
                };
                ctx.terminal()
                    .borrow_mut()
                    .backend_mut()
                    .update_color_table(t);
                Control::Changed
            }
            _ => Control::Continue,
        });

        try_flow!({
            if state.error_dlg.active() {
                state.error_dlg.handle(event, Dialog).into()
            } else {
                Control::Continue
            }
        });

        ctx.handle_focus(event);

        if let crossterm::event::Event::Mouse(m) = event {
            event_flow!({
                state.mouse_event = Some(m.clone());
                Control::Changed
            });
        }

        try_flow!(match state.menu.handle(event, Regular) {
            MenuOutcome::Activated(0) => Control::Quit,
            v => v.into(),
        });
    }

    match event {
        AppEvent::TimeOut(t) => event_flow!({
            ctx.upsec = t.counter as u64;
            Control::Changed
        }),
        AppEvent::Quit => event_flow!(Control::Quit),
        _ => {}
    }

    // match event {
    //     AppEvent::Rendered => {
    //         ctx.set_focus(FocusBuilder::rebuild_for(state, ctx.take_focus()));
    //         Ok(Control::Continue)
    //     }
    //     AppEvent::Message(s) => {
    //         state.error_dlg.append(s.as_str());
    //         Ok(Control::Changed)
    //     }
    //     _ => Ok(Control::Continue),
    // }

    Ok(Control::Continue)
}

pub fn error(
    event: Error,
    state: &mut Minimal,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    error!("{:?}", event);
    state.error_dlg.append(format!("{:?}", &*event).as_str());
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

const COLORS1: ColorTable = ColorTable {
    BLACK: [0, 0, 0],
    RED: [170, 0, 0],
    GREEN: [0, 170, 0],
    YELLOW: [170, 85, 0],
    BLUE: [0, 0, 170],
    MAGENTA: [170, 0, 170],
    CYAN: [0, 170, 170],
    GRAY: [170, 170, 170],
    DARKGRAY: [85, 85, 85],
    LIGHTRED: [255, 85, 85],
    LIGHTGREEN: [85, 255, 85],
    LIGHTYELLOW: [255, 255, 85],
    LIGHTBLUE: [85, 85, 255],
    LIGHTMAGENTA: [255, 85, 255],
    LIGHTCYAN: [85, 255, 255],
    WHITE: [255, 255, 255],
};

const COLORS2: ColorTable = ColorTable {
    BLACK: [12, 12, 12],
    RED: [197, 15, 31],
    GREEN: [19, 161, 14],
    YELLOW: [193, 156, 0],
    BLUE: [0, 55, 218],
    MAGENTA: [136, 23, 152],
    CYAN: [58, 150, 221],
    GRAY: [204, 204, 204],
    DARKGRAY: [118, 118, 118],
    LIGHTRED: [231, 72, 86],
    LIGHTGREEN: [22, 198, 12],
    LIGHTYELLOW: [249, 241, 165],
    LIGHTBLUE: [59, 120, 255],
    LIGHTMAGENTA: [180, 0, 158],
    LIGHTCYAN: [97, 214, 214],
    WHITE: [242, 242, 242],
};
