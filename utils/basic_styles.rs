use anyhow::Error;
use crossterm::event::MouseEventKind;
use log::{debug, error};
use rat_event::{Dialog, HandleEvent, Regular, ct_event, event_flow, try_flow};
use rat_focus::{FocusBuilder, impl_has_focus};
use rat_salsa_wgpu::event::{QuitEvent, RenderedEvent};
use rat_salsa_wgpu::event_type::CompositeWinitEvent;
use rat_salsa_wgpu::event_type::convert_crossterm::ConvertCrosstermEx;
use rat_salsa_wgpu::font_data::FontData;
use rat_salsa_wgpu::poll::{PollBlink, PollQuit, PollRendered, PollTimers};
use rat_salsa_wgpu::timer::TimeOut;
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_tui};
use rat_theme4::palette::Colors;
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, WidgetStyle, create_salsa_theme};
use rat_widget::event::MenuOutcome;
use rat_widget::menu::{MenuLine, MenuLineState};
use rat_widget::msgdialog::{MsgDialog, MsgDialogState};
use rat_widget::statusline_stacked::StatusLineStacked;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::{Modifier, Style, Stylize};
use ratatui_core::text::{Line, Span, Text};
use ratatui_core::widgets::{StatefulWidget, Widget};
use std::fs;
use std::path::PathBuf;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key, SmolStr};

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
            .window_position(winit::dpi::PhysicalPosition::new(30, 30))
            .font_family("Hack Nerd Font Mono")
            .font_size(22.)
            // .viewport(ratatui_wgpu::Viewport::Shrink { width: 40, height: 40 })
            // .bg_color(Color::Red)
            // .fg_color(Color::White)
            .rapid_blink(1)
            .slow_blink(4)
            .poll(PollBlink::new(0, 200))
            // .poll(PollTick::new(0, 500))
            // .poll(PollTimers::new())
            .poll(PollQuit)
            .poll(PollRendered),
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
            fonts: FontData.installed_fonts().clone(),
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
    pub menu: MenuLineState,
    pub mouse_event: Option<crossterm::event::MouseEvent>,
    pub font_idx: usize,
    pub error_dlg: MsgDialogState,
}

impl_has_focus!(menu for Minimal);

pub fn init(state: &mut Minimal, ctx: &mut Global) -> Result<(), Error> {
    ctx.set_focus(FocusBuilder::build_for(state));
    ctx.focus().first();

    // ctx.add_timer(
    //     TimerDef::new()
    //         .repeat_forever()
    //         .timer(Duration::from_secs(1)),
    // );

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

    buf.set_style(area, Style::new().white().on_dark_gray());

    Text::from_iter([
        Line::from(""),
        Line::from(format!("** {} **", ctx.font_family())),
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
        Line::from(" H̴̢͕̠͖͇̻͓̙̞͔͕͓̰͋͛͂̃̌͂͆͜͠").style(Style::new()),
        Line::from(" ...").style(Style::new()),
        Line::from(" ..").style(Style::new()),
        Line::from(" .").style(Style::new()),
        Line::from(" ///").style(Style::new()),
        Line::from(" ///").style(Style::new()),
        Line::from(" /").style(Style::new()),
    ])
    .render(layout[0], buf);

    let mut status_area = layout[1];
    let menu = MenuLine::new()
        .styles(ctx.theme.style(WidgetStyle::MENU))
        .title("-!-")
        .item_parsed("_Next font")
        .item_parsed("_Prev font")
        .item_parsed("_+Size")
        .item_parsed("_-Size")
        .item_parsed("_Quit");
    let m_len = menu.width();
    status_area.x = m_len;
    status_area.width = status_area.width.saturating_sub(m_len).max(15);
    menu.render(layout[1], buf, &mut state.menu);

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
        .render(status_area, buf);

    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Minimal,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    if let AppEvent::Event(event) = event {
        try_flow!(match &event.event {
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
        try_flow!(match &event {
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
            ct_event!(key press CONTROL-'+') => {
                incr_font(state, ctx)
            }
            ct_event!(key press CONTROL-'-') => {
                decr_font(state, ctx)
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

        try_flow!(match state.menu.handle(event, Regular) {
            MenuOutcome::Activated(0) => next_font(state, ctx),
            MenuOutcome::Activated(1) => prev_font(state, ctx),
            MenuOutcome::Activated(2) => incr_font(state, ctx),
            MenuOutcome::Activated(3) => decr_font(state, ctx),
            MenuOutcome::Activated(4) => Control::Quit,
            v => v.into(),
        });

        if let crossterm::event::Event::Mouse(m) = event {
            if m.kind != MouseEventKind::Moved {
                state.mouse_event = Some(m.clone());
                ctx.queue(Control::Changed)
            }
        }
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
    state: &mut Minimal,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    error!("{:?}", event);
    state.error_dlg.append(format!("{:?}", &*event).as_str());
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
