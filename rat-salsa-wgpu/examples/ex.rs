use anyhow::Error;
use log::error;
use rat_event::try_flow;
use rat_focus::{FocusBuilder, impl_has_focus};
use rat_salsa_wgpu::{Control, SalsaAppContext, SalsaContext};
use rat_salsa_wgpu::{RunConfig, run_wgpu};
use rat_theme4::palette::Colors;
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, WidgetStyle, create_salsa_theme};
use rat_widget::menu::{MenuLine, MenuLineState};
use rat_widget::msgdialog::{MsgDialog, MsgDialogState};
use rat_widget::statusline_stacked::StatusLineStacked;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{StatefulWidget, Widget};
use std::fs;
use std::path::PathBuf;
use winit::event::{ElementState, Modifiers, WindowEvent};
use winit::keyboard::{Key, SmolStr};

pub fn main() -> Result<(), Error> {
    setup_logging()?;

    let config = Config::default();
    let theme = create_salsa_theme("Imperial Shell");
    let mut global = Global::new(config, theme);
    let mut state = Minimal::default();

    run_wgpu(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::default()?
            .font_family("Courier New"),
    )?;

    Ok(())
}

// fn create_fonts() {
//     let mut fontdb = Database::new();
//     fontdb.load_system_fonts();
//
//     let fonts = fontdb
//         .faces()
//         .filter_map(|info| {
//             if info.monospaced {
//                 dbg!(info);
//             }
//             if (info.monospaced
//                 || info.post_script_name.contains("Emoji")
//                 || info.post_script_name.contains("emoji"))
//                 && info.index == 0
//             {
//                 Some(info.id)
//             } else {
//                 None
//             }
//         })
//         .collect::<Vec<_>>();
//
//     let fonts = fonts
//         .into_iter()
//         .filter_map(|id| fontdb.with_face_data(id, |d, _| d.to_vec()))
//         .collect::<Vec<_>>();
//
//     let fonts = fonts
//         .iter()
//         .filter_map(|d| Font::new(d))
//         .collect::<Vec<_>>();
//
// }

/// Globally accessible data/state.
pub struct Global {
    // the salsa machinery
    ctx: SalsaAppContext<AppEvent, Error>,

    pub cfg: Config,
    pub theme: SalsaTheme,
    pub status: String,
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
}

impl From<(WindowEvent, Modifiers)> for AppEvent {
    fn from(value: (WindowEvent, Modifiers)) -> Self {
        AppEvent::Event(value)
    }
}

#[derive(Debug, Default)]
pub struct Minimal {
    pub menu: MenuLineState,
    pub error_dlg: MsgDialogState,
}

impl_has_focus!(menu for Minimal);

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
        Constraint::Fill(61), //
        Constraint::Fill(39),
    ])
    .split(layout[1]);

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
    let status_color_1 = ctx.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 3);
    let status_color_2 = ctx.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 2);

    StatusLineStacked::new()
        .style(ctx.theme.style(Style::STATUS_BASE))
        .center_margin(1)
        .center(Line::from(ctx.status.as_str()))
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

pub fn init(state: &mut Minimal, ctx: &mut Global) -> Result<(), Error> {
    ctx.set_focus(FocusBuilder::build_for(state));
    ctx.focus().first();
    Ok(())
}

pub fn event(
    event: &AppEvent,
    _state: &mut Minimal,
    _ctx: &mut Global,
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

        // try_flow!({
        //     if state.error_dlg.active() {
        //         state.error_dlg.handle(event, Dialog).into()
        //         Control::Continue
        //     } else {
        //         Control::Continue
        //     }
        // });

        // ctx.handle_focus(event);

        // try_flow!(match state.menu.handle(event, Regular) {
        //     MenuOutcome::Activated(0) => Control::Quit,
        //     v => v.into(),
        // });
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
    let log_path = PathBuf::from("../..");
    let log_file = log_path.join("log.log");
    _ = fs::remove_file(&log_file);
    fern::Dispatch::new()
        .format(|out, message, _record| {
            out.finish(format_args!("{}", message)) //
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file(&log_file)?)
        .apply()?;
    Ok(())
}
