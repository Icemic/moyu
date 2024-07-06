#[cfg(not(feature = "web"))]
pub trait ThreadFeature: Send + Sync {}

#[cfg(feature = "web")]
pub trait ThreadFeature {}
