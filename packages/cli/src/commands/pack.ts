/**
 * Engine pack command.
 *
 * Packages game assets and engine files for distribution.
 * Target platforms use CDN asset keys (e.g. windows-amd64, linux-amd64, web-universal).
 */

import { ZipWriter, configure } from '@zip.js/zip.js';
import { spawn } from 'node:child_process';
import { createReadStream, createWriteStream, existsSync } from 'node:fs';
import { cp, mkdir, readFile, readdir, rm, stat } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { Readable, Writable } from 'node:stream';
import { defineCommand } from 'citty';
import consola from 'consola';
import { formatBytes, loadMeta } from '../utils/engine.js';
import { metaFile, platformDir, requireProjectRoot } from '../utils/project.js';

// Disable web workers – not available in Node.js
configure({ useWebWorkers: false });

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const WEB_PLATFORM = 'web-universal';

const NATIVE_EXECUTABLES: Record<string, string> = {
  'windows-amd64': 'moyu.exe',
  'linux-amd64': 'moyu',
  'linux-aarch64': 'moyu',
  'macos-amd64': 'moyu',
  'macos-aarch64': 'moyu',
};

// ---------------------------------------------------------------------------
// Command definition
// ---------------------------------------------------------------------------

export default defineCommand({
  meta: {
    name: 'pack',
    description: 'Package game for distribution',
  },
  args: {
    target: {
      type: 'string',
      description: 'Target platform (e.g. windows-amd64, linux-amd64, web-universal)',
      required: true,
    },
    compress: {
      type: 'boolean',
      description: 'Output as a zip archive',
      default: false,
    },
    output: {
      type: 'string',
      description: 'Output directory path',
    },
  },
  run: async ({ args }) => {
    const projectRoot = requireProjectRoot();
    const metaPath = metaFile(projectRoot);

    // Load metadata to resolve active version
    const meta = await loadMeta(metaPath);
    if (!meta?.active) {
      consola.error('No active engine version. Run "moyu download" first.');
      process.exit(1);
    }

    const activeVersion = meta.active.version;
    const target = args.target;
    const isWeb = target === WEB_PLATFORM;

    // Validate target
    if (!isWeb && !NATIVE_EXECUTABLES[target]) {
      const supported = [...Object.keys(NATIVE_EXECUTABLES), WEB_PLATFORM].join(', ');
      consola.error(`Unsupported target: "${target}".\nSupported targets: ${supported}`);
      process.exit(1);
    }

    const targetPath = platformDir(projectRoot, activeVersion, target);
    if (!existsSync(targetPath)) {
      consola.error(
        `Platform "${target}" not downloaded for version ${activeVersion}.\n` +
        'Run "moyu download" to download it.',
      );
      process.exit(1);
    }

    const compress = args.compress;
    const tmpPackDir = join(projectRoot, '.moyu', 'tmp-pack');

    const dateString = new Date().toISOString().replace(/[-:T.Z]/g, '');
    const outputDir = args.output
      ? resolve(projectRoot, args.output)
      : join(projectRoot, '.moyu', 'release', dateString);

    consola.info(`Target: ${target}`);
    consola.info(`Engine version: ${activeVersion}`);
    consola.info(`Compress: ${compress}`);
    consola.info(`Output: ${outputDir}`);

    // 1. Build the project
    await buildProject(projectRoot);

    // 2. Prepare tmp-pack directory
    if (existsSync(tmpPackDir)) {
      consola.info('Cleaning existing tmp-pack directory...');
      await rm(tmpPackDir, { recursive: true, force: true });
    }
    await mkdir(tmpPackDir, { recursive: true });

    // 3. Copy common files
    await copyAssets(projectRoot, tmpPackDir);
    await copyIndexJson(projectRoot, tmpPackDir);
    await copyBundleJs(projectRoot, tmpPackDir);

    // 4. Copy platform-specific files
    if (isWeb) {
      await copyWebEngine(projectRoot, targetPath, tmpPackDir);
    } else {
      await copyNativeEngine(target, targetPath, tmpPackDir);
    }

    // 5. Output
    if (compress) {
      await createZip(tmpPackDir, outputDir);
    } else {
      await copyToOutput(tmpPackDir, outputDir);
    }

    // 6. Cleanup
    consola.info('Cleaning up tmp-pack...');
    await rm(tmpPackDir, { recursive: true, force: true });

    consola.success('Pack complete!');
  },
});

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

async function buildProject(projectRoot: string): Promise<void> {
  consola.start('Building project with rspack...');

  const rspackBin =
    process.platform === 'win32'
      ? join(projectRoot, 'node_modules', '.bin', 'rspack.cmd')
      : join(projectRoot, 'node_modules', '.bin', 'rspack');

  await new Promise<void>((resolve, reject) => {
    const child = spawn(rspackBin, ['build'], {
      cwd: projectRoot,
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`rspack build exited with code ${code}`));
    });
  });

  consola.success('Build complete.');
}

// ---------------------------------------------------------------------------
// Copy helpers
// ---------------------------------------------------------------------------

async function copyBundleJs(projectRoot: string, tmpPackDir: string): Promise<void> {
  const indexJsonPath = join(projectRoot, 'index.json');
  let entryFilename = 'index.js';

  if (existsSync(indexJsonPath)) {
    try {
      const parsed = JSON.parse(await readFile(indexJsonPath, 'utf-8'));
      if (typeof parsed.entryFilename === 'string' && parsed.entryFilename) {
        entryFilename = parsed.entryFilename.replace(/^\.[/\\]/, '');
      }
    } catch {
      consola.warn('Failed to parse index.json for entryFilename – using fallback "index.js".');
    }
  }

  const srcPath = join(projectRoot, 'dist', entryFilename);
  if (!existsSync(srcPath)) {
    consola.error(`Bundle not found: ${srcPath}`);
    process.exit(1);
  }

  consola.info(`Copying bundle: ${basename(entryFilename)}`);
  await cp(srcPath, join(tmpPackDir, basename(entryFilename)));
}

async function copyAssets(projectRoot: string, tmpPackDir: string): Promise<void> {
  const assetsDir = join(projectRoot, 'assets');
  if (!existsSync(assetsDir)) {
    consola.error('Assets directory not found.');
    process.exit(1);
  }
  consola.info('Copying assets...');
  await cp(assetsDir, join(tmpPackDir, 'assets'), { recursive: true });
}

async function copyIndexJson(projectRoot: string, tmpPackDir: string): Promise<void> {
  const indexJson = join(projectRoot, 'index.json');
  if (!existsSync(indexJson)) {
    consola.error('index.json not found.');
    process.exit(1);
  }
  consola.info('Copying index.json...');
  await cp(indexJson, join(tmpPackDir, 'index.json'));
}

async function copyNativeEngine(target: string, targetPath: string, tmpPackDir: string): Promise<void> {
  const exeName = NATIVE_EXECUTABLES[target];
  const exePath = join(targetPath, exeName);

  if (!existsSync(exePath)) {
    consola.error(`Engine executable not found: ${exePath}\n` + 'Run "moyu download" to download engine files.');
    process.exit(1);
  }

  consola.info(`Copying native engine executable: ${exeName}`);
  await cp(exePath, join(tmpPackDir, exeName));
}

async function copyWebEngine(projectRoot: string, webPath: string, tmpPackDir: string): Promise<void> {
  if (!existsSync(webPath)) {
    consola.error('Web engine directory not found. Run "moyu download" to download web-universal platform.');
    process.exit(1);
  }

  consola.info('Copying web engine files...');
  await cp(webPath, tmpPackDir, { recursive: true });

  const indexHtml = join(projectRoot, 'index.html');
  if (!existsSync(indexHtml)) {
    consola.error('index.html not found in project root.');
    process.exit(1);
  }
  consola.info('Copying index.html...');
  await cp(indexHtml, join(tmpPackDir, 'index.html'));
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

async function addDirToZip(zipWriter: ZipWriter<unknown>, dirPath: string, zipBasePath: string): Promise<void> {
  const entries = await readdir(dirPath, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dirPath, entry.name);
    const entryName = zipBasePath ? `${zipBasePath}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      await addDirToZip(zipWriter, fullPath, entryName);
    } else {
      const readable = Readable.toWeb(createReadStream(fullPath)) as ReadableStream<Uint8Array>;
      await zipWriter.add(entryName, readable, {
        useUnicodeFileNames: true,
        level: 6,
      });
    }
  }
}

async function createZip(tmpPackDir: string, outputDir: string): Promise<void> {
  await mkdir(outputDir, { recursive: true });
  const zipPath = join(outputDir, 'game.zip');

  consola.start(`Creating zip archive: ${zipPath}`);

  const writableStream = Writable.toWeb(createWriteStream(zipPath)) as WritableStream<Uint8Array>;

  const zipWriter = new ZipWriter(writableStream, {
    level: 6,
    useUnicodeFileNames: true,
  });
  await addDirToZip(zipWriter, tmpPackDir, '');
  await zipWriter.close();

  const { size } = await stat(zipPath);
  consola.success(`Archive created: ${zipPath} (${formatBytes(size)})`);
}

async function copyToOutput(tmpPackDir: string, outputDir: string): Promise<void> {
  const gamePath = join(outputDir, 'game');
  await mkdir(gamePath, { recursive: true });

  consola.info(`Copying files to: ${gamePath}`);
  await cp(tmpPackDir, gamePath, { recursive: true });
}
