![semver](https://img.shields.io/badge/semver-☑-FFD700)
![stable](https://img.shields.io/badge/stability-stable-8A2BE2)
[![crates.io](https://img.shields.io/crates/v/rat-salsa-wgpu.svg)](https://crates.io/crates/rat-salsa-wgpu)
[![Documentation](https://docs.rs/rat-salsa-wgpu/badge.svg)](https://docs.rs/rat-salsa-wgpu)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/license-APACHE-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
![](https://tokei.rs/b1/github/thscharler/rat-salsa-wgpu)

# rat-salsa (wgpu)

Implements the same API as [rat-salsa][rat-salsa], but uses
[rat-wgpu][rat-wgpu] as the backend.

Kudos to <https://github.com/Jesterhearts/ratatui-wgpu> which
was the base for rat-wgpu.

# Running it

To run a rat-salsa application with rat-salsa-wgpu you have
to switch out the `RunConfig` and `run_tui` from rat-salsa-wgpu.

## Dual use

If you want to compile with either rat-salsa or rat-salsa-wgpu
the best way today is to create two features and use them
to switch between the crates.

```
    #[cfg(feature = "term")]
    pub(crate) use rat_salsa;
    #[cfg(feature = "wgpu")]
    pub(crate) use rat_salsa_wgpu as rat_salsa;
```

You may still need some more `#[cfg]` to make your application work.

## Additions

### SalsaContext

* window() - access the underlying window
* font_size()/set_font_size()
* font_family()/set_font_family()
* cursor_style()/set_cursor_style()
* cursor_color()/set_cursor_color()
* set_fg_color() - default color
* set_bg_color() - default color
* image_buffer() - image rendering

### Control

Adds one event `Control::Blink`. This is uses together with
PollBlink to make blinking text and cursor work.

### Included Fonts

rat-wgpu can statically include fallbacks for a regular font a
symbol font and an emoji font.

## Examples

Are always a good starting point ...

[rat-wgpu]: https://github.com/thscharler/rat-wgpu

[rat-salsa]: https://github.com/thscharler/rat-salsa


