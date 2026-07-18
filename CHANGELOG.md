# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

1. Clamp opacity value in `set_opacity` method of `Node` to ensure it stays within the range of 0.0 to 1.0.

## [0.15.3] - 2026-07-14

### Fixed

1. Add `contains_children` method in `Focusable` trait to allow shadowing children in hit testing.

## [0.15.2] - 2026-07-08

### Changed

1. Change refresh rate when window moved to a different monitor.
2. Set window icon on Windows platform.

### Fixed

1. Errors caused by integer conversion issues.

## [0.15.1] - 2026-06-26

### Fixed

1. Crash when resuming from minimized state on Windows
2. Optimize redraw scheduling on native platforms by only requesting redraw when necessary
3. Disable auto destroy texture when `Texture` dropped to avoid potential use-after-free issues
4. Fix texture UV coordinates in shader

### Changed

1. Refactor shader renderer

### Added

1. Add prepared event for shader node

## [0.15.0] - 2026-06-15

### Added

1. Generic shader (including transition) support with many builtin transition effects.

### Fixed

1. Limit surface formats to Bgra8Unorm and Rgba8Unorm
2. Update snapshot format conversion to use TryFrom for better error handling
3. Enable logging adapter info on web
4. Fix an issue that `COPY_SRC` usage is not correctly detected on webgpu backend

## [0.14.1] - 2026-06-01

### Added

- Add local storage fallback for web platform when OPFS is not available
- Align the behavior of Back gesture on Android with close button on desktop platforms

### Fixed

- Fix an compatibility issue of video shader on WebGL backend causing crash
- Improve surface texture usage configuration
- Avoid crashes on WebGL backend on snapshot and backdrop pass
- Fix Android target compilation
- Fix a crash on Android caused by incorrect handling of setup function
- Use Fifo instead of Mailbox for surface texture on Android to avoid error logs
- Fix some file reading issues on Android
- Fix wrong stage size on Android
- Improve splash screen on some cases
- Crash when resume from background on Android

## [0.14.0] - 2026-05-22

### Added

- Implement story replacement and recovery planning

## [0.13.0] - 2026-05-20

### Added

- (Scenario) Support local context and add local variable management commands
- (Core) Add ReadFile command and ReadFormat type for file reading
- (Audio) Add LoadAndPlay command to AudioCommand

## [0.12.0] - 2026-05-09

### Breaking

- Introduce global volume control for audio instances via `setGlobalVolume` command, allowing real-time volume adjustment and wildcard matching for dynamic channels.

### Added

- Add checkpoint caching mechanism to let users rollback to previous line of scenario script, improving debugging experience.
- Seek implementation in core and kit, enabling fast jump to specific line of scenario script.

### Fixed

- Fix an issue where audio with fade-in enabled would start at full volume, by setting the initial volume to 0 when fade-in is enabled.
- Fix an issue where text nodes keep old text content when visible is set to false.
- Add stale flag to Audio struct to prevent playing already stopped audio instances.
- In `dispatch_event`, replace `set_timeout` with `queue_microtask` for web platform.

## [0.11.0] - 2026-04-16

### Added

1. Add wheel event handling and update related types
2. Implement backlog management with record, getRecords, and jumpToRecord commands
3. Add auto mode management with ticketing and barrier handling
4. Enhance audio plugin's `play` function to support `onStopped` callback and add `waitForEnd` option

### Fixed

1. Change `target_width` and `target_height` in `sprite` node from `Patch<u32>` to `Patch<f32>`
2. Enhance auto ticket management with pending state handling and barrier association
3. Make audio volume to be linear to match user expectation

## [0.10.0] - 2026-04-04

### Breaking

- Introduce dynamic dispatching for Node trait to reduce boilerplate code when creating new nodes.
- Introduce `Patch<T>` type for node properties to support partial updates and default values in a more ergonomic way.

### Added

- Now every node has a `bounds` property representing its axis-aligned bounding box in local coordinates.
- Catch uncaptured wgpu errors and show a fatal error message before exiting.
- Add `<video>` node for video playback, supporting common video formats on all platforms via the `moyu-video` crate.
- Add sandboxed eval and incremental runner for scenario script, allowing dynamic code execution.
- Add custom parameters to MoyuConfig for engine access

### Fixed

- Potential deadlock on hit testing.
- Enhance rect calculation for `Clip` and `Backdrop` nodes.
- `Filter` node uses bounds size to allocate offscreen texture instead of full canvas size.
- Fix rendering crash when bounds of `Backdrop` node out of canvas.
- Wrong instance count in RenderCommand for instanced drawing.
- Crash when scenario parsing failed.
- Increase the maximum stack size of QuickJS runtime to avoid stack overflow in complex scenarios.
- Change SetWaiting time type to f64 for better JS compatibility

### Optimized

- Add RenderState for managing rendering state
- Optimize RenderCommand collection by culling invisible nodes based on their global bounds
- Use Arc::ptr equality for Node comparison to avoid locking and improve performance
- Introduce `.into_node_lock()` helper method for easier conversion to `Arc<RwLock<Box<dyn Node>>>`

## [0.9.0] - 2026-01-13

### Breaking

- Rename `name` to `key` in SetPermanentVariable and GetPermanentVariable (#37)
- Rename `surface_size` to `initial_surface_size` in `index.json` for clarity
- Implement correct canvas size handling on web platform, considering device pixel ratio. (#38)
- Rename `layer_x`/`layer_y` to `offset_x`/`offset_y` for mouse and touch events
- Upgrade `winit` to 0.30.
- Upgrade `react` and related packages to 19.x.
- Remove `path` field from AddStory command in Scenario plugin.
- Make `NextLine` command in Scenario plugin async command.
- Update `sixu` to 0.8.0 which introduces breaking changes in parsed command format.
- Refactor all rendering pipelines to use the new RenderQueue architecture.

### Added

- Support opus audio format on all platforms, including wasm32.
- Allow adding extra data to save files in addition to metadata.
- Add `assets:`, `appdata:`, `saves:` and `data:` URL schemes for file access.
- Add fade time option for audio playback and transitions
- Add `set_timeout` and `clear_timeout` functions to schedule async tasks in QuickJS & browser environment.
- Add waiting state management in Scenario plugin with `SetWaiting` command.
- Implement `GetCursorPosition` command of Text node to get the current cursor position.
- Enhance text node to support printing text following existing content (nvl mode).
- Support passing configuration from JavaScript when initializing Moyu on web platform.
- Support dynamically loading scenario script files in scenario script.
- Add `ts-rs` across multiple crates and update package.json for bindings generation
- Add builtin WebSocket, `fetch`, dom (partly), etc. support in QuickJS environment on native platforms.
- Support React hot reloading in development mode on native platforms.
- Adds filters and backdrop-filters support via new RenderQueue architecture.

### Changed

- Avoid using `static mut` for global variables to avoid undefined behavior.
- Update edition to 2024 on all crates.
- Upgrade `wgpu` to 27.x.
- Upgrade `quickjs-rusty` to 0.10 which fixes a number conversion issue.
- Fix crashes on Web when taking snapshots due to double mapping of snapshot buffer.
- Fix wrong conversion json object to Map in `create_promise` on Web.
- Fix error logs when releasing audio resources that not played.
- Add global `window` object in QuickJS environment corresponding to browser environment.
- Enhance console logger, `[unrepresentable value]` is shown for unrepresentable values instead of panicking.
- Avoid compiling quickjs runtime on wasm32 target to remove warnings from `rust-analyzer`.
- Update `sixu` to 0.3.0 for better flow control.
- Update `huozi` to 0.14.1 and use `<>` as style tag in text layout parsing.
- Correctly set node id to 0 instead of 1 for the root node.
- Improve hit testing to always hit the root node at least.
- Support specifying entry point when starting a story in Scenario plugin.
- Fix `COPY_BYTES_PER_ROW_ALIGNMENT` problem when taking snapshots.
- Enhance size setting logic in SpriteRenderer.
- Fixes wrong size setting in sprite node when `area` is set.
- Fixes an issue causing slplash screen stuck on web platform.
- Fixed graphic size issues on initialization on web platform.
- Refactor DPI handling logic on web platform.
- Update scale factor in surface size storage when scale factor changed
- Make `.send_event()` method of `PluginEventSource` and `NodeEventSource` use async dispatch to avoid potential deadlocks.

### Performance

- Split the submission of staging belt encoders to improve performance.

## [0.8.0] - 2025-09-28

New milestone release. As a new start, we are adopting a more structured changelog format.

### Added

### Changed

### Fixed

### Breaking

### Removed

### Deprecated

### Performance

### Tests

### Security

### Documentation

### Miscellaneous
