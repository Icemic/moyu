//! Error codes returned by the plugin entry points.

/// Success.
pub const OK: i32 = 0;

/// Invalid argument, such as a null handle or an out-of-range index.
pub const INVALID_ARGUMENT: i32 = 1;

/// Host and plugin do not share the same major contract version.
pub const VERSION_MISMATCH: i32 = 2;

/// The plugin could not be used: it is missing, or a dependency it loads at
/// runtime is missing or too old.
pub const CORE_UNAVAILABLE: i32 = 3;

/// Asset could not be read, such as a missing model file.
pub const FILE: i32 = 4;

/// Asset or SDK data could not be parsed.
pub const PARSE: i32 = 5;

/// Operation is outside the supported feature set.
pub const UNSUPPORTED: i32 = 6;

/// Internal plugin failure.
pub const INTERNAL: i32 = 7;
