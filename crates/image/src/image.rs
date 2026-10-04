use std::borrow::Cow;

use fast_image_resize::pixels::U8x4;
use fast_image_resize::{ImageView, ImageViewMut};

use crate::alpha_mask::AlphaMask;
use crate::error::{ImageError, Result};
use crate::ops;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8Image {
    width: u32,
    height: u32,
    stride: u32,
    data: Vec<u8>,
}

unsafe impl ImageView for Rgba8Image {
    type Pixel = U8x4;

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn iter_rows(&self, start_row: u32) -> impl Iterator<Item = &[Self::Pixel]> {
        let row_bytes = self.width as usize * 4;
        let stride = self.stride as usize;
        self.data[start_row as usize * stride..stride * self.height as usize]
            .chunks_exact(stride)
            .map(move |row| unsafe { row[..row_bytes].align_to::<U8x4>().1 })
    }
}

unsafe impl ImageViewMut for Rgba8Image {
    fn iter_rows_mut(&mut self, start_row: u32) -> impl Iterator<Item = &mut [Self::Pixel]> {
        let row_bytes = self.width as usize * 4;
        let stride = self.stride as usize;
        self.data[start_row as usize * stride..stride * self.height as usize]
            .chunks_exact_mut(stride)
            .map(move |row| unsafe { row[..row_bytes].align_to_mut::<U8x4>().1 })
    }
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
        fast_image_resize::premultiply_alpha_inplace_u8x4(self);
    }

    /// Build the alpha mask for pixel-level hit testing.
    ///
    /// Transparent texels are accumulated a byte at a time, so each write covers
    /// eight texels and nothing is written for the opaque stretches in between.
    /// The bitmap is allocated lazily, so an image whose texels are all
    /// non-transparent only returns [`AlphaMask::Opaque`].
    pub fn extract_alpha_mask(&self) -> AlphaMask {
        let row_bytes = self.width as usize * 4;
        let stride = self.stride as usize;
        let byte_count = (self.width as usize * self.height as usize).div_ceil(8);
        let mut transparent: Option<Vec<u8>> = None;

        // Texels are numbered row-major, so the accumulator carries across row
        // boundaries and only the trailing partial byte needs a separate flush.
        let mut byte = 0u8;
        let mut bit = 0u8;
        let mut byte_index = 0usize;

        for row in 0..self.height as usize {
            let start = row * stride;
            for pixel in self.data[start..start + row_bytes].chunks_exact(4) {
                byte |= u8::from(pixel[3] == 0) << bit;
                bit += 1;

                if bit == 8 {
                    if byte != 0 {
                        transparent
                            .get_or_insert_with(|| vec![0u8; byte_count])[byte_index] |= byte;
                    }
                    byte = 0;
                    bit = 0;
                    byte_index += 1;
                }
            }
        }

        if bit != 0 && byte != 0 {
            transparent
                .get_or_insert_with(|| vec![0u8; byte_count])[byte_index] |= byte;
        }

        match transparent {
            Some(transparent) => AlphaMask::Bitmap {
                width: self.width,
                height: self.height,
                transparent,
            },
            None => AlphaMask::Opaque,
        }
    }

    pub fn resize(&self, width: u32, height: u32) -> Result<Self> {
        ops::resize(self, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::Rgba8Image;
    use crate::AlphaMask;

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

    #[test]
    fn alpha_mask_marks_transparent_texels() {
        // Row padding is filled with zeros as well; the mask must only look at pixels.
        let image = Rgba8Image::from_rgba8(
            1,
            3,
            8,
            vec![
                1, 2, 3, 255, 0, 0, 0, 0, //
                4, 5, 6, 1, 0, 0, 0, 0, //
                7, 8, 9, 0, 0, 0, 0, 0,
            ],
        )
        .unwrap();

        let mask = image.extract_alpha_mask();

        assert!(mask.is_opaque(0, 0));
        assert!(mask.is_opaque(0, 1));
        assert!(!mask.is_opaque(0, 2));
        assert!(!mask.is_opaque(1, 0));
    }

    #[test]
    fn alpha_mask_stays_opaque_without_transparent_texels() {
        let image = Rgba8Image::from_rgba8(2, 1, 8, vec![1, 2, 3, 255, 4, 5, 6, 1]).unwrap();

        assert_eq!(image.extract_alpha_mask(), AlphaMask::Opaque);
    }

    #[test]
    fn alpha_mask_packs_texels_across_byte_boundaries() {
        let mut data = vec![1u8; 16 * 4];
        for x in [0usize, 8, 9, 15] {
            data[x * 4 + 3] = 0;
        }
        let image = Rgba8Image::from_rgba8(16, 1, 64, data).unwrap();

        let mask = image.extract_alpha_mask();

        assert!(!mask.is_opaque(0, 0));
        assert!(mask.is_opaque(1, 0));
        assert!(mask.is_opaque(7, 0));
        assert!(!mask.is_opaque(8, 0));
        assert!(!mask.is_opaque(9, 0));
        assert!(mask.is_opaque(10, 0));
        assert!(mask.is_opaque(14, 0));
        assert!(!mask.is_opaque(15, 0));
    }
}
