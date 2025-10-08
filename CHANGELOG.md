# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Support opus audio format on all platforms, including wasm32.

### Changed

- Avoid using `static mut` for global variables to avoid undefined behavior.
- Update edition to 2024 on all crates.
- Upgrade `wgpu` to 27.x.
- Upgrade `quickjs-rusty` to 0.10 which fixes a number conversion issue.

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
