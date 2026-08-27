import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { cp, mkdir, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { Uint8ArrayReader, Uint8ArrayWriter, ZipReader } from '@zip.js/zip.js';
import consola from 'consola';

const VIDEO_DECODER_VERSION = 'v0.4.0';
const VIDEO_DECODER_RELEASE_URL =
  'https://github.com/Icemic/video-decoder/releases/download/v0.4.0';

interface VideoDecoderAsset {
  archive: string;
  library: string;
  sha256: string;
}

const VIDEO_DECODER_ASSETS: Record<string, VideoDecoderAsset> = {
  'windows-amd64': {
    archive: 'windows-x86_64.zip',
    library: 'moyu_video.dll',
    sha256: 'ebad3130a46402599d16088be3663e1bb95854672d49f6998684e141e7db10c4',
  },
  'linux-amd64': {
    archive: 'linux-x86_64.zip',
    library: 'libmoyu_video.so',
    sha256: 'b982ec8ecb31efe8457f74e807638d94bd8a198f942098f65bd0d50e8cca317e',
  },
  'linux-aarch64': {
    archive: 'linux-aarch64.zip',
    library: 'libmoyu_video.so',
    sha256: '72081cc8f9512a6359b5e0dae5f04a56fdcd558f22497ec72d61f074f71534f6',
  },
  'macos-amd64': {
    archive: 'macos-x86_64.zip',
    library: 'libmoyu_video.dylib',
    sha256: 'bed21829a2e63369cb5bc335de2e2b0a84979103b72f776adca1db76e5e184c1',
  },
  'macos-aarch64': {
    archive: 'macos-aarch64.zip',
    library: 'libmoyu_video.dylib',
    sha256: '54bb1aedb6b474b1305e45b9e7af3e000db935da4fec7e991baec5cbec1fe5c6',
  },
  'android-aarch64': {
    archive: 'android-aarch64.zip',
    library: 'libmoyu_video.so',
    sha256: '0f0909720e064b418a32d06cc3ec86f1e79db1e2e99c27b77771ba5aad8d8f12',
  },
};

export function supportsVideoDecoder(target: string): boolean {
  return target in VIDEO_DECODER_ASSETS;
}

export async function ensureVideoDecoderLibrary(projectRoot: string, target: string): Promise<string> {
  const asset = VIDEO_DECODER_ASSETS[target];
  if (!asset) throw new Error(`Video decoder is not available for target: ${target}`);

  const cacheDir = join(projectRoot, '.moyu', 'video-decoder', VIDEO_DECODER_VERSION, target);
  const cachedLibrary = join(cacheDir, asset.library);
  if (existsSync(cachedLibrary)) return cachedLibrary;

  consola.start(`Downloading video decoder ${VIDEO_DECODER_VERSION} for ${target}...`);
  const response = await fetch(`${VIDEO_DECODER_RELEASE_URL}/${asset.archive}`, { cache: 'no-store' });
  if (!response.ok) {
    throw new Error(`Failed to download video decoder: HTTP ${response.status} ${response.statusText}`);
  }

  const archive = new Uint8Array(await response.arrayBuffer());
  const actualSha256 = createHash('sha256').update(archive).digest('hex');
  if (actualSha256 !== asset.sha256) {
    throw new Error(
      `Video decoder SHA-256 mismatch.\nExpected: ${asset.sha256}\nActual:   ${actualSha256}`,
    );
  }

  const cacheRoot = join(projectRoot, '.moyu', 'video-decoder');
  const tmpDir = join(cacheRoot, `.tmp-${VIDEO_DECODER_VERSION}-${target}`);
  await rm(tmpDir, { recursive: true, force: true });
  await mkdir(tmpDir, { recursive: true });
  try {
    await extractVideoDecoderLibrary(archive, tmpDir, asset.library);
    await rm(cacheDir, { recursive: true, force: true });
    await mkdir(cacheDir, { recursive: true });
    await cp(join(tmpDir, asset.library), cachedLibrary);
  } finally {
    await rm(tmpDir, { recursive: true, force: true });
  }

  consola.success(`Video decoder cached: ${cachedLibrary}`);
  return cachedLibrary;
}

async function extractVideoDecoderLibrary(archive: Uint8Array, destDir: string, library: string): Promise<void> {
  const reader = new ZipReader(new Uint8ArrayReader(archive));
  try {
    for (const entry of await reader.getEntries()) {
      if (entry.directory || entry.filename !== library) continue;
      await writeFile(join(destDir, library), await entry.getData(new Uint8ArrayWriter()));
      return;
    }
    throw new Error(`Video decoder archive does not contain ${library}.`);
  } finally {
    await reader.close();
  }
}
