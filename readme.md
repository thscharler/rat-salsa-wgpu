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

This is 0.1 btw. 


![image][refFilesGif]
![image][refMDEditGif]

[refFilesGif]: https://github.com/thscharler/rat-salsa/blob/master/rat-salsa-wgpu/files.gif?raw=true

[refMDEditGif]: https://github.com/thscharler/rat-salsa/blob/master/rat-salsa-wgpu/mdedit.gif?raw=true

[ratatui-wgpu]: https://github.com/Jesterhearts/ratatui-wgpu

[rat-salsa]: https://github.com/thscharler/rat-salsa


