import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { chmod, cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { isAbsolute, join, normalize, relative, resolve, sep } from 'node:path';
import { StringDecoder } from 'node:string_decoder';
import { Uint8ArrayReader, Uint8ArrayWriter, ZipReader } from '@zip.js/zip.js';
import consola from 'consola';

export const ANDROID_PLATFORM = 'android-aarch64';
export const ANDROID_FORMATS = ['debug-apk', 'release-apk', 'release-aab', 'android-project'] as const;

const ANDROID_TEMPLATE_VERSION = '0.0.1';
const ANDROID_TEMPLATE_URL = 'https://github.com/DeepSpaceMill/moyu-android/archive/refs/tags/v0.0.1.zip';
const ANDROID_TEMPLATE_SHA256 = 'ffdac6565403eb99d6ffed3462075ddde1ec43179f44dd0433974cece8f4104c';

type AndroidFormat = (typeof ANDROID_FORMATS)[number];

interface AndroidConfig {
  applicationId: string;
  appName: string;
  versionCode: number;
  versionName: string;
  orientation: string;
  icon: {
    source: string;
    background: {
      color: string;
    };
    foregroundPadding: number;
  };
  signing: {
    keystorePath: string;
    keyAlias: string;
  };
}

interface PackAndroidOptions {
  projectRoot: string;
  engineDir: string;
  runtimePackageDir: string;
  outputDir: string;
  format: string | undefined;
}

interface AndroidSigning {
  keystorePath: string;
  keyAlias: string;
  password: string;
}

const DEFAULT_ANDROID_CONFIG: AndroidConfig = {
  applicationId: 'com.example.game',
  appName: 'My Visual Novel',
  versionCode: 1,
  versionName: '1.0.0',
  orientation: 'landscape',
  icon: {
    source: '',
    background: {
      color: '#ffffff',
    },
    foregroundPadding: 0.18,
  },
  signing: {
    keystorePath: '',
    keyAlias: '',
  },
};

export function isAndroidFormat(value: string | undefined): value is AndroidFormat {
  return ANDROID_FORMATS.includes(value as AndroidFormat);
}

export async function packAndroid(options: PackAndroidOptions): Promise<void> {
  if (!isAndroidFormat(options.format)) {
    throw new Error(`Invalid --android-format. Supported values: ${ANDROID_FORMATS.join(', ')}`);
  }

  const config = await loadAndroidConfig(options.projectRoot);
  validateAndroidConfig(config);

  const templateDir = await ensureAndroidTemplate(options.projectRoot);
  const workdir = await ensureAndroidWorkdir(options.projectRoot, templateDir);

  consola.info('Syncing Android project...');
  await syncAndroidProject(
    options.projectRoot,
    options.engineDir,
    options.runtimePackageDir,
    templateDir,
    workdir,
    config,
  );

  if (options.format === 'android-project') {
    const outputPath = join(options.outputDir, 'android-project');
    consola.info(`Exporting Android Studio project: ${outputPath}`);
    await copyAndroidProject(workdir, outputPath);
    return;
  }

  const signing = options.format === 'debug-apk' ? undefined : await resolveAndroidSigning(options.projectRoot, config);
  const artifact = await runGradleBuild(workdir, options.format, signing);
  const extension = options.format === 'release-aab' ? 'aab' : 'apk';
  const outputPath = join(options.outputDir, `game-${options.format}.${extension}`);
  await mkdir(options.outputDir, { recursive: true });
  await cp(artifact, outputPath);
  consola.success(`Android artifact created: ${outputPath}`);
}

async function loadAndroidConfig(projectRoot: string): Promise<AndroidConfig> {
  const configPath = join(projectRoot, 'moyu.json');
  if (!existsSync(configPath)) return structuredClone(DEFAULT_ANDROID_CONFIG);

  let raw: unknown;
  try {
    raw = JSON.parse(await readFile(configPath, 'utf-8'));
  } catch {
    throw new Error('Failed to parse moyu.json.');
  }

  const android = isRecord(raw) && isRecord(raw.android) ? raw.android : {};
  const icon = isRecord(android.icon) ? android.icon : {};
  const background = isRecord(icon.background) ? icon.background : {};
  const signing = isRecord(android.signing) ? android.signing : {};

  return {
    applicationId: stringValue(android.applicationId, DEFAULT_ANDROID_CONFIG.applicationId),
    appName: stringValue(android.appName, DEFAULT_ANDROID_CONFIG.appName),
    versionCode: numberValue(android.versionCode, DEFAULT_ANDROID_CONFIG.versionCode),
    versionName: stringValue(android.versionName, DEFAULT_ANDROID_CONFIG.versionName),
    orientation: stringValue(android.orientation, DEFAULT_ANDROID_CONFIG.orientation),
    icon: {
      source: stringValue(icon.source, DEFAULT_ANDROID_CONFIG.icon.source),
      background: {
        color: stringValue(background.color, DEFAULT_ANDROID_CONFIG.icon.background.color),
      },
      foregroundPadding: numberValue(icon.foregroundPadding, DEFAULT_ANDROID_CONFIG.icon.foregroundPadding),
    },
    signing: {
      keystorePath: stringValue(signing.keystorePath, DEFAULT_ANDROID_CONFIG.signing.keystorePath),
      keyAlias: stringValue(signing.keyAlias, DEFAULT_ANDROID_CONFIG.signing.keyAlias),
    },
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function stringValue(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback;
}

function numberValue(value: unknown, fallback: number): number {
  return typeof value === 'number' ? value : fallback;
}

function validateAndroidConfig(config: AndroidConfig): void {
  if (
    !/^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$/.test(config.applicationId) ||
    config.applicationId.startsWith('ink.momoyu')
  ) {
    throw new Error('Invalid Android application ID.');
  }
  if (!config.appName.trim()) throw new Error('Android app name is required.');
  if (!Number.isInteger(config.versionCode) || config.versionCode <= 0) {
    throw new Error('Android version code must be a positive integer.');
  }
  if (!config.versionName.trim() || /[\r\n]/.test(config.versionName)) {
    throw new Error('Invalid Android version name.');
  }
  if (!['landscape', 'portrait', 'sensor'].includes(config.orientation)) {
    throw new Error('Android orientation must be landscape, portrait, or sensor.');
  }
  if (!/^#[0-9a-fA-F]{6}$/.test(config.icon.background.color.trim())) {
    throw new Error('Android icon background color must use #RRGGBB format.');
  }
  assertRelativeConfigPath(config.icon.source, 'Android icon source');
  assertRelativeConfigPath(config.signing.keystorePath, 'Android keystore path');
}

function assertRelativeConfigPath(value: string, label: string): void {
  if (value.trim() && isAbsolute(value)) {
    throw new Error(`${label} must be relative to the project directory.`);
  }
}

async function ensureAndroidTemplate(projectRoot: string): Promise<string> {
  const cacheDir = join(projectRoot, '.moyu', 'android-template', ANDROID_TEMPLATE_VERSION);
  if (existsSync(cacheDir)) return cacheDir;

  consola.start(`Downloading Android template ${ANDROID_TEMPLATE_VERSION}...`);
  const response = await fetch(ANDROID_TEMPLATE_URL, { cache: 'no-store' });
  if (!response.ok) {
    throw new Error(`Failed to download Android template: HTTP ${response.status} ${response.statusText}`);
  }

  const archive = new Uint8Array(await response.arrayBuffer());
  const actualSha256 = createHash('sha256').update(archive).digest('hex');
  if (actualSha256 !== ANDROID_TEMPLATE_SHA256) {
    throw new Error(
      `Android template SHA-256 mismatch.\nExpected: ${ANDROID_TEMPLATE_SHA256}\nActual:   ${actualSha256}`,
    );
  }

  const tmpDir = join(projectRoot, '.moyu', 'android-template', `.tmp-${ANDROID_TEMPLATE_VERSION}`);
  await rm(tmpDir, { recursive: true, force: true });
  await mkdir(tmpDir, { recursive: true });
  try {
    await extractTemplateArchive(archive, tmpDir);
    await mkdir(join(projectRoot, '.moyu', 'android-template'), { recursive: true });
    await cp(join(tmpDir, `moyu-android-${ANDROID_TEMPLATE_VERSION}`), cacheDir, { recursive: true });
  } finally {
    await rm(tmpDir, { recursive: true, force: true });
  }

  consola.success('Android template cached.');
  return cacheDir;
}

async function extractTemplateArchive(archive: Uint8Array, destDir: string): Promise<void> {
  const reader = new ZipReader(new Uint8ArrayReader(archive));
  try {
    for (const entry of await reader.getEntries()) {
      const entryPath = normalize(entry.filename.replaceAll('\\', '/'));
      if (isAbsolute(entryPath) || entryPath === '..' || entryPath.startsWith(`..${sep}`)) {
        throw new Error(`Unsafe Android template archive entry: ${entry.filename}`);
      }

      const outputPath = resolve(destDir, entryPath);
      const relativePath = relative(destDir, outputPath);
      if (relativePath === '..' || relativePath.startsWith(`..${sep}`) || isAbsolute(relativePath)) {
        throw new Error(`Unsafe Android template archive entry: ${entry.filename}`);
      }

      if (entry.directory) {
        await mkdir(outputPath, { recursive: true });
      } else {
        await mkdir(resolve(outputPath, '..'), { recursive: true });
        await writeFile(outputPath, await entry.getData(new Uint8ArrayWriter()));
      }
    }
  } finally {
    await reader.close();
  }
}

async function ensureAndroidWorkdir(projectRoot: string, templateDir: string): Promise<string> {
  const workdir = join(projectRoot, '.moyu', 'android-build', ANDROID_PLATFORM);
  if (!existsSync(workdir)) {
    consola.info('Initializing Android work directory...');
    await mkdir(join(projectRoot, '.moyu', 'android-build'), { recursive: true });
    await cp(templateDir, workdir, { recursive: true });
  }
  return workdir;
}

async function syncAndroidProject(
  projectRoot: string,
  engineDir: string,
  runtimePackageDir: string,
  templateDir: string,
  workdir: string,
  config: AndroidConfig,
): Promise<void> {
  const appDir = join(workdir, 'app');
  const mainDir = join(appDir, 'src', 'main');
  const assetsDir = join(mainDir, 'assets');
  const resDir = join(mainDir, 'res');

  await rm(assetsDir, { recursive: true, force: true });
  await cp(runtimePackageDir, assetsDir, { recursive: true });

  const engineLibrary = join(engineDir, 'libmoyu.so');
  if (!existsSync(engineLibrary)) throw new Error(`Android engine library not found: ${engineLibrary}`);
  const jniLibsDir = join(mainDir, 'jniLibs', 'arm64-v8a');
  await mkdir(jniLibsDir, { recursive: true });
  await cp(engineLibrary, join(jniLibsDir, 'libmoyu.so'));

  await rm(resDir, { recursive: true, force: true });
  await cp(join(templateDir, 'app', 'src', 'main', 'res'), resDir, { recursive: true });
  await syncLauncherIcons(projectRoot, resDir, config);
  await writeAndroidProperties(workdir, config);
  await writeAppName(resDir, config.appName);
}

async function syncLauncherIcons(projectRoot: string, resDir: string, config: AndroidConfig): Promise<void> {
  const source = config.icon.source.trim();
  if (!source) return;

  const sourcePath = resolve(projectRoot, source);
  if (!existsSync(sourcePath)) throw new Error(`Android icon source not found: ${sourcePath}`);
  if (!source.toLowerCase().endsWith('.png')) throw new Error('Android icon source must be a PNG file.');

  const anydpiDir = join(resDir, 'mipmap-anydpi-v26');
  const drawableDir = join(resDir, 'drawable');
  const drawableV24Dir = join(resDir, 'drawable-v24');
  const drawableNodpiDir = join(resDir, 'drawable-nodpi');
  await Promise.all(
    [anydpiDir, drawableDir, drawableV24Dir, drawableNodpiDir].map((dir) => mkdir(dir, { recursive: true })),
  );

  const adaptiveIcon =
    '<?xml version="1.0" encoding="utf-8"?>\n' +
    '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n' +
    '    <background android:drawable="@drawable/ic_launcher_background" />\n' +
    '    <foreground android:drawable="@drawable/ic_launcher_foreground" />\n' +
    '</adaptive-icon>\n';
  await writeFile(join(anydpiDir, 'ic_launcher.xml'), adaptiveIcon);
  await writeFile(join(anydpiDir, 'ic_launcher_round.xml'), adaptiveIcon);
  await writeFile(
    join(drawableDir, 'ic_launcher_background.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n' +
      '<shape xmlns:android="http://schemas.android.com/apk/res/android" android:shape="rectangle">\n' +
      `    <solid android:color="${config.icon.background.color.trim()}" />\n` +
      '</shape>\n',
  );

  const padding = Number.isFinite(config.icon.foregroundPadding)
    ? Math.min(Math.max(config.icon.foregroundPadding, 0), 0.5) * 108
    : DEFAULT_ANDROID_CONFIG.icon.foregroundPadding * 108;
  await writeFile(
    join(drawableV24Dir, 'ic_launcher_foreground.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n' +
      '<inset xmlns:android="http://schemas.android.com/apk/res/android"\n' +
      '    android:drawable="@drawable/ic_launcher_foreground_base"\n' +
      `    android:insetLeft="${padding.toFixed(2)}dp"\n` +
      `    android:insetTop="${padding.toFixed(2)}dp"\n` +
      `    android:insetRight="${padding.toFixed(2)}dp"\n` +
      `    android:insetBottom="${padding.toFixed(2)}dp" />\n`,
  );
  await cp(sourcePath, join(drawableNodpiDir, 'ic_launcher_foreground_base.png'));

  for (const density of ['mdpi', 'hdpi', 'xhdpi', 'xxhdpi', 'xxxhdpi']) {
    const dir = join(resDir, `mipmap-${density}`);
    await mkdir(dir, { recursive: true });
    await Promise.all([
      rm(join(dir, 'ic_launcher.webp'), { force: true }),
      rm(join(dir, 'ic_launcher_round.webp'), { force: true }),
    ]);
    await cp(sourcePath, join(dir, 'ic_launcher.png'));
    await cp(sourcePath, join(dir, 'ic_launcher_round.png'));
  }
}

async function writeAndroidProperties(workdir: string, config: AndroidConfig): Promise<void> {
  const orientation =
    config.orientation === 'portrait'
      ? 'sensorPortrait'
      : config.orientation === 'sensor'
        ? 'fullSensor'
        : 'sensorLandscape';
  const versionName = config.versionName.replaceAll('\\', '\\\\');
  await writeFile(
    join(workdir, 'moyu.properties'),
    `MOYU_APPLICATION_ID=${config.applicationId}\n` +
      `MOYU_VERSION_CODE=${config.versionCode}\n` +
      `MOYU_VERSION_NAME=${versionName}\n` +
      `MOYU_ORIENTATION=${orientation}\n`,
  );
}

async function writeAppName(resDir: string, appName: string): Promise<void> {
  const escaped = appName
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;');
  const valuesDir = join(resDir, 'values');
  await mkdir(valuesDir, { recursive: true });
  await writeFile(
    join(valuesDir, 'strings.xml'),
    `<?xml version="1.0" encoding="utf-8"?>\n<resources>\n    <string name="app_name">${escaped}</string>\n</resources>\n`,
  );
}

async function resolveAndroidSigning(projectRoot: string, config: AndroidConfig): Promise<AndroidSigning> {
  if (!config.signing.keystorePath.trim()) throw new Error('Android keystore path is required for release builds.');
  if (!config.signing.keyAlias.trim()) throw new Error('Android signing key alias is required for release builds.');

  const keystorePath = resolve(projectRoot, config.signing.keystorePath);
  if (!existsSync(keystorePath)) throw new Error(`Android keystore not found: ${keystorePath}`);

  const password = await promptHidden('Android signing password: ');
  if (!password) throw new Error('Android signing password is required.');
  return { keystorePath, keyAlias: config.signing.keyAlias.trim(), password };
}

async function promptHidden(message: string): Promise<string> {
  if (!process.stdin.isTTY || !process.stdout.isTTY || typeof process.stdin.setRawMode !== 'function') {
    throw new Error('Android release signing password must be entered in an interactive terminal.');
  }

  return new Promise((resolvePrompt, reject) => {
    let value = '';
    const stdin = process.stdin;
    const decoder = new StringDecoder('utf8');

    const cleanup = () => {
      stdin.off('data', onData);
      stdin.setRawMode(false);
      stdin.pause();
    };
    const finish = (result: string) => {
      cleanup();
      process.stdout.write('\n');
      resolvePrompt(result);
    };
    const onData = (chunk: Buffer) => {
      for (const character of decoder.write(chunk)) {
        if (character === '\u0003') {
          cleanup();
          process.stdout.write('\n');
          reject(new Error('Cancelled.'));
          return;
        }
        if (character === '\r' || character === '\n') {
          finish(value);
          return;
        }
        if (character === '\b' || character === '\u007f') {
          value = Array.from(value).slice(0, -1).join('');
          continue;
        }
        value += character;
      }
    };

    process.stdout.write(message);
    stdin.setRawMode(true);
    stdin.resume();
    stdin.on('data', onData);
  });
}

async function runGradleBuild(
  workdir: string,
  format: Exclude<AndroidFormat, 'android-project'>,
  signing?: AndroidSigning,
): Promise<string> {
  const task =
    format === 'debug-apk'
      ? ':app:assembleDebug'
      : format === 'release-apk'
        ? ':app:assembleRelease'
        : ':app:bundleRelease';
  const gradlew = join(workdir, process.platform === 'win32' ? 'gradlew.bat' : 'gradlew');
  if (!existsSync(gradlew)) throw new Error(`Gradle wrapper not found: ${gradlew}`);
  if (process.platform !== 'win32') await chmod(gradlew, 0o755);

  consola.start(`Running Gradle ${task}...`);
  await new Promise<void>((resolveBuild, reject) => {
    const child = spawn(gradlew, [task], {
      cwd: workdir,
      stdio: 'inherit',
      env: signing
        ? {
            ...process.env,
            MOYU_KEYSTORE_PATH: signing.keystorePath,
            MOYU_STORE_PASSWORD: signing.password,
            MOYU_KEY_ALIAS: signing.keyAlias,
            MOYU_KEY_PASSWORD: signing.password,
          }
        : process.env,
    });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) resolveBuild();
      else reject(new Error(`Gradle build exited with code ${code}`));
    });
  });

  const artifact =
    format === 'debug-apk'
      ? join(workdir, 'app', 'build', 'outputs', 'apk', 'debug', 'app-debug.apk')
      : format === 'release-apk'
        ? join(workdir, 'app', 'build', 'outputs', 'apk', 'release', 'app-release.apk')
        : join(workdir, 'app', 'build', 'outputs', 'bundle', 'release', 'app-release.aab');
  if (!existsSync(artifact)) throw new Error(`Gradle reported success but artifact was not found: ${artifact}`);
  return artifact;
}

async function copyAndroidProject(workdir: string, outputDir: string): Promise<void> {
  const excluded = new Set(['.git', '.gradle', '.idea', 'build', `app${sep}build`, 'local.properties']);
  const relativeOutput = relative(workdir, outputDir);
  const relativeWorkdir = relative(outputDir, workdir);
  const outputContainsWorkdir =
    !relativeWorkdir || (!relativeWorkdir.startsWith(`..${sep}`) && relativeWorkdir !== '..');
  if (!relativeOutput || (!relativeOutput.startsWith(`..${sep}`) && relativeOutput !== '..') || outputContainsWorkdir) {
    throw new Error(`Unsafe Android project output path: ${outputDir}`);
  }
  await rm(outputDir, { recursive: true, force: true });
  await cp(workdir, outputDir, {
    recursive: true,
    filter: (source) => {
      const entry = relative(workdir, source);
      return !excluded.has(entry);
    },
  });
}
