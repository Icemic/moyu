/// Alpha channel of a decoded image, used for pixel-level hit testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlphaMask {
    /// Every texel counts as a hit. Kept as a sentinel so fully opaque images do
    /// not pay for a bitmap.
    Opaque,
    /// Bitmap of fully transparent texels.
    Bitmap {
        width: u32,
        height: u32,
        /// Row-major and least significant bit first; a set bit marks a texel that
        /// must not receive hits.
        transparent: Vec<u8>,
    },
}

impl AlphaMask {
    /// Whether the texel at `(x, y)` counts as a hit.
    ///
    /// `Opaque` ignores the coordinates. For `Bitmap`, coordinates outside the
    /// image do not count as a hit.
    pub fn is_opaque(&self, x: u32, y: u32) -> bool {
        match self {
            AlphaMask::Opaque => true,
            AlphaMask::Bitmap {
                width,
                height,
                transparent,
            } => {
                if x >= *width || y >= *height {
                    return false;
                }

                let index = y as usize * *width as usize + x as usize;
                transparent[index / 8] & (1 << (index % 8)) == 0
            }
        }
    }
}
