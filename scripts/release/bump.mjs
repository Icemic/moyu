#!/usr/bin/env node

/**
 * Local version bump helper for independent monorepo release tracks.
 *
 * Usage:
 *   node scripts/release/bump.mjs patch             # defaults to moyu engine
 *   node scripts/release/bump.mjs 0.23.0            # defaults to moyu engine
 *   node scripts/release/bump.mjs moyu minor
 *   node scripts/release/bump.mjs devtool patch
 *   node scripts/release/bump.mjs kit 0.14.2
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { createInterface } from 'node:readline/promises';
import { fileURLToPath } from 'node:url';

const SEMVER_RE = /^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;
const VALID_TARGETS = ['moyu', 'devtool', 'kit'];
const BUMP_KINDS = ['major', 'minor', 'patch'];

const rawArgs = process.argv.slice(2);
const autoConfirm = rawArgs.includes('-y') || rawArgs.includes('--yes');
const args = rawArgs.filter((a) => a !== '-y' && a !== '--yes');

let target;
let versionSpec;

if (args.length === 1) {
  target = 'moyu';
  versionSpec = args[0];
} else if (args.length === 2) {
  [target, versionSpec] = args;
} else {
  console.error('Usage: node scripts/release/bump.mjs [moyu|devtool|kit] <major|minor|patch|semver> [-y]');
  process.exit(1);
}

if (!VALID_TARGETS.includes(target)) {
  console.error(`Unknown target "${target}". Valid targets: ${VALID_TARGETS.join(', ')}`);
  process.exit(1);
}

if (!BUMP_KINDS.includes(versionSpec) && !SEMVER_RE.test(versionSpec)) {
  console.error(`Invalid version or bump kind: "${versionSpec}". Expected major, minor, patch, or semver.`);
  process.exit(1);
}

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..', '..');

function readCargoVersion(relPath) {
  const content = readFileSync(join(repoRoot, relPath), 'utf-8');
  const match = content.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`Could not find \`version = "..."\` in ${relPath}`);
  }
  return match[1];
}

function readPackageVersion(relPath) {
  const pkg = JSON.parse(readFileSync(join(repoRoot, relPath), 'utf-8'));
  return pkg.version;
}

function computeNextVersion(current, spec) {
  if (!BUMP_KINDS.includes(spec)) {
    return spec;
  }
  const match = current.match(SEMVER_RE);
  if (!match) {
    throw new Error(`Current version "${current}" is not valid semver`);
  }
  const major = Number(match[1]);
  const minor = Number(match[2]);
  const patch = Number(match[3]);
  if (spec === 'major') return `${major + 1}.0.0`;
  if (spec === 'minor') return `${major}.${minor + 1}.0`;
  return `${major}.${minor}.${patch + 1}`;
}

function updatePackageJson(relPath, nextVersion) {
  const fullPath = join(repoRoot, relPath);
  const pkg = JSON.parse(readFileSync(fullPath, 'utf-8'));
  pkg.version = nextVersion;
  writeFileSync(fullPath, `${JSON.stringify(pkg, null, 2)}\n`);
}

function updateCargoToml(relPath, nextVersion) {
  const fullPath = join(repoRoot, relPath);
  const content = readFileSync(fullPath, 'utf-8');
  const updated = content.replace(/^version\s*=\s*"[^"]+"/m, `version = "${nextVersion}"`);
  writeFileSync(fullPath, updated);
}

const plan = [];
let nextVersion;

if (target === 'moyu') {
  const cur = readCargoVersion('Cargo.toml');
  nextVersion = computeNextVersion(cur, versionSpec);
  plan.push({ type: 'cargo', path: 'Cargo.toml', prev: cur, next: nextVersion });
} else if (target === 'devtool') {
  const curCargo = readCargoVersion('crates/devtool/Cargo.toml');
  const curPkg = readPackageVersion('packages/cli/package.json');
  nextVersion = computeNextVersion(curCargo, versionSpec);
  plan.push({ type: 'cargo', path: 'crates/devtool/Cargo.toml', prev: curCargo, next: nextVersion });
  plan.push({ type: 'pkg', path: 'packages/cli/package.json', prev: curPkg, next: nextVersion });
} else {
  const cur = readPackageVersion('packages/kit/package.json');
  nextVersion = computeNextVersion(cur, versionSpec);
  plan.push({ type: 'pkg', path: 'packages/kit/package.json', prev: cur, next: nextVersion });
}

console.log(`Target: ${target}`);
for (const item of plan) {
  console.log(`  ${item.path}: ${item.prev} -> ${item.next}`);
}

if (!autoConfirm) {
  if (!process.stdin.isTTY) {
    console.error('Confirmation required. Re-run with -y in non-interactive mode.');
    process.exit(1);
  }
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  const answer = (await rl.question('\nApply version bump? (y/N): ')).trim().toLowerCase();
  rl.close();
  if (answer !== 'y' && answer !== 'yes') {
    console.log('Cancelled.');
    process.exit(0);
  }
}

for (const item of plan) {
  if (item.type === 'cargo') updateCargoToml(item.path, item.next);
  else updatePackageJson(item.path, item.next);
}

console.log('\nUpdated successfully. Next steps:');
if (target === 'moyu') {
  console.log(`  git commit -am "chore: release v${nextVersion}"`);
  console.log(`  git tag v${nextVersion}`);
} else if (target === 'devtool') {
  console.log(`  git commit -am "chore(devtool): release v${nextVersion}"`);
  console.log(`  git tag devtool-v${nextVersion}`);
} else {
  console.log(`  git commit -am "chore(kit): release v${nextVersion}"`);
  console.log(`  git tag kit-v${nextVersion}`);
}
console.log('  git push --follow-tags');
