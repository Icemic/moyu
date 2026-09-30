/**
 * Multi-resolution asset preflight.
 *
 * A project can ship variants of an asset at several pixel ratios (`abc@2x.png`).
 * The engine picks one at load time, but it cannot list the assets directory, so
 * it only ever tries the scales listed in `multiResAssetScales`. A variant that
 * the configuration does not cover is dead weight: it ships inside the package
 * and is never loaded. These checks surface that before the project is run or
 * packed. Findings are warnings; nothing is blocked.
 *
 * The naming rules mirror `crates/resource/src/variant.rs`.
 */

import { existsSync } from 'node:fs';
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import consola from 'consola';

/** Scale of the base asset, which the engine always considers available. */
const BASE_SCALE = 1;

interface VariantFile {
  /** Path relative to `assets/`, with forward slashes. */
  path: string;
  /** Pixel ratio in the file name. */
  scale: number;
  /** Path of the asset this variant belongs to, relative to `assets/`. */
  base: string;
}

interface MultiResConfig {
  /** `"off"`, `"auto"`, or a forced scale. */
  mode: 'off' | 'auto' | number;
  /** Candidate scales besides the base asset. */
  scales: number[];
}

/**
 * Warn about variant files the current configuration cannot use.
 *
 * Silently does nothing when the project has no variants, since there is nothing
 * to report in that case.
 */
export async function checkMultiResAssets(projectRoot: string): Promise<void> {
  const assetsDir = join(projectRoot, 'assets');
  const variants = await scanVariants(assetsDir);
  if (variants.length === 0) return;

  const config = await readMultiResConfig(projectRoot);
  reportUnusedScales(variants, config);
  reportUndeclaredScales(variants, config);
  reportMissingBase(variants, assetsDir);
}

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

/**
 * Variants whose scale the engine will never try: the feature is off, or the
 * scale is missing from the candidate list.
 */
function reportUnusedScales(variants: VariantFile[], config: MultiResConfig): void {
  if (config.mode === 'off') {
    consola.warn(
      `Found ${count(variants.length, 'variant file')}, but multiResAssets is "off", so only the base ` +
        'assets are used.\nSet "multiResAssets": "auto" in index.json to use them.',
    );
    return;
  }

  const used = config.mode === 'auto' ? new Set(config.scales) : new Set([config.mode]);
  const unused = groupByScale(variants, (variant) => !used.has(variant.scale));

  for (const [scale, files] of unused) {
    const hint =
      scale === BASE_SCALE
        ? 'the base asset already covers scale 1, so it is never used'
        : config.mode === 'auto'
          ? 'add it to multiResAssetScales in index.json to use them'
          : `multiResAssets forces scale ${config.mode}`;

    consola.warn(
      `Scale ${scale} is not tried by the engine (${count(files.length, 'file')}, e.g. ${files[0].path}): ${hint}`,
    );
  }
}

/** Candidate scales that no file uses, which costs one failed read per asset. */
function reportUndeclaredScales(variants: VariantFile[], config: MultiResConfig): void {
  if (config.mode !== 'auto') return;

  const present = new Set(variants.map((variant) => variant.scale));
  const missing = config.scales.filter((scale) => !present.has(scale));
  if (missing.length === 0) return;

  consola.info(
    `multiResAssetScales declares ${missing.join(', ')}, but no variant file uses ` +
      `${missing.length === 1 ? 'it' : 'them'}. Every asset will make one extra failed read for each.`,
  );
}

/** Variants with no base asset, which nothing can reference. */
function reportMissingBase(variants: VariantFile[], assetsDir: string): void {
  const missing = variants.filter((variant) => !existsSync(join(assetsDir, variant.base)));
  if (missing.length === 0) return;

  consola.warn(
    `No base asset for ${count(missing.length, 'variant file')}, so nothing can reference ` +
      `${missing.length === 1 ? 'it' : 'them'}:\n` +
      missing.map((variant) => `  ${variant.path} (expected ${variant.base})`).join('\n'),
  );
}

// ---------------------------------------------------------------------------
// Project scan
// ---------------------------------------------------------------------------

/** Collect every variant file under `assetsDir`. */
async function scanVariants(assetsDir: string): Promise<VariantFile[]> {
  if (!existsSync(assetsDir)) return [];

  const found: VariantFile[] = [];
  await walk(assetsDir, assetsDir, found);
  return found;
}

async function walk(dir: string, assetsDir: string, found: VariantFile[]): Promise<void> {
  const entries = await readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(dir, entry.name);

    if (entry.isDirectory()) {
      await walk(fullPath, assetsDir, found);
      continue;
    }

    const scale = parseVariantScale(entry.name);
    if (scale === null) continue;

    const relative = fullPath.slice(assetsDir.length + 1).split(/[\\/]/).join('/');
    const base = relative.slice(0, relative.lastIndexOf('/') + 1) + baseName(entry.name);
    found.push({ path: relative, scale, base });
  }
}

/** Read the multi-resolution fields from `index.json`. */
async function readMultiResConfig(projectRoot: string): Promise<MultiResConfig> {
  const indexPath = join(projectRoot, 'index.json');
  /** Matches `MoyuConfig::default`: variants are off unless the project asks for them. */
  const fallback: MultiResConfig = { mode: 'off', scales: [] };

  if (!existsSync(indexPath)) return fallback;

  let config: Record<string, unknown>;
  try {
    config = JSON.parse(await readFile(indexPath, 'utf-8')) as Record<string, unknown>;
  } catch {
    // A malformed index.json is reported when the engine loads it.
    return fallback;
  }

  const mode = config.multiResAssets;
  const scales = Array.isArray(config.multiResAssetScales)
    ? config.multiResAssetScales.filter((scale): scale is number => typeof scale === 'number' && scale > 0)
    : [1.5, 2];

  return {
    mode: typeof mode === 'number' ? mode : mode === 'auto' ? 'auto' : 'off',
    scales: scales.filter((scale) => scale !== BASE_SCALE),
  };
}

// ---------------------------------------------------------------------------
// File name parsing
// ---------------------------------------------------------------------------

/**
 * Scale in a `@<scale>x` suffix, or `null` when the name has none.
 *
 * Mirrors the engine: the suffix sits in the stem, so the extension is taken
 * after the last dot and the suffix is separated on the last `@`.
 */
function parseVariantScale(fileName: string): number | null {
  const dot = fileName.lastIndexOf('.');
  const stem = dot === -1 ? fileName : fileName.slice(0, dot);
  const at = stem.lastIndexOf('@');
  if (at === -1) return null;

  const suffix = stem.slice(at + 1);
  const digits = suffix.endsWith('x') || suffix.endsWith('X') ? suffix.slice(0, -1) : null;
  if (digits === null || !/^\d+(\.\d+)?$/.test(digits)) return null;

  const scale = Number(digits);
  return scale > 0 ? scale : null;
}

/** Base asset name for a variant file name. */
function baseName(fileName: string): string {
  const dot = fileName.lastIndexOf('.');
  const stem = dot === -1 ? fileName : fileName.slice(0, dot);
  const extension = dot === -1 ? '' : fileName.slice(dot);

  return `${stem.slice(0, stem.lastIndexOf('@'))}${extension}`;
}

/** Group entries by scale, keeping the order scales were first seen in. */
function groupByScale(
  variants: VariantFile[],
  predicate: (variant: VariantFile) => boolean,
): Map<number, VariantFile[]> {
  const groups = new Map<number, VariantFile[]>();

  for (const variant of variants) {
    if (!predicate(variant)) continue;

    const group = groups.get(variant.scale);
    if (group) {
      group.push(variant);
    } else {
      groups.set(variant.scale, [variant]);
    }
  }

  return groups;
}

function count(amount: number, noun: string): string {
  return `${amount} ${noun}${amount === 1 ? '' : 's'}`;
}
