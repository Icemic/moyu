use std::time::Duration;

pub(crate) fn frame_duration(numerator: u16, denominator: u16) -> Duration {
    let denominator = u64::from(if denominator == 0 { 100 } else { denominator });
    Duration::from_nanos(u64::from(numerator) * 1_000_000_000 / denominator)
}

pub(crate) fn rgb_to_rgba(source: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for pixel in source
        .chunks_exact(3)
        .take(width as usize * height as usize)
    {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }
    rgba
}

pub(crate) fn gray_to_rgba(source: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for &value in source.iter().take(width as usize * height as usize) {
        rgba.extend_from_slice(&[value, value, value, 255]);
    }
    rgba
}

pub(crate) fn gray_alpha_to_rgba(source: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for pixel in source
        .chunks_exact(2)
        .take(width as usize * height as usize)
    {
        rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
    }
    rgba
}

pub(crate) fn blend_over(destination: &mut [u8], source: &[u8]) {
    let source_alpha = source[3] as f32 / 255.0;
    let destination_alpha = destination[3] as f32 / 255.0;
    let alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    if alpha == 0.0 {
        destination.fill(0);
        return;
    }

    destination[0] = ((source[0] as f32 * source_alpha
        + destination[0] as f32 * destination_alpha * (1.0 - source_alpha))
        / alpha) as u8;
    destination[1] = ((source[1] as f32 * source_alpha
        + destination[1] as f32 * destination_alpha * (1.0 - source_alpha))
        / alpha) as u8;
    destination[2] = ((source[2] as f32 * source_alpha
        + destination[2] as f32 * destination_alpha * (1.0 - source_alpha))
        / alpha) as u8;
    destination[3] = (alpha * 255.0) as u8;
}
