# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Support opus audio format on all platforms, including wasm32.
- Allow adding extra data to save files in addition to metadata.
- Add `assets:`, `appdata:`, `saves:` and `data:` URL schemes for file access.
- Add fade time option for audio playback and transitions
- Add `set_timeout` and `clear_timeout` functions to schedule async tasks in QuickJS & browser environment.
- Add waiting state management in Scenario plugin with `SetWaiting` command.

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
