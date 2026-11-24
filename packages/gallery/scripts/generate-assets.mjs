// Generates every gallery sprite, for both appearance modes, from `src/theme/tokens.json`.
//
// Sprites are baked with their final colours rather than tinted at runtime: a card needs
// a border lighter than its fill in dark mode and darker in light mode, which a single
// neutral texture multiplied by one tint cannot express. Baking from the same token file
// the runtime reads keeps the two in sync.
//
// Only two assets stay neutral and tintable at runtime:
//   images/pixel.png — 1x1 white, stretched for dividers, bars and solid fills
//   images/chip.png  — flat rounded chip, tinted with an arbitrary demo colour
//
// Usage: node scripts/generate-assets.mjs

import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  blendPixel,
  createCanvas,
  encodePng,
  fillCircle,
  fillRect,
  fillRoundRect,
  savePng,
  strokeCircle,
  strokeRoundRect,
  strokeSegment,
} from './lib/raster.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ASSETS_DIR = join(HERE, '..', 'assets');
const IMAGES_DIR = join(ASSETS_DIR, 'images');
const TOKENS = JSON.parse(readFileSync(join(HERE, '..', 'src', 'theme', 'tokens.json'), 'utf8'));
const SS = 4;

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

function rgb(hex) {
  const h = hex.replace('#', '');
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

function rgba(hex, alpha = 255) {
  return [...rgb(hex), alpha];
}

// ---------------------------------------------------------------------------
// Sprite recipes
// ---------------------------------------------------------------------------

/**
 * Nine-slice chrome: filled rounded rect with an optional 1px hairline, drawn one pixel
 * in so the stroke is not clipped by the canvas edge.
 */
function chromeSprite({ size, radius, fill, border = null, borderWidth = 2 }) {
  const s = SS;
  const c = createCanvas(size * s, size * s);
  const inset = s;
  fillRoundRect(c, inset, inset, size * s - inset, size * s - inset, radius * s, rgba(fill));
  if (border) {
    const bw = borderWidth * s;
    strokeRoundRect(
      c,
      inset + bw / 2,
      inset + bw / 2,
      size * s - inset - bw / 2,
      size * s - inset - bw / 2,
      Math.max(0, radius * s - bw / 2),
      bw,
      rgba(border),
    );
  }
  return c;
}

function panelSprite(palette) {
  return chromeSprite({ size: 64, radius: TOKENS.radius.panel, fill: palette.surface });
}

function buttonSprite({ fill }) {
  return chromeSprite({ size: 64, radius: TOKENS.radius.control, fill });
}

function checkboxSprite(palette, checked) {
  const size = 48;
  const s = SS;
  const c = createCanvas(size * s, size * s);
  const inset = 2 * s;
  const radius = TOKENS.radius.small * s;
  const fill = checked ? palette.accent : palette.controlIdle;
  const border = checked ? palette.accent : palette.controlBorder;

  fillRoundRect(c, inset, inset, size * s - inset, size * s - inset, radius, rgba(fill));
  if (!checked) {
    strokeRoundRect(
      c,
      inset + 0.75 * s,
      inset + 0.75 * s,
      size * s - inset - 0.75 * s,
      size * s - inset - 0.75 * s,
      Math.max(0, radius - 0.5 * s),
      1.5 * s,
      rgba(border),
    );
  } else {
    // Tick drawn in the accent's foreground colour, so it stays legible in both modes.
    const u = (v) => (v / 48) * size * s;
    strokeSegment(c, u(13), u(25), u(20.5), u(32.5), u(4.6), rgba(palette.onAccent));
    strokeSegment(c, u(20.5), u(32.5), u(35), u(16), u(4.6), rgba(palette.onAccent));
  }
  return c;
}

function radioSprite(palette, checked) {
  const size = 48;
  const s = SS;
  const mid = (size / 2) * s;
  const c = createCanvas(size * s, size * s);

  fillCircle(c, mid, mid, 20 * s, rgba(checked ? palette.accent : palette.controlIdle));
  if (!checked) {
    strokeCircle(c, mid, mid, 19.2 * s, 1.5 * s, rgba(palette.controlBorder));
  } else {
    fillCircle(c, mid, mid, 8.5 * s, rgba(palette.onAccent));
  }
  return c;
}

function selectSprite(palette, state) {
  const fill = state === 'idle' ? palette.controlIdle : state === 'hover' ? palette.controlHover : palette.controlPress;
  const c = chromeSprite({ size: 64, radius: TOKENS.radius.control, fill });
  const s = SS;
  const u = (v) => v * s;
  // The chevron sits inside the right slice, which never stretches.
  strokeSegment(c, u(43), u(28), u(48), u(34), u(2.2), rgba(palette.textSecondary));
  strokeSegment(c, u(48), u(34), u(53), u(28), u(2.2), rgba(palette.textSecondary));
  return c;
}

function listItemSprite(palette, state) {
  const fill = state === 'idle' ? palette.surface : palette.controlHover;
  return chromeSprite({ size: 64, radius: TOKENS.radius.small, fill });
}

function trackSprite(palette, state) {
  const s = SS;
  const w = 64 * s;
  const h = 40 * s;
  const c = createCanvas(w, h);
  const fill = state === 'idle' ? palette.track : state === 'hover' ? palette.controlHover : palette.controlPress;
  fillRoundRect(c, 0, h / 2 - 6 * s, w, h / 2 + 6 * s, 6 * s, rgba(fill));
  return c;
}

function thumbSprite(palette, state) {
  const size = 40;
  const s = SS;
  const mid = (size / 2) * s;
  const c = createCanvas(size * s, size * s);
  const fill = state === 'press' ? palette.accent : palette.surface;
  fillCircle(c, mid, mid, 14 * s, rgba(fill));
  strokeCircle(c, mid, mid, 13.4 * s, 1.6 * s, rgba(palette.accent));
  return c;
}

function inputSprite(palette, state) {
  const map = {
    idle: { fill: palette.controlIdle, border: null },
    hover: { fill: palette.controlHover, border: null },
    press: { fill: palette.controlPress, border: null },
    focused: { fill: palette.accentSoft, border: null },
    readonly: { fill: palette.surfaceMuted, border: null },
    disabled: { fill: palette.surfaceMuted, border: null },
  }[state];

  return chromeSprite({ size: 64, radius: TOKENS.radius.control, fill: map.fill, border: map.border });
}

// ---------------------------------------------------------------------------
// Page background: off-white or near-black with a 32px / 160px grid
// ---------------------------------------------------------------------------

function makeNoise(seed) {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    state >>>= 0;
    return state / 0xffffffff;
  };
}

function backgroundSprite(palette) {
  const width = 1920;
  const height = 1080;
  const c = createCanvas(width, height);
  const [r, g, b] = rgb(palette.background);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const i = (y * width + x) * 4;
      c.data[i] = r;
      c.data[i + 1] = g;
      c.data[i + 2] = b;
      c.data[i + 3] = 255;
    }
  }

  // Grid tinted with the mode's ink; alpha comes from the tokens.
  const line = rgb(palette.gridLine);
  for (let x = 0; x <= width; x += 32) {
    fillRect(c, x, 0, x + 1, height, [...line, palette.gridAlpha]);
  }
  for (let y = 0; y <= height; y += 32) {
    fillRect(c, 0, y, width, y + 1, [...line, palette.gridAlpha]);
  }
  for (let x = 0; x <= width; x += 160) {
    fillRect(c, x, 0, x + 1, height, [...line, palette.gridMajorAlpha]);
  }
  for (let y = 0; y <= height; y += 160) {
    fillRect(c, 0, y, width, y + 1, [...line, palette.gridMajorAlpha]);
  }

  // Fine grain so large flat areas do not band.
  if (palette.noiseAlpha > 0) {
    const random = makeNoise(0x9e3779b9);
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const v = random();
        const light = v >= 0.5;
        blendPixel(
          c,
          x,
          y,
          light ? 255 : 0,
          light ? 255 : 0,
          light ? 255 : 0,
          palette.noiseAlpha * Math.abs(v - 0.5) * 2,
        );
      }
    }
  }

  return c;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

mkdirSync(IMAGES_DIR, { recursive: true });

for (const [mode, palette] of Object.entries(TOKENS.modes)) {
  const dir = join(IMAGES_DIR, mode);
  mkdirSync(dir, { recursive: true });

  const save = (canvas, filename) => {
    savePng(canvas, join(dir, filename), { supersample: SS });
    return filename;
  };

  const written = [];

  written.push(save(panelSprite(palette), 'panel.png'));
  written.push(save(buttonSprite({ fill: palette.controlIdle }), 'button_idle.png'));
  written.push(save(buttonSprite({ fill: palette.controlHover }), 'button_hover.png'));
  written.push(save(buttonSprite({ fill: palette.controlPress }), 'button_press.png'));
  // Tabs rest on the page, so they take the surface colour rather than a control fill.
  written.push(save(buttonSprite({ fill: palette.surface }), 'tab_idle.png'));
  written.push(save(buttonSprite({ fill: palette.surfaceMuted }), 'tab_hover.png'));
  written.push(save(buttonSprite({ fill: palette.surfaceMuted }), 'tab_press.png'));
  written.push(save(buttonSprite({ fill: palette.accentSoft }), 'button_selected.png'));
  written.push(save(buttonSprite({ fill: palette.accentSoft }), 'button_selected_hover.png'));
  written.push(save(buttonSprite({ fill: palette.accent }), 'primary_idle.png'));
  written.push(save(buttonSprite({ fill: palette.accentHover }), 'primary_hover.png'));
  written.push(save(buttonSprite({ fill: palette.accentPress }), 'primary_press.png'));

  written.push(save(checkboxSprite(palette, false), 'checkbox.png'));
  written.push(save(checkboxSprite(palette, false), 'checkbox_hover.png'));
  written.push(save(checkboxSprite(palette, true), 'checkbox_checked.png'));
  written.push(save(checkboxSprite(palette, true), 'checkbox_checked_hover.png'));
  written.push(save(radioSprite(palette, false), 'radio.png'));
  written.push(save(radioSprite(palette, false), 'radio_hover.png'));
  written.push(save(radioSprite(palette, true), 'radio_checked.png'));
  written.push(save(radioSprite(palette, true), 'radio_checked_hover.png'));

  for (const state of ['idle', 'hover', 'press']) {
    written.push(save(selectSprite(palette, state), `select_${state}.png`));
    written.push(save(listItemSprite(palette, state), `select_item_${state}.png`));
    written.push(save(trackSprite(palette, state), `track_${state}.png`));
    written.push(save(thumbSprite(palette, state), `thumb_${state}.png`));
  }
  written.push(
    save(chromeSprite({ size: 64, radius: TOKENS.radius.panel, fill: palette.controlIdle }), 'select_list.png'),
  );

  for (const state of ['idle', 'hover', 'press', 'focused', 'readonly', 'disabled']) {
    written.push(save(inputSprite(palette, state), `input_${state}.png`));
  }

  // Background is drawn at native size, no supersampling.
  savePng(backgroundSprite(palette), join(dir, 'bg.png'), { supersampled: false });

  console.log(`generated images/${mode}/ (${String(written.length)} sprites + bg)`);
}

// --- neutral, runtime-tinted assets ------------------------------------------------

{
  const c = createCanvas(8, 8);
  fillRect(c, 0, 0, 8, 8, [255, 255, 255, 255]);
  savePng(c, join(IMAGES_DIR, 'pixel.png'), { supersampled: false });

  savePng(chromeSprite({ size: 64, radius: TOKENS.radius.chip, fill: '#FFFFFF' }), join(IMAGES_DIR, 'chip.png'), {
    supersample: SS,
  });
  console.log('generated images/pixel.png, images/chip.png (neutral, tinted at runtime)');
}

// --- demo content images (mode independent) ---------------------------------------
//
// Neutral where a page tints them at runtime: `ring.png` is the alpha-hit-test target.
// It is deliberately opaque inside the ring, so only the four corners fall through to
// the plate underneath it.

{
  const size = 48;
  const s = SS;
  const mid = (size / 2) * s;
  const c = createCanvas(size * s, size * s);
  fillCircle(c, mid, mid, 20 * s, [64, 64, 64, 255]);
  strokeCircle(c, mid, mid, 18.5 * s, 3 * s, [255, 255, 255, 255]);
  savePng(c, join(IMAGES_DIR, 'ring.png'), { supersample: SS });
}

function sampleImage() {
  // Colourful test image for sprite/area/filter demos: saturated quadrants, a centred
  // disc, and a hue strip so hue-sensitive filters have something to shift.
  const w = 250;
  const h = 240;
  const c = createCanvas(w, h);
  fillRect(c, 0, 0, w / 2, h / 2, [59, 130, 196, 255]);
  fillRect(c, w / 2, 0, w, h / 2, [76, 166, 106, 255]);
  fillRect(c, 0, h / 2, w / 2, h, [196, 90, 90, 255]);
  fillRect(c, w / 2, h / 2, w, h, [138, 95, 192, 255]);
  fillCircle(c, w / 2, h / 2, 44, [245, 245, 245, 255]);
  strokeCircle(c, w / 2, h / 2, 44, 6, [20, 24, 32, 255]);
  for (let x = 0; x < w; x += 1) {
    const t = x / w;
    const [r, g, b] = hueToRgb(t);
    fillRect(c, x, h - 26, x + 1, h, [r, g, b, 255]);
  }
  strokeRoundRect(c, 1, 1, w - 1, h - 1, 0, 3, [255, 255, 255, 200]);
  return c;
}

function ninesliceModeSample() {
  // Corner blocks, striped edges and a dotted centre make stretch, repeat, mirror and
  // blank tell apart at a glance; the rounded border shows corners never distort.
  const s = SS;
  const u = (v) => v * s;
  const c = createCanvas(u(64), u(64));
  const GRAY = (n) => [n, n, n, 255];

  fillRect(c, 0, 0, u(64), u(64), GRAY(64));
  for (const [x, y] of [
    [5, 5],
    [49, 5],
    [5, 49],
    [49, 49],
  ]) {
    fillRect(c, u(x), u(y), u(x + 10), u(y + 10), GRAY(208));
  }
  for (let x = 16; x < 48; x += 8) {
    fillRect(c, u(x), u(4), u(x + 4), u(14), GRAY(200));
    fillRect(c, u(x), u(50), u(x + 4), u(60), GRAY(200));
  }
  for (let y = 16; y < 48; y += 8) {
    fillRect(c, u(4), u(y), u(14), u(y + 4), GRAY(200));
    fillRect(c, u(50), u(y), u(60), u(y + 4), GRAY(200));
  }
  fillRect(c, u(20), u(18), u(24), u(46), GRAY(210));
  fillRect(c, u(20), u(18), u(44), u(22), GRAY(210));
  fillRect(c, u(20), u(29), u(36), u(33), GRAY(210));
  strokeRoundRect(c, u(1), u(1), u(63), u(63), u(6), u(2), [255, 255, 255, 255]);
  return c;
}

function hueToRgb(t) {
  const h = (t % 1) * 6;
  const c = 0.95 * 0.72;
  const x = c * (1 - Math.abs((h % 2) - 1));
  const m = 0.95 - c;
  const sector = Math.floor(h) % 6;
  const [r, g, b] = [
    [c, x, 0],
    [x, c, 0],
    [0, c, x],
    [0, x, c],
    [x, 0, c],
    [c, 0, x],
  ][sector];
  return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}

savePng(sampleImage(), join(IMAGES_DIR, 'sample.png'), { supersampled: false });
savePng(ninesliceModeSample(), join(IMAGES_DIR, 'nineslice_modes.png'), { supersample: SS });

// --- cursors ----------------------------------------------------------------------
//
// Handed straight to the windowing library, so they are drawn at their final size.

const CURSOR_INK = [13, 19, 34, 255];
const CURSOR_GOLD = [245, 197, 66, 255];
const CURSOR_CYAN = [79, 209, 197, 255];
const CURSOR_GREEN = [110, 231, 168, 255];
const CURSOR_PINK = [244, 114, 182, 255];
const CURSOR_ORANGE = [251, 146, 60, 255];

function fillPolygon(canvas, points, color) {
  const [r, g, b, a] = color;
  let minY = Infinity;
  let maxY = -Infinity;
  for (const [, y] of points) {
    minY = Math.min(minY, y);
    maxY = Math.max(maxY, y);
  }
  for (let y = Math.max(0, Math.floor(minY)); y < Math.min(canvas.height, Math.ceil(maxY)); y += 1) {
    const row = y + 0.5;
    const crossings = [];
    for (let i = 0; i < points.length; i += 1) {
      const [x0, y0] = points[i];
      const [x1, y1] = points[(i + 1) % points.length];
      if (y0 === y1) continue;
      if (row >= Math.min(y0, y1) && row < Math.max(y0, y1)) {
        crossings.push(x0 + ((row - y0) / (y1 - y0)) * (x1 - x0));
      }
    }
    crossings.sort((p, q) => p - q);
    for (let i = 0; i + 1 < crossings.length; i += 2) {
      fillRect(canvas, crossings[i], y, crossings[i + 1], y + 1, [r, g, b, a]);
    }
  }
}

function strokePolygon(canvas, points, width, color) {
  for (let i = 0; i < points.length; i += 1) {
    const [x0, y0] = points[i];
    const [x1, y1] = points[(i + 1) % points.length];
    strokeSegment(canvas, x0, y0, x1, y1, width, color);
  }
}

function cursorArrow() {
  const c = createCanvas(32, 32);
  const points = [
    [3, 2],
    [3, 25],
    [10, 19],
    [14, 30],
    [20, 27],
    [16, 17],
    [26, 17],
  ];
  fillPolygon(c, points, CURSOR_GOLD);
  strokePolygon(c, points, 2.4, CURSOR_INK);
  return c;
}

function cursorHand() {
  const c = createCanvas(32, 32);
  const parts = [
    [13, 3, 19, 26, 3],
    [18, 8, 24, 26, 2.5],
    [7, 13, 13, 26, 2.5],
    [2, 16, 8, 23, 3],
    [5, 20, 26, 30, 5],
  ];
  for (const [x0, y0, x1, y1, radius] of parts) {
    fillRoundRect(c, x0, y0, x1, y1, radius, CURSOR_CYAN);
  }
  for (const [x0, y0, x1, y1, radius] of parts) {
    strokeRoundRect(c, x0, y0, x1, y1, radius, 1.6, CURSOR_INK);
  }
  return c;
}

function cursorIBeam() {
  const c = createCanvas(32, 32);
  fillRect(c, 13, 4, 19, 28, CURSOR_GREEN);
  fillRect(c, 6, 4, 26, 8, CURSOR_GREEN);
  fillRect(c, 6, 24, 26, 28, CURSOR_GREEN);
  strokeRoundRect(c, 12, 3, 20, 29, 1, 2, CURSOR_INK);
  strokeRoundRect(c, 6, 4, 26, 8, 1, 2, CURSOR_INK);
  strokeRoundRect(c, 6, 24, 26, 28, 1, 2, CURSOR_INK);
  fillRect(c, 14, 4, 18, 28, CURSOR_GREEN);
  fillRect(c, 7, 5, 25, 7, CURSOR_GREEN);
  fillRect(c, 7, 25, 25, 27, CURSOR_GREEN);
  return c;
}

function cursorCrosshair() {
  const c = createCanvas(32, 32);
  const mid = 16;
  const draw = (width, color) => {
    strokeCircle(c, mid, mid, 7, width, color);
    strokeSegment(c, mid, 1, mid, 8, width, color);
    strokeSegment(c, mid, 24, mid, 31, width, color);
    strokeSegment(c, 1, mid, 8, mid, width, color);
    strokeSegment(c, 24, mid, 31, mid, width, color);
  };
  // Outline first, then the visible stroke again on top, so the dark line only
  // widens the silhouette.
  draw(2.6, CURSOR_PINK);
  draw(4.4, CURSOR_INK);
  draw(2.6, CURSOR_PINK);
  return c;
}

function cursorMove() {
  const c = createCanvas(32, 32);
  const mid = 16;
  const heads = [
    [
      [mid, 1],
      [10, 10],
      [22, 10],
    ],
    [
      [mid, 31],
      [10, 22],
      [22, 22],
    ],
    [
      [1, mid],
      [10, 10],
      [10, 22],
    ],
    [
      [31, mid],
      [22, 10],
      [22, 22],
    ],
  ];
  fillRect(c, 14, 6, 18, 26, CURSOR_ORANGE);
  fillRect(c, 6, 14, 26, 18, CURSOR_ORANGE);
  for (const head of heads) fillPolygon(c, head, CURSOR_ORANGE);

  strokeRoundRect(c, 14, 6, 18, 26, 1, 1.8, CURSOR_INK);
  strokeRoundRect(c, 6, 14, 26, 18, 1, 1.8, CURSOR_INK);
  for (const head of heads) strokePolygon(c, head, 1.8, CURSOR_INK);

  fillRect(c, 14.9, 7, 17.1, 25, CURSOR_ORANGE);
  fillRect(c, 7, 14.9, 25, 17.1, CURSOR_ORANGE);
  for (const head of heads) {
    const [[ax, ay], [bx, by], [cx, cy]] = head;
    fillPolygon(
      c,
      [
        [ax + (bx - ax) * 0.22, ay + (by - ay) * 0.22],
        [bx + (cx - bx) * 0.12, by + (cy - by) * 0.12],
        [cx + (bx - cx) * 0.12, cy + (by - cy) * 0.12],
      ],
      CURSOR_ORANGE,
    );
  }
  return c;
}

{
  const dir = join(ASSETS_DIR, 'cursors');
  mkdirSync(dir, { recursive: true });
  const cursors = {
    'arrow.png': cursorArrow(),
    'hand.png': cursorHand(),
    'ibeam.png': cursorIBeam(),
    'crosshair.png': cursorCrosshair(),
    'move.png': cursorMove(),
  };
  for (const [filename, canvas] of Object.entries(cursors)) {
    writeFileSync(join(dir, filename), encodePng(canvas.width, canvas.height, Buffer.from(canvas.data.buffer)));
  }
  console.log(`generated cursors/ (${String(Object.keys(cursors).length)} files)`);
}

// --- clean up sprites from the previous, single-mode theme -------------------------
//
// They are no longer referenced: every control sprite now lives under images/<mode>/.
for (const stale of [
  'panel.png',
  'button.png',
  'button_hover.png',
  'button_press.png',
  'checked.png',
  'checked_hover.png',
  'checked_press.png',
  'unchecked.png',
  'unchecked_hover.png',
  'unchecked_press.png',
  'radio_checked.png',
  'radio_checked_hover.png',
  'radio_checked_press.png',
  'radio_unchecked.png',
  'radio_unchecked_hover.png',
  'radio_unchecked_press.png',
  'dropdown.png',
  'dropdown_hover.png',
  'dropdown_press.png',
  'dropdown_list.png',
  'dropdown_listitem.png',
  'dropdown_listitem_hover.png',
  'dropdown_listitem_press.png',
  'scrollbar.png',
  'slider_handle.png',
  'slider_handle_hover.png',
  'slider_handle_press.png',
  'slider_track.png',
  'slider_track_hover.png',
  'slider_track_press.png',
  'bg.png',
]) {
  rmSync(join(IMAGES_DIR, stale), { force: true });
}

console.log('done');
