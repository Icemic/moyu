//! Contract version and capability negotiation.

/// Major version of the contract; host and plugin must share it.
///
/// 2.0 replaced the raw SDK blend mode with a decided value, turned the
/// parameter type into a contract enum, and dropped `MaskCommand::clear_before`.
/// The changes live in shared structs, so a 1.x plugin cannot be paired with a
/// 2.x host and the version gate is what keeps them apart.
pub const ABI_VERSION_MAJOR: u16 = 2;

/// Minor version of the contract.
pub const ABI_VERSION_MINOR: u16 = 0;

/// Encoded contract version, `major << 16 | minor`.
pub const ABI_VERSION: u32 = (ABI_VERSION_MAJOR as u32) << 16 | ABI_VERSION_MINOR as u32;

/// Reserved capability bits reported by the plugin at load time.
///
/// No bit is defined in this version; the constant is reserved for optional
/// feature detection in later versions.
pub const CAPS: u64 = 0;

/// Whether a plugin built with `version` shares the major version of this contract.
pub const fn is_compatible(version: u32) -> bool {
    version >> 16 == ABI_VERSION >> 16
}
