use fast_image_resize::images::{TypedImage, TypedImageRef};
use fast_image_resize::pixels::U8x4;

pub(crate) fn copy_premultiplied_rgba8(source: &[u8], output: &mut Vec<u8>) {
    output.resize(source.len(), 0);
    let width = u32::try_from(source.len() / 4).expect("RGBA8 buffer width fits in u32");
    let source = TypedImageRef::<U8x4>::from_buffer(width, 1, source).expect("valid RGBA8 source");
    let mut destination =
        TypedImage::<U8x4>::from_buffer(width, 1, output).expect("valid RGBA8 output");

    fast_image_resize::premultiply_alpha_u8x4(&source, &mut destination);
}
