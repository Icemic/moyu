pub fn premultiply_alpha(rgba: &mut [u8]) {
    // 32000 is a rough threshold value.
    if rgba.len() > 32000 {
        premultiply_alpha_mthread(rgba);
    } else {
        premultiply_alpha_auto_simd(rgba);
    }
}

pub fn premultiply_alpha_auto_simd(rgba: &mut [u8]) {
    rgba.chunks_exact_mut(4).for_each(|p| {
        let alpha = p[3] as f32 / 255.0;
        p[0] = (p[0] as f32 * alpha) as u8;
        p[1] = (p[1] as f32 * alpha) as u8;
        p[2] = (p[2] as f32 * alpha) as u8;
    });
}

pub fn premultiply_alpha_mthread(rgba: &mut [u8]) {
    use rayon::iter::ParallelIterator;
    use rayon::prelude::ParallelSliceMut;
    rgba.par_chunks_exact_mut(4).for_each(|p| {
        let alpha = p[3] as f32 / 255.0;
        p[0] = (p[0] as f32 * alpha) as u8;
        p[1] = (p[1] as f32 * alpha) as u8;
        p[2] = (p[2] as f32 * alpha) as u8;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // in fact it's not a test, just a performance test
    // due to lack of #[bench], I use #[test] instead
    #[test]
    fn test_premultiply_alpha() {
        let pairs = [
            (3840, 2160),
            (2560, 1440),
            (1920, 1080),
            (1280, 720),
            (1024, 768),
            (800, 600),
            (640, 480),
            (320, 240),
            (240, 160),
            (200, 150),
            (160, 120),
            (80, 60),
            (20, 15),
            (10, 7),
            (3, 2),
            (1, 1),
        ];

        for (w, h) in pairs.iter() {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let mut rgba = vec![100u8; w * h * 4];
            let time = std::time::Instant::now();
            premultiply_alpha(&mut rgba);
            println!("{}x{} ({}): {:?}", w, h, w * h, time.elapsed());
        }
    }
}
