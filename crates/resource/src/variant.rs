//! Multi-resolution asset variants.
//!
//! A requested asset (`abc.png`) may have variant files carrying a scale suffix
//! (`abc@2x.png`). Which file is used depends on the display ratio, the number of
//! physical pixels one stage unit occupies at the current surface and stage size.
//!
//! The engine cannot list the assets directory, so the existence of a variant can
//! only be determined by reading it. The scales worth trying are therefore
//! configured (`multiResAssetScales`) rather than discovered. See
//! `rfcs/2026-09-30-multi-res-assets.md`.

use std::sync::atomic::{AtomicU32, Ordering};

use moyu_pal::config::{MultiResAssetPolicy, MultiResAssetsMode, get_engine_config};
use moyu_pal::dir::assets_dir;
use moyu_pal::{fs, url::Url};

/// Scale of the base asset, which every project references and which always exists.
const BASE_SCALE: f32 = 1.0;

/// Display ratio, defaulting to `1.0` until the engine reports a real value.
///
/// Stored as IEEE-754 bits so that reads and writes stay lock-free.
static ASSET_SCALE: AtomicU32 = AtomicU32::new(BASE_SCALE.to_bits());

/// Current display ratio, the target scale for asset loading.
pub fn asset_scale() -> f32 {
    f32::from_bits(ASSET_SCALE.load(Ordering::Relaxed))
}

/// Update the display ratio. Called by the engine whenever the surface or stage
/// size changes.
pub fn set_asset_scale(scale: f32) {
    ASSET_SCALE.store(scale.to_bits(), Ordering::Relaxed);
}

/// Read the first candidate of `src` that `load` accepts.
///
/// Returns the URL of the file that was used, its pixel ratio, and whatever
/// `load` produced from its content. The base asset is always among the
/// candidates, so `None` means the asset could not be loaded at all.
pub(crate) async fn load_variant<T, F>(src: &str, mut load: F) -> Option<(Url, f32, T)>
where
    F: FnMut(Vec<u8>) -> Option<T>,
{
    let assets_dir = assets_dir();

    for candidate in candidates(src) {
        let url = match assets_dir.join(&candidate.path) {
            Ok(url) => url,
            Err(err) => {
                log::error!("invalid asset path '{}': {}", candidate.path, err);
                continue;
            }
        };

        let bytes = match fs::read(&url).await {
            Ok(bytes) => bytes,
            Err(err) => {
                // A missing candidate is the normal case for assets without variants.
                log::debug!("asset candidate '{}' is not available: {}", url, err);
                continue;
            }
        };

        match load(bytes) {
            Some(value) => return Some((url, candidate.scale, value)),
            // Some static servers answer unknown paths with an HTML fallback
            // instead of a 404, so undecodable content also counts as a miss.
            None => log::debug!("asset candidate '{}' is not valid content", url),
        }
    }

    None
}

/// Read an asset file, using the multi-resolution variant that matches the
/// current display ratio.
///
/// Returns the URL of the file that was used, its pixel ratio, and its content.
/// Callers that decode the content themselves can use this instead of going
/// through [`ResourceManager`](crate::ResourceManager), which keeps the variant
/// rules in one place.
pub async fn read_asset(src: &str) -> Option<(Url, f32, Vec<u8>)> {
    load_variant(src, Some).await
}

/// A file that may hold the content of a logical asset.
struct Candidate {
    /// Path relative to the assets directory.
    path: String,
    /// Pixel ratio the file holds.
    scale: f32,
}

/// Candidate files for a logical asset path, in the order they should be tried.
fn candidates(src: &str) -> Vec<Candidate> {
    let (_, file_name) = split_path(src);
    let (stem, _) = split_extension(file_name);

    let scales = match split_scale_suffix(stem).1 {
        // A name that carries a scale suffix pins that exact file.
        Some(scale) => vec![scale],
        None => config_scales(),
    };

    scales
        .into_iter()
        .map(|scale| Candidate {
            path: variant_path(src, scale),
            scale,
        })
        .collect()
}

/// Path of the file holding `src` at `scale`. The base scale is `src` itself.
///
/// The scale is written the way [`split_scale_suffix`] reads it: `2` renders as
/// `@2x`, since a float's shortest form drops the trailing `.0`.
fn variant_path(src: &str, scale: f32) -> String {
    if scale == BASE_SCALE {
        return src.to_owned();
    }

    let (dir, file_name) = split_path(src);
    let (stem, ext) = split_extension(file_name);
    let (base_stem, _) = split_scale_suffix(stem);

    format!("{dir}{base_stem}@{scale}x{ext}")
}

/// Scales to try, read from the engine configuration.
fn config_scales() -> Vec<f32> {
    let config = get_engine_config();

    scales_for(
        config.multi_res_assets,
        &config.multi_res_asset_scales,
        config.multi_res_asset_policy,
        asset_scale(),
    )
}

/// Scales to try, ordered and deduplicated. The base asset is always included,
/// as the fallback when no variant exists.
fn scales_for(
    mode: MultiResAssetsMode,
    configured: &[f32],
    policy: MultiResAssetPolicy,
    target: f32,
) -> Vec<f32> {
    match mode {
        // Variants are disabled, use the base asset only.
        MultiResAssetsMode::Off => vec![BASE_SCALE],
        // A fixed scale bypasses the candidate set. It is tried before the base
        // asset, so the order is kept rather than sorted.
        MultiResAssetsMode::Fixed(scale) if scale != BASE_SCALE => vec![scale, BASE_SCALE],
        MultiResAssetsMode::Fixed(_) => vec![BASE_SCALE],
        MultiResAssetsMode::Auto => {
            let mut scales = configured.to_vec();
            scales.push(BASE_SCALE);
            let scales = dedupe(scales);

            match policy {
                MultiResAssetPolicy::Quality => order_quality(scales, target),
                MultiResAssetPolicy::Balanced => order_balanced(scales, target),
            }
        }
    }
}

/// Smallest scale that is not below the target first, then downwards to the base
/// asset, so an asset is never upscaled while a larger variant exists.
fn order_quality(scales: Vec<f32>, target: f32) -> Vec<f32> {
    let (above, below): (Vec<f32>, Vec<f32>) =
        scales.into_iter().partition(|scale| *scale >= target);

    above.into_iter().chain(below.into_iter().rev()).collect()
}

/// Closest to the target in log space; ties prefer the larger scale.
fn order_balanced(mut scales: Vec<f32>, target: f32) -> Vec<f32> {
    scales.sort_by(|left, right| {
        let by_distance = distance_to(*left, target).total_cmp(&distance_to(*right, target));
        by_distance.then_with(|| right.total_cmp(left))
    });

    scales
}

/// Distance between a scale and the target, compared in log space so that
/// halving and doubling count the same.
fn distance_to(scale: f32, target: f32) -> f32 {
    (scale / target).ln().abs()
}

/// Sort and remove duplicates so that a scale is not tried twice.
fn dedupe(mut scales: Vec<f32>) -> Vec<f32> {
    scales.sort_by(f32::total_cmp);
    scales.dedup();
    scales
}

/// Split a path into its directory part, including the trailing slash, and its
/// file name.
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(index) => path.split_at(index + 1),
        None => ("", path),
    }
}

/// Split a file name into its stem and its extension, including the dot.
///
/// The extension starts at the last dot, so names like `icon.small.png` keep
/// their middle dots in the stem.
fn split_extension(file_name: &str) -> (&str, &str) {
    match file_name.rfind('.') {
        Some(index) => file_name.split_at(index),
        None => (file_name, ""),
    }
}

/// Split a `@<scale>x` suffix off a file stem, returning the remaining stem and
/// the scale. A stem without a valid suffix is returned unchanged.
fn split_scale_suffix(stem: &str) -> (&str, Option<f32>) {
    let Some(index) = stem.rfind('@') else {
        return (stem, None);
    };

    let suffix = &stem[index + 1..];
    let digits = suffix.strip_suffix('x').or_else(|| suffix.strip_suffix('X'));

    match digits.and_then(|digits| digits.parse::<f32>().ok()) {
        // Zero and negative scales are meaningless.
        Some(scale) if scale > 0.0 => (&stem[..index], Some(scale)),
        _ => (stem, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_scale_suffix() {
        assert_eq!(split_scale_suffix("room"), ("room", None));
        assert_eq!(split_scale_suffix("room@2x"), ("room", Some(2.0)));
        assert_eq!(split_scale_suffix("room@1.5x"), ("room", Some(1.5)));
        assert_eq!(split_scale_suffix("room@2X"), ("room", Some(2.0)));
        assert_eq!(split_scale_suffix("a@b@2x"), ("a@b", Some(2.0)));

        // Names that are not scale suffixes stay untouched.
        assert_eq!(split_scale_suffix("room@2"), ("room@2", None));
        assert_eq!(split_scale_suffix("room@0x"), ("room@0x", None));
        assert_eq!(split_scale_suffix("room@-1x"), ("room@-1x", None));
        assert_eq!(split_scale_suffix("room@1.5.5x"), ("room@1.5.5x", None));
        assert_eq!(split_scale_suffix("mail@address"), ("mail@address", None));
    }

    #[test]
    fn splits_extension_at_last_dot() {
        assert_eq!(split_extension("room.png"), ("room", ".png"));
        assert_eq!(split_extension("icon.small.png"), ("icon.small", ".png"));
        assert_eq!(split_extension("room"), ("room", ""));
    }

    #[test]
    fn builds_variant_paths() {
        assert_eq!(variant_path("bg/room.png", BASE_SCALE), "bg/room.png");
        assert_eq!(variant_path("bg/room.png", 2.0), "bg/room@2x.png");
        assert_eq!(variant_path("bg/room.png", 1.5), "bg/room@1.5x.png");
        assert_eq!(variant_path("ui/icon.small.png", 2.0), "ui/icon.small@2x.png");
        // A path that already carries a scale is rewritten, not appended to.
        assert_eq!(variant_path("bg/room@2x.png", 1.5), "bg/room@1.5x.png");
        assert_eq!(variant_path("room", 2.0), "room@2x");
    }

    /// The generated name has to be readable by the parser, or a variant could
    /// never be found again once written.
    #[test]
    fn generated_paths_round_trip() {
        for scale in [0.5, 1.25, 1.5, 2.0, 3.0] {
            let path = variant_path("bg/room.png", scale);
            let (_, file_name) = split_path(&path);
            let (stem, _) = split_extension(file_name);

            assert_eq!(split_scale_suffix(stem).1, Some(scale), "path {path}");
        }
    }

    #[test]
    fn orders_scales_by_policy() {
        let scales = || dedupe(vec![1.5, 2.0, BASE_SCALE]);

        assert_eq!(order_quality(scales(), 2.0), vec![2.0, 1.5, 1.0]);
        assert_eq!(order_quality(scales(), 1.33), vec![1.5, 2.0, 1.0]);
        // The base asset comes first, so no variant is probed at all.
        assert_eq!(order_quality(scales(), 0.67), vec![1.0, 1.5, 2.0]);

        assert_eq!(order_balanced(scales(), 2.0), vec![2.0, 1.5, 1.0]);
        assert_eq!(order_balanced(scales(), 1.1), vec![1.0, 1.5, 2.0]);
        // Scales mirrored around the target are equally distant, and the larger
        // one wins the tie.
        assert_eq!(order_balanced(vec![4.0, 1.0], 2.0), vec![4.0, 1.0]);
    }

    #[test]
    fn reads_candidate_scales_from_config() {
        let auto = MultiResAssetsMode::Auto;
        let quality = MultiResAssetPolicy::Quality;

        assert_eq!(
            scales_for(auto, &[1.5, 2.0], quality, 2.0),
            vec![2.0, 1.5, 1.0]
        );
        // An empty list means the base asset only.
        assert_eq!(scales_for(auto, &[], quality, 2.0), vec![1.0]);
        // An unset target keeps the base asset first.
        assert_eq!(scales_for(auto, &[1.5], quality, 1.0), vec![1.0, 1.5]);
        // Duplicates and the base asset in the list collapse.
        assert_eq!(scales_for(auto, &[2.0, 2.0, 1.0], quality, 2.0), vec![2.0, 1.0]);

        assert_eq!(scales_for(MultiResAssetsMode::Off, &[2.0], quality, 2.0), vec![1.0]);
        // A fixed scale wins over the ratio, ordering aside from the policy.
        assert_eq!(
            scales_for(MultiResAssetsMode::Fixed(1.5), &[2.0], quality, 2.0),
            vec![1.5, 1.0]
        );
        assert_eq!(
            scales_for(MultiResAssetsMode::Fixed(1.0), &[2.0], quality, 2.0),
            vec![1.0]
        );
    }
}
