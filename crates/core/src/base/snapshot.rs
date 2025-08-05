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

impl From<wgpu::TextureFormat> for SnapshotFormat {
    fn from(format: wgpu::TextureFormat) -> Self {
        match format {
            wgpu::TextureFormat::Rgba8Unorm => SnapshotFormat::Rgba8,
            wgpu::TextureFormat::Bgra8Unorm => SnapshotFormat::Bgra8,
            wgpu::TextureFormat::Rgba16Float => SnapshotFormat::Rgba16f,
            _ => panic!("Unsupported texture format for snapshot"),
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
    pub data: Vec<u8>,
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

    pub fn save_to_buffer(&self, format: image::ImageFormat) -> std::io::Result<Vec<u8>> {
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

        // Create DynamicImage based on the format
        let image = match self.format {
            SnapshotFormat::Rgba8 => {
                // RGBA8 can be used directly
                match image::RgbaImage::from_raw(self.width, self.height, self.data.clone()) {
                    Some(img) => image::DynamicImage::ImageRgba8(img),
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Failed to create image from raw RGBA8 data",
                        ));
                    }
                }
            }
            SnapshotFormat::Bgra8 => {
                // BGRA8 needs to be converted to RGBA using image crate's pixel operations
                match image::RgbaImage::from_raw(self.width, self.height, self.data.clone()) {
                    Some(mut img) => {
                        // Use image crate's pixel iterator for BGRA -> RGBA conversion
                        for pixel in img.pixels_mut() {
                            let [b, g, r, a] = pixel.0;
                            pixel.0 = [r, g, b, a]; // BGRA -> RGBA
                        }
                        image::DynamicImage::ImageRgba8(img)
                    }
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Failed to create image from raw BGRA8 data",
                        ));
                    }
                }
            }
            SnapshotFormat::Rgba16f => {
                unreachable!("RGBA16F should be handled above");
            }
        };

        // Encode as byte array in the specified format
        let mut buffer = std::io::Cursor::new(Vec::new());
        if let Err(e) = image.write_to(&mut buffer, format) {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }

        Ok(buffer.into_inner())
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        // Infer format from file extension
        let format = if let Some(extension) = std::path::Path::new(path).extension() {
            match extension.to_str().unwrap_or("").to_lowercase().as_str() {
                "png" => image::ImageFormat::Png,
                "jpg" | "jpeg" => image::ImageFormat::Jpeg,
                "bmp" => image::ImageFormat::Bmp,
                "tiff" | "tif" => image::ImageFormat::Tiff,
                "webp" => image::ImageFormat::WebP,
                _ => image::ImageFormat::Png, // Default to PNG
            }
        } else {
            image::ImageFormat::Png // Default to PNG
        };

        // Use save_to_buffer to get encoded data
        let buffer = self.save_to_buffer(format)?;

        // Write to file
        std::fs::write(path, buffer)?;

        log::info!(
            "Snapshot saved to {:?} (format: {}, image format: {:?})",
            path,
            self.format,
            format
        );
        Ok(())
    }

    /// Resize the snapshot, modifying itself
    ///
    /// # Arguments
    /// - `width`: New width
    /// - `height`: New height  
    /// - `keep_aspect`: Whether to maintain aspect ratio. If true, will scale proportionally to fit within the specified dimensions, one side may be smaller than specified
    pub fn resize(&mut self, width: u32, height: u32, keep_aspect: bool) -> std::io::Result<()> {
        // RGBA16F format is not supported for resize operation because image crate doesn't support it directly
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

        // Create original image
        let image = match image::RgbaImage::from_raw(self.width, self.height, self.data.clone()) {
            Some(img) => image::DynamicImage::ImageRgba8(img),
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to create image from raw data",
                ));
            }
        };

        // Perform scaling
        let image = image.resize(
            target_width,
            target_height,
            image::imageops::FilterType::Lanczos3,
        );
        let image = image.to_rgba8();

        // Update data
        let data = image.into_raw();

        self.width = target_width;
        self.height = target_height;
        self.data = data;

        Ok(())
    }
}
