//! Video decoder plugin download, SHA-256 verification, and caching.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::service::{ServiceReporter, download_to_file_verified};

const VIDEO_DECODER_VERSION: &str = "v0.4.0";
const VIDEO_DECODER_RELEASE_URL: &str =
    "https://github.com/Icemic/video-decoder/releases/download/v0.4.0";

struct VideoDecoderAsset {
    archive: &'static str,
    library: &'static str,
    sha256: &'static str,
}

fn asset_for_target(target: &str) -> Option<VideoDecoderAsset> {
    match target {
        "windows-amd64" => Some(VideoDecoderAsset {
            archive: "windows-x86_64.zip",
            library: "moyu_video.dll",
            sha256: "ebad3130a46402599d16088be3663e1bb95854672d49f6998684e141e7db10c4",
        }),
        "linux-amd64" => Some(VideoDecoderAsset {
            archive: "linux-x86_64.zip",
            library: "libmoyu_video.so",
            sha256: "b982ec8ecb31efe8457f74e807638d94bd8a198f942098f65bd0d50e8cca317e",
        }),
        "linux-aarch64" => Some(VideoDecoderAsset {
            archive: "linux-aarch64.zip",
            library: "libmoyu_video.so",
            sha256: "72081cc8f9512a6359b5e0dae5f04a56fdcd558f22497ec72d61f074f71534f6",
        }),
        "macos-amd64" => Some(VideoDecoderAsset {
            archive: "macos-x86_64.zip",
            library: "libmoyu_video.dylib",
            sha256: "bed21829a2e63369cb5bc335de2e2b0a84979103b72f776adca1db76e5e184c1",
        }),
        "macos-aarch64" => Some(VideoDecoderAsset {
            archive: "macos-aarch64.zip",
            library: "libmoyu_video.dylib",
            sha256: "54bb1aedb6b474b1305e45b9e7af3e000db935da4fec7e991baec5cbec1fe5c6",
        }),
        "android-aarch64" => Some(VideoDecoderAsset {
            archive: "android-aarch64.zip",
            library: "libmoyu_video.so",
            sha256: "0f0909720e064b418a32d06cc3ec86f1e79db1e2e99c27b77771ba5aad8d8f12",
        }),
        _ => None,
    }
}

pub fn supports_video_decoder(target: &str) -> bool {
    asset_for_target(target).is_some()
}

pub async fn ensure_video_decoder_library(
    project_root: &Path,
    target: &str,
    reporter: &ServiceReporter,
) -> Result<PathBuf> {
    let asset = asset_for_target(target)
        .ok_or_else(|| anyhow::anyhow!("Video decoder is not available for target: {target}"))?;

    let cache_dir = project_root
        .join(".moyu")
        .join("video-decoder")
        .join(VIDEO_DECODER_VERSION)
        .join(target);
    let cached_library = cache_dir.join(asset.library);
    if cached_library.is_file() {
        return Ok(cached_library);
    }

    reporter.info(format!(
        "Downloading video decoder {VIDEO_DECODER_VERSION} for {target}..."
    ));
    let url = format!("{VIDEO_DECODER_RELEASE_URL}/{}", asset.archive);
    let tmp_zip = std::env::temp_dir().join(format!(
        "moyu-video-decoder-{}.zip",
        uuid::Uuid::new_v4()
    ));
    download_to_file_verified(&url, &tmp_zip, asset.sha256, reporter).await?;
    let archive_bytes = fs::read(&tmp_zip)?;
    let _ = fs::remove_file(&tmp_zip);
    reporter.cancel.check()?;

    let tmp_dir = project_root
        .join(".moyu")
        .join("video-decoder")
        .join(format!(".tmp-{VIDEO_DECODER_VERSION}-{target}"));
    let library_name = asset.library.to_string();
    let cached_clone = cached_library.clone();
    let cache_clone = cache_dir.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)?;
        }
        fs::create_dir_all(&tmp_dir)?;

        let res = (|| -> Result<()> {
            let cursor = std::io::Cursor::new(&archive_bytes);
            let mut zip = zip::ZipArchive::new(cursor)?;
            let mut extracted = false;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i)?;
                if entry.is_dir() || entry.name()?.as_ref() != library_name {
                    continue;
                }
                let mut out = File::create(tmp_dir.join(&library_name))?;
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::io::Write::write_all(&mut out, &buf)?;
                extracted = true;
                break;
            }
            if !extracted {
                bail!("Video decoder archive does not contain {library_name}.");
            }
            if cache_clone.exists() {
                fs::remove_dir_all(&cache_clone)?;
            }
            fs::create_dir_all(&cache_clone)?;
            fs::copy(tmp_dir.join(&library_name), &cached_clone)?;
            Ok(())
        })();

        let _ = fs::remove_dir_all(&tmp_dir);
        res
    })
    .await??;

    reporter.success(format!("Video decoder cached: {}", cached_library.display()));
    Ok(cached_library)
}
