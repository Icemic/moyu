use std::error::Error;

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ImageError {
    message: String,
    #[source]
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl ImageError {
    pub(crate) fn invalid_layout() -> Self {
        Self::new("invalid RGBA8 image layout")
    }

    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub(crate) fn with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, ImageError>;
