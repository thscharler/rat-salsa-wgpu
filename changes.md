# 1.0.7

* fix: dependencies

# 1.0.6

* feat: make font-resizing a feature.
* refactor: cleanup shutdown with one more app state.
* refactor: cleanup polling extra event sources.

# 1.0.5

* refactor: remove Control::Blink. Can replace this with
a builtin timer in winit. Make Control exactly the same as
in rat-salsa. 

* feat: add RunConfig::blink(Duration) for the timer.

# 1.0.4

* fix: RedrawRequested event needs a Terminal::clear() for
correct redraw on linux.
* use WgpuBuilder sync build functions.

# 1.0.3

something strange with crates.io. get the deleted crate version?

# 1.0.0

Initial release, working quite nicely.