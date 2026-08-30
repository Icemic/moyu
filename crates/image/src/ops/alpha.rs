pub(crate) fn premultiply_alpha(data: &mut [u8], width: u32, height: u32, stride: u32) {
    let row_bytes = width as usize * 4;
    for row in data.chunks_exact_mut(stride as usize).take(height as usize) {
        for pixel in row[..row_bytes].chunks_exact_mut(4) {
            let alpha = pixel[3] as u16;
            pixel[0] = (pixel[0] as u16 * alpha / 255) as u8;
            pixel[1] = (pixel[1] as u16 * alpha / 255) as u8;
            pixel[2] = (pixel[2] as u16 * alpha / 255) as u8;
        }
    }
}

pub(crate) fn copy_premultiplied_rgba8(source: &[u8], output: &mut Vec<u8>) {
    output.clear();
    output.reserve(source.len());

    for pixel in source.chunks_exact(4) {
        let alpha = pixel[3] as u16;
        output.extend_from_slice(&[
            (pixel[0] as u16 * alpha / 255) as u8,
            (pixel[1] as u16 * alpha / 255) as u8,
            (pixel[2] as u16 * alpha / 255) as u8,
            pixel[3],
        ]);
    }
}
