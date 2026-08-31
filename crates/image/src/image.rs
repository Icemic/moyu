use std::borrow::Cow;

use crate::error::{ImageError, Result};
use crate::ops;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8Image {
    width: u32,
    height: u32,
    stride: u32,
    data: Vec<u8>,
}

impl Rgba8Image {
    pub fn from_rgba8(width: u32, height: u32, stride: u32, data: Vec<u8>) -> Result<Self> {
        Self::validate_layout(width, height, stride, data.len())?;

        Ok(Self {
            width,
            height,
            stride,
            data,
        })
    }

    pub fn from_bgra8(width: u32, height: u32, stride: u32, mut data: Vec<u8>) -> Result<Self> {
        Self::validate_layout(width, height, stride, data.len())?;

        let row_bytes = Self::row_bytes(width)?;
        for row in data.chunks_exact_mut(stride as usize).take(height as usize) {
            for pixel in row[..row_bytes].chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }

        Ok(Self {
            width,
            height,
            stride,
            data,
        })
    }

    fn validate_layout(width: u32, height: u32, stride: u32, data_len: usize) -> Result<()> {
        let row_bytes = Self::row_bytes(width)?;
        let stride = stride as usize;
        let required_len = stride
            .checked_mul(height as usize)
            .ok_or_else(ImageError::invalid_layout)?;

        if stride < row_bytes || data_len < required_len {
            return Err(ImageError::invalid_layout());
        }

        Ok(())
    }

    fn row_bytes(width: u32) -> Result<usize> {
        (width as usize)
            .checked_mul(4)
            .ok_or_else(ImageError::invalid_layout)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stride(&self) -> u32 {
        self.stride
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn to_compact(&self) -> Cow<'_, [u8]> {
        let row_bytes = Self::row_bytes(self.width).expect("validated image width");
        let compact_len = row_bytes
            .checked_mul(self.height as usize)
            .expect("validated image layout");
        if self.stride as usize == row_bytes {
            return Cow::Borrowed(&self.data[..compact_len]);
        }

        let mut data = Vec::with_capacity(compact_len);
        for row in self
            .data
            .chunks_exact(self.stride as usize)
            .take(self.height as usize)
        {
            data.extend_from_slice(&row[..row_bytes]);
        }
        Cow::Owned(data)
    }

    pub fn premultiply_alpha_in_place(&mut self) {
        ops::premultiply_alpha(&mut self.data, self.width, self.height, self.stride);
    }

    pub fn resize(&self, width: u32, height: u32) -> Result<Self> {
        ops::resize(self, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::Rgba8Image;

    #[test]
    fn compact_data_skips_stride_padding() {
        let image = Rgba8Image::from_rgba8(
            1,
            2,
            8,
            vec![1, 2, 3, 4, 9, 9, 9, 9, 5, 6, 7, 8, 9, 9, 9, 9],
        )
        .unwrap();

        assert_eq!(image.to_compact().as_ref(), [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn compact_data_skips_trailing_bytes() {
        let image = Rgba8Image::from_rgba8(1, 1, 4, vec![1, 2, 3, 4, 9, 9]).unwrap();

        assert_eq!(image.to_compact().as_ref(), [1, 2, 3, 4]);
    }

    #[test]
    fn bgra_input_preserves_padding_and_alpha() {
        let image = Rgba8Image::from_bgra8(1, 1, 8, vec![3, 2, 1, 4, 9, 9, 9, 9]).unwrap();

        assert_eq!(image.data(), [1, 2, 3, 4, 9, 9, 9, 9]);
    }

    #[test]
    fn premultiply_updates_only_pixel_channels() {
        let mut image = Rgba8Image::from_rgba8(1, 1, 4, vec![200, 100, 50, 128]).unwrap();

        image.premultiply_alpha_in_place();

        assert_eq!(image.data(), [100, 50, 25, 128]);
    }

    #[test]
    fn premultiply_handles_large_strided_images_without_touching_padding() {
        let width = 100;
        let height = 100;
        let stride = 404;
        let mut data = vec![9; stride * height];
        for row in data.chunks_exact_mut(stride) {
            for pixel in row[..width * 4].chunks_exact_mut(4) {
                pixel.copy_from_slice(&[200, 100, 50, 128]);
            }
        }
        let mut image =
            Rgba8Image::from_rgba8(width as u32, height as u32, stride as u32, data).unwrap();

        image.premultiply_alpha_in_place();

        for row in image.data().chunks_exact(stride) {
            assert!(
                row[..width * 4]
                    .chunks_exact(4)
                    .all(|pixel| pixel == [100, 50, 25, 128])
            );
            assert_eq!(&row[width * 4..], [9, 9, 9, 9]);
        }
    }

    #[test]
    fn resize_returns_compact_rgba8() {
        let image =
            Rgba8Image::from_rgba8(2, 1, 8, vec![10, 20, 30, 255, 40, 50, 60, 255]).unwrap();

        let resized = image.resize(4, 3).unwrap();

        assert_eq!(
            (resized.width(), resized.height(), resized.stride()),
            (4, 3, 16)
        );
        assert_eq!(resized.data().len(), 4 * 3 * 4);
        assert!(resized.data().chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn rejects_invalid_layout() {
        assert!(Rgba8Image::from_rgba8(2, 1, 4, vec![0; 8]).is_err());
        assert!(Rgba8Image::from_rgba8(1, 2, 4, vec![0; 4]).is_err());
    }
}
