![semver](https://img.shields.io/badge/semver-☑-FFD700)
![stable](https://img.shields.io/badge/stability-stable-8A2BE2)
[![crates.io](https://img.shields.io/crates/v/rat-salsa-wgpu.svg)](https://crates.io/crates/rat-salsa-wgpu)
[![Documentation](https://docs.rs/rat-salsa-wgpu/badge.svg)](https://docs.rs/rat-salsa-wgpu)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/license-APACHE-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
![](https://tokei.rs/b1/github/thscharler/rat-salsa-wgpu)

# rat-salsa [wgpu]

Implements the same API as [rat-salsa][rat-salsa], but uses 
[ratatui-wgpu][ratatui-wgpu] as the backend.

## Running this

I'm still waiting for some feedback from upstream about the changes/fixes
I added to ratatui-wgpu. 

So for now there is a `main` branch that runs on par with 
jesterharts's ratatui-wgpu. This one has a lot of broken rendering and
is lacking ergonomic features. 

All my changes are in the `remastered` branch. If you want to report anything
try this branch first.

## Status 

I'm happy with what I can do with the remastered branch, and I'm currently 
waiting for some feedback. 

So this will stay as a github only repo for the foreseeable future.

I still stick a 1.0 to it, just to signal that I think it's fine but
with drawbacks.

## RunConfig

Usually all you have to do to switch is use the RunConfig
provided by rat-salsa-wgpu, which has a different API to accommodate 
for the different setting. 

```
RunConfig::new(ConvertCrossterm::new())?
            .font_family("JetBrainsMono Nerd Font Mono")
            .font_size(20.)
            .window_title("MD Edit")
            .rapid_blink_millis(200)
            .poll(PollRendered)
            .poll(PollTasks::default())
            .poll(PollTimers::default())
            .poll(PollRendered)
            .poll(PollQuit),
```

- ConvertCrossterm: Converts winit-events to crossterm events.
- font_family(): UI font ... 
- window_title(): Set the window title
- ...: There are more such settings. 

## SalsaContext

- Gives access to the underlying window. 
- Allows changing the font-family and font-size.

## Quirks

Except from the difficult status ...

But the font-rendering looks fine now. It'll even cope if you
accidentially use a variable width font and only look half-broken.

If you activate all the fallback fonts most things should render fine.
I don't use color-emojis and rather have more text-like ones, but you
can easily switch this out.

## Dual use

If you want to compile with either rat-salsa or rat-salsa-wgpu I found 
this approach.

* define two features 
 
```
[features]
default = ["wgpu"]
wgpu = ["dep:rat-salsa-wgpu"]
term = ["dep:rat-salsa"]
```
and use the crates optionally. 

```
rat-salsa = { version = "3.0", optional = true }
rat-salsa-wgpu = { version = "1.0", optional = true }
```

In your main 

```
#[cfg(feature = "term")]
pub(crate) use rat_salsa;
#[cfg(feature = "wgpu")]
pub(crate) use rat_salsa_wgpu as rat_salsa;
```

Where ever you are using rat-salsa, refer to the crate-wide alias.

```
use crate::rat_salsa::{Control, SalsaContext};
```

## Included Fonts

> This is currently pending, there are a few PR's waiting. 
> But there is a fallback font if you don't set anything. 

* [OpenMoji-black-glyf][refOpenMoji]  (CC-BY-SA-4.0 license)
* NotoSansSymbols2-Regular (OFL license)
* CascadiaMono-Regular (OFL License)

There is a feature flag for each of the fonts. They are all
active by default, but you can turn them off and save a few MB in 
binary size.

If you turn them all off, you need to set a font.

## Icons

If you want to use an icon, there is `img_icon` in the examples, 
that will dump the image as a raw rgba file that can be directly `include!`d. 


![image][refFilesGif]
![image][refMDEditGif]

## Diff rat-salsa

### SalsaContext

Adds functions only useful for the graphical context or simply not available for a TUI.

* window() - access the underlying window
* font_size()/set_font_size()
* font_family()/set_font_family()
* cursor_style()/set_cursor_style()
* cursor_color()/set_cursor_color()
* set_fg_color() - default color
* set_bg_color() - default color

### Control

* Control::Blink - One extra control to make the cursor blink and to enable blinking text.

  This is necessary to communicate the need for a targeted redraw of just the blinking
  things. ratatui-wgpu relies on an external source for time. 

  It's pretty useless for anything else, but it allows you to add `PollBlink`
  and adjust the timings for blinking. 

### run_tui and RunConfig

Those are completely different from rat-salsa but try to 
provide an analog api. 

The main event-loop runs as a winit-eventloop. All the extra event-sources
are run in a separate polling thread and communicate back using winit's user-event
feature. 

This allows it to run your application in one thread, eliminating the
need for unwanted Send/Sync, but it still adds a Send bound to a few things
to make this work. 


[refOpenMoji]: https://github.com/hfg-gmuend/openmoji/tree/master/font/OpenMoji-black-glyf

[refFilesGif]: https://github.com/thscharler/rat-salsa/blob/master/rat-salsa-wgpu/files.gif?raw=true

[refMDEditGif]: https://github.com/thscharler/rat-salsa/blob/master/rat-salsa-wgpu/mdedit.gif?raw=true

[ratatui-wgpu]: https://github.com/Jesterhearts/ratatui-wgpu

[rat-salsa]: https://github.com/thscharler/rat-salsa


