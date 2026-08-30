use serde::{Deserialize, Serialize};

/// Represents the format of the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SnapshotFormat {
    Rgba8,
    Bgra8,
    Rgba16f,
}

impl std::fmt::Display for SnapshotFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotFormat::Rgba8 => write!(f, "RGBA8"),
            SnapshotFormat::Bgra8 => write!(f, "BGRA8"),
            SnapshotFormat::Rgba16f => write!(f, "RGBA16F"),
        }
    }
}

impl TryFrom<wgpu::TextureFormat> for SnapshotFormat {
    type Error = wgpu::TextureFormat;

    fn try_from(format: wgpu::TextureFormat) -> Result<Self, Self::Error> {
        match format {
            wgpu::TextureFormat::Rgba8Unorm => Ok(SnapshotFormat::Rgba8),
            wgpu::TextureFormat::Bgra8Unorm => Ok(SnapshotFormat::Bgra8),
            wgpu::TextureFormat::Rgba16Float => Ok(SnapshotFormat::Rgba16f),
            _ => Err(format),
        }
    }
}

/// Represents a snapshot of the window's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// The width of the snapshot in pixels.
    pub width: u32,
    /// The height of the snapshot in pixels.
    pub height: u32,
    /// The raw pixel data of the snapshot.
    /// Note: This data may contain padding bytes for memory alignment.
    /// Use `stride` to determine the actual bytes per row in the buffer.
    pub data: Vec<u8>,
    /// The stride (bytes per row) in the data buffer.
    /// This may be larger than `width * bytes_per_pixel()` due to alignment requirements.
    pub stride: u32,
    /// The texture format of the snapshot data.
    pub format: SnapshotFormat,
}

impl Snapshot {
    pub fn bytes_per_pixel(&self) -> u32 {
        match self.format {
            SnapshotFormat::Rgba8 => 4,
            SnapshotFormat::Bgra8 => 4,
            SnapshotFormat::Rgba16f => 8,
        }
    }

    /// Convert raw snapshot data with potential padding and BGRA format to RGBA8.
    fn to_rgba8(&self) -> std::io::Result<moyu_image::Rgba8Image> {
        let image = match self.format {
            SnapshotFormat::Rgba8 => moyu_image::Rgba8Image::from_rgba8(
                self.width,
                self.height,
                self.stride,
                self.data.clone(),
            ),
            SnapshotFormat::Bgra8 => moyu_image::Rgba8Image::from_bgra8(
                self.width,
                self.height,
                self.stride,
                self.data.clone(),
            ),
            SnapshotFormat::Rgba16f => unreachable!("RGBA16F is rejected before conversion"),
        };

        image.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    pub fn save_to_buffer(&self) -> std::io::Result<Vec<u8>> {
        // RGBA16F format is not supported for saving as image
        if matches!(self.format, SnapshotFormat::Rgba16f) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "RGBA16F format is not supported for saving as image",
            ));
        }

        if self.data.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Snapshot data is empty",
            ));
        }

        let image = self.to_rgba8()?;
        moyu_image::encode_webp(&image)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str());
        if !extension.is_some_and(|extension| extension.eq_ignore_ascii_case("webp")) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Snapshot files must use the .webp extension",
            ));
        }

        let buffer = self.save_to_buffer()?;

        // Write to file
        std::fs::write(path, buffer)?;

        log::info!("Snapshot saved to {:?} (format: {})", path, self.format);
        Ok(())
    }

    /// Resize the snapshot, modifying itself
    ///
    /// # Arguments
    /// - `width`: New width
    /// - `height`: New height  
    /// - `keep_aspect`: Whether to maintain aspect ratio. If true, will scale proportionally to fit within the specified dimensions, one side may be smaller than specified
    pub fn resize(&mut self, width: u32, height: u32, keep_aspect: bool) -> std::io::Result<()> {
        // RGBA16F is outside moyu_image's RGBA8 processing model.
        if matches!(self.format, SnapshotFormat::Rgba16f) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "RGBA16F format is not supported for resize operation",
            ));
        }

        if self.data.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Snapshot data is empty",
            ));
        }

        // If dimensions are the same, no operation needed
        if self.width == width && self.height == height {
            return Ok(());
        }

        // Calculate actual target dimensions
        let (target_width, target_height) = if keep_aspect {
            let aspect_ratio = self.width as f64 / self.height as f64;
            let target_aspect = width as f64 / height as f64;

            if aspect_ratio > target_aspect {
                // Original image is wider, use width as reference
                (width, (width as f64 / aspect_ratio) as u32)
            } else {
                // Original image is taller, use height as reference
                ((height as f64 * aspect_ratio) as u32, height)
            }
        } else {
            (width, height)
        };

        let image = self.to_rgba8()?;
        let image = image
            .resize(target_width, target_height)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
        let data = image.into_data();

        self.width = target_width;
        self.height = target_height;
        self.data = data;
        // After resize, there's no padding
        self.stride = target_width * self.bytes_per_pixel();
        // After resize, format is RGBA8
        self.format = SnapshotFormat::Rgba8;

        Ok(())
    }
}
