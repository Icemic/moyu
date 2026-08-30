use rayon::prelude::*;

const PARALLEL_PREMULTIPLY_THRESHOLD: usize = 32_000;

pub(crate) fn premultiply_alpha(data: &mut [u8], width: u32, height: u32, stride: u32) {
    let row_bytes = width as usize * 4;
    let rows = &mut data[..stride as usize * height as usize];

    if row_bytes * height as usize > PARALLEL_PREMULTIPLY_THRESHOLD {
        rows.par_chunks_exact_mut(stride as usize)
            .for_each(|row| premultiply_row(&mut row[..row_bytes]));
    } else {
        rows.chunks_exact_mut(stride as usize)
            .for_each(|row| premultiply_row(&mut row[..row_bytes]));
    }
}

#[inline(always)]
fn premultiply_row(row: &mut [u8]) {
    for pixel in row.chunks_exact_mut(4) {
        let alpha = pixel[3] as u16;
        pixel[0] = (pixel[0] as u16 * alpha / 255) as u8;
        pixel[1] = (pixel[1] as u16 * alpha / 255) as u8;
        pixel[2] = (pixel[2] as u16 * alpha / 255) as u8;
    }
}

pub(crate) fn copy_premultiplied_rgba8(source: &[u8], output: &mut Vec<u8>) {
    output.resize(source.len(), 0);

    if source.len() > PARALLEL_PREMULTIPLY_THRESHOLD {
        output
            .par_chunks_exact_mut(4)
            .zip(source.par_chunks_exact(4))
            .for_each(|(destination, source)| premultiply_pixel(source, destination));
    } else {
        output
            .chunks_exact_mut(4)
            .zip(source.chunks_exact(4))
            .for_each(|(destination, source)| premultiply_pixel(source, destination));
    }
}

#[inline(always)]
fn premultiply_pixel(source: &[u8], destination: &mut [u8]) {
    let alpha = source[3] as u16;
    destination[0] = (source[0] as u16 * alpha / 255) as u8;
    destination[1] = (source[1] as u16 * alpha / 255) as u8;
    destination[2] = (source[2] as u16 * alpha / 255) as u8;
    destination[3] = source[3];
}
