mod animation;
mod codecs;
mod error;
mod image;
mod ops;
mod utils;

pub use animation::{AnimationDecoder, AnimationFormat};
pub use error::ImageError;
pub use image::Rgba8Image;

pub fn decode(data: &[u8]) -> Result<Rgba8Image, ImageError> {
    codecs::decode(data)
}

pub fn encode_webp(image: &Rgba8Image) -> Result<Vec<u8>, ImageError> {
    codecs::encode_webp(image)
}

#[cfg(test)]
mod tests {
    use super::{AnimationDecoder, AnimationFormat, Rgba8Image, decode, encode_webp};

    const PNG: &[u8] = include_bytes!("../fixtures/dice.png");
    const WEBP: &[u8] = include_bytes!("../fixtures/dice.webp");
    const JPEG: &[u8] = include_bytes!("../fixtures/zoltan-tasi-CLJeQCr2F_A-unsplash.jpg");
    const JXL: &[u8] = include_bytes!("../fixtures/dice.jxl");
    const APNG: &[u8] = include_bytes!("../fixtures/anim-icos.apng.png");
    const ANIMATED_WEBP: &[u8] = include_bytes!("../fixtures/anim-icos.webp");
    const ANIMATED_JXL: &[u8] = include_bytes!("../fixtures/anim-icos.jxl");

    #[test]
    fn decodes_supported_static_formats_as_rgba8() {
        for data in [PNG, WEBP, JPEG, JXL] {
            let image = decode(data).unwrap();
            assert!(image.width() > 0);
            assert!(image.height() > 0);
            assert_eq!(
                image.data().len(),
                image.width() as usize * image.height() as usize * 4
            );
        }
    }

    #[test]
    fn rejects_unknown_static_format() {
        assert!(decode(b"not an image").is_err());
    }

    #[test]
    fn webp_encoding_round_trips_rgba8_data() {
        let image = Rgba8Image::from_rgba8(2, 1, 8, vec![1, 2, 3, 255, 40, 50, 60, 128]).unwrap();

        let encoded = encode_webp(&image).unwrap();
        let decoded = decode(&encoded).unwrap();

        assert_eq!(decoded.width(), image.width());
        assert_eq!(decoded.height(), image.height());
        assert_eq!(decoded.data(), image.data());
    }

    #[test]
    fn apng_restarts_with_the_same_first_frame() {
        assert_animation_restarts(APNG, AnimationFormat::Apng);
    }

    #[test]
    fn animated_webp_restarts_with_the_same_first_frame() {
        assert_animation_restarts(ANIMATED_WEBP, AnimationFormat::WebP);
    }

    #[test]
    fn animated_jxl_restarts_with_the_same_first_frame() {
        assert_animation_restarts(ANIMATED_JXL, AnimationFormat::Jxl);
    }

    fn assert_animation_restarts(data: &[u8], format: AnimationFormat) {
        let mut decoder = AnimationDecoder::new(data.to_vec(), format).unwrap();
        assert!(decoder.advance().unwrap().is_some());

        let mut first = Vec::new();
        decoder.write_premultiplied_frame(&mut first).unwrap();
        assert_eq!(
            first.len(),
            decoder.width() as usize * decoder.height() as usize * 4
        );
        assert!(
            first
                .chunks_exact(4)
                .all(|pixel| pixel[0] <= pixel[3] && pixel[1] <= pixel[3] && pixel[2] <= pixel[3])
        );

        let mut frame_count = 1;
        while decoder.advance().unwrap().is_some() {
            frame_count += 1;
        }
        assert!(frame_count > 1);

        decoder.restart().unwrap();
        assert!(decoder.advance().unwrap().is_some());
        let mut restarted = Vec::new();
        decoder.write_premultiplied_frame(&mut restarted).unwrap();
        assert_eq!(restarted, first);
    }
}
