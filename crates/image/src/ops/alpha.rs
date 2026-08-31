use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{MulDiv, PixelType};

pub(crate) fn premultiply_alpha(data: &mut [u8], width: u32, height: u32, stride: u32) {
    let row_bytes = width as usize * 4;
    let rows = &mut data[..stride as usize * height as usize];
    let mul_div = MulDiv::new();

    for row in rows.chunks_exact_mut(stride as usize) {
        let mut image = Image::from_slice_u8(width, 1, &mut row[..row_bytes], PixelType::U8x4)
            .expect("validated RGBA8 row");
        mul_div
            .multiply_alpha_inplace(&mut image)
            .expect("U8x4 alpha multiplication is supported");
    }
}

pub(crate) fn copy_premultiplied_rgba8(source: &[u8], output: &mut Vec<u8>) {
    output.resize(source.len(), 0);
    let width = u32::try_from(source.len() / 4).expect("RGBA8 buffer width fits in u32");
    let source = ImageRef::new(width, 1, source, PixelType::U8x4).expect("valid RGBA8 source");
    let mut destination =
        Image::from_slice_u8(width, 1, output, PixelType::U8x4).expect("valid RGBA8 output");

    MulDiv::new()
        .multiply_alpha(&source, &mut destination)
        .expect("U8x4 alpha multiplication is supported");
}
