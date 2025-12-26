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

## RunConfig

Usually all you have to do to switch is use the RunConfig
provided by rat-salsa-wgpu, which has a different API to accomodate 
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

> This is 0.1 btw. 

## Dual use

If you want to compile with either rat-salsa or rat-salsa-wgpu I found 
this approach.

* define two features 
 
```
[features]
default = ["wgpu"]
wgpu = ["dep:rat-salsa-wgpu"]
crossterm = ["dep:rat-salsa"]
```
and use the crates optionally. 

```
rat-salsa = { version = "3.0", optional = true }
rat-salsa-wgpu = { version = "0.1", optional = true }
```

In your main 

```
#[cfg(feature = "crossterm")]
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

The first two fonts are embedded when you use the feature flags `fallback_emoji_font`
and `fallback_symbol_font` and are always included when setting a font-family.

The third font is always embedded and used as a absolute fallback.

## Icons

If you want to use an icon, there is `img_icon` in the examples, 
that will dump the image as a raw rgba file that can be directly `include!`d. 


![image][refFilesGif]
![image][refMDEditGif]


[refOpenMoji]: https://github.com/hfg-gmuend/openmoji/tree/master/font/OpenMoji-black-glyf

[refFilesGif]: https://github.com/thscharler/rat-salsa/blob/master/rat-salsa-wgpu/files.gif?raw=true

[refMDEditGif]: https://github.com/thscharler/rat-salsa/blob/master/rat-salsa-wgpu/mdedit.gif?raw=true

[ratatui-wgpu]: https://github.com/Jesterhearts/ratatui-wgpu

[rat-salsa]: https://github.com/thscharler/rat-salsa


