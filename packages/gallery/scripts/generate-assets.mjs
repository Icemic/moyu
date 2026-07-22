// Generates all UI image assets for the gallery as neutral, tintable sprites.
//
// The engine applies tint multiplicatively (texel * tint), so every sprite is
// drawn on a grayscale base: white pixels take the tint color exactly, gray
// pixels become darker shades of the same hue. Pages colorize them via the
// shared theme (`src/theme.ts`).
//
// Usage: node scripts/generate-assets.mjs

import { deflateSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const IMAGES_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'assets', 'images');
const SUPERSAMPLE = 4;

// ---------------------------------------------------------------------------
// PNG encoding (no external dependencies)
// ---------------------------------------------------------------------------

const CRC_TABLE = new Int32Array(256);
for (let n = 0; n < 256; n += 1) {
  let c = n;
  for (let k = 0; k < 8; k += 1) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  CRC_TABLE[n] = c;
}

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data) {
  const chunk = Buffer.alloc(12 + data.length);
  chunk.writeUInt32BE(data.length, 0);
  chunk.write(type, 4, 'ascii');
  data.copy(chunk, 8);
  chunk.writeUInt32BE(crc32(chunk.subarray(4, 8 + data.length)), 8 + data.length);
  return chunk;
}

function encodePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA

  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y += 1) {
    raw[y * (width * 4 + 1)] = 0; // filter: none
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    pngChunk('IHDR', ihdr),
    pngChunk('IDAT', deflateSync(raw, { level: 9 })),
    pngChunk('IEND', Buffer.alloc(0)),
  ]);
}

// ---------------------------------------------------------------------------
// Minimal raster canvas with src-over blending
// ---------------------------------------------------------------------------

function createCanvas(width, height) {
  return { width, height, data: new Uint8ClampedArray(width * height * 4) };
}

function blendPixel(canvas, x, y, r, g, b, a) {
  if (x < 0 || y < 0 || x >= canvas.width || y >= canvas.height || a <= 0) return;
  const i = (y * canvas.width + x) * 4;
  const srcA = a / 255;
  const dstA = canvas.data[i + 3] / 255;
  const outA = srcA + dstA * (1 - srcA);
  if (outA <= 0) return;
  canvas.data[i] = Math.round((r * srcA + canvas.data[i] * dstA * (1 - srcA)) / outA);
  canvas.data[i + 1] = Math.round((g * srcA + canvas.data[i + 1] * dstA * (1 - srcA)) / outA);
  canvas.data[i + 2] = Math.round((b * srcA + canvas.data[i + 2] * dstA * (1 - srcA)) / outA);
  canvas.data[i + 3] = Math.round(outA * 255);
}

function fillRect(canvas, x0, y0, x1, y1, color) {
  const [r, g, b, a] = color;
  for (let y = Math.max(0, Math.floor(y0)); y < Math.min(canvas.height, Math.ceil(y1)); y += 1) {
    for (let x = Math.max(0, Math.floor(x0)); x < Math.min(canvas.width, Math.ceil(x1)); x += 1) {
      const coverX = Math.min(x + 1, x1) - Math.max(x, x0);
      const coverY = Math.min(y + 1, y1) - Math.max(y, y0);
      if (coverX > 0 && coverY > 0) blendPixel(canvas, x, y, r, g, b, a * coverX * coverY);
    }
  }
}

// Signed distance to a rounded rectangle; negative inside.
function roundedRectSdf(px, py, x0, y0, x1, y1, radius) {
  const cx = (x0 + x1) / 2;
  const cy = (y0 + y1) / 2;
  const hw = (x1 - x0) / 2 - radius;
  const hh = (y1 - y0) / 2 - radius;
  const qx = Math.abs(px - cx) - hw;
  const qy = Math.abs(py - cy) - hh;
  const ax = Math.max(qx, 0);
  const ay = Math.max(qy, 0);
  return Math.hypot(ax, ay) + Math.min(Math.max(qx, qy), 0) - radius;
}

function fillRoundRect(canvas, x0, y0, x1, y1, radius, color) {
  const [r, g, b, a] = color;
  for (let y = Math.max(0, Math.floor(y0 - 1)); y < Math.min(canvas.height, Math.ceil(y1 + 1)); y += 1) {
    for (let x = Math.max(0, Math.floor(x0 - 1)); x < Math.min(canvas.width, Math.ceil(x1 + 1)); x += 1) {
      const d = roundedRectSdf(x + 0.5, y + 0.5, x0, y0, x1, y1, radius);
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

function strokeRoundRect(canvas, x0, y0, x1, y1, radius, width, color) {
  const [r, g, b, a] = color;
  for (let y = Math.max(0, Math.floor(y0 - width)); y < Math.min(canvas.height, Math.ceil(y1 + width)); y += 1) {
    for (let x = Math.max(0, Math.floor(x0 - width)); x < Math.min(canvas.width, Math.ceil(x1 + width)); x += 1) {
      const d = Math.abs(roundedRectSdf(x + 0.5, y + 0.5, x0, y0, x1, y1, radius)) - width / 2;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

function fillCircle(canvas, cx, cy, radius, color) {
  const [r, g, b, a] = color;
  for (let y = Math.max(0, Math.floor(cy - radius - 1)); y < Math.min(canvas.height, Math.ceil(cy + radius + 1)); y += 1) {
    for (let x = Math.max(0, Math.floor(cx - radius - 1)); x < Math.min(canvas.width, Math.ceil(cx + radius + 1)); x += 1) {
      const d = Math.hypot(x + 0.5 - cx, y + 0.5 - cy) - radius;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

function strokeCircle(canvas, cx, cy, radius, width, color) {
  const [r, g, b, a] = color;
  for (let y = Math.max(0, Math.floor(cy - radius - width)); y < Math.min(canvas.height, Math.ceil(cy + radius + width)); y += 1) {
    for (let x = Math.max(0, Math.floor(cx - radius - width)); x < Math.min(canvas.width, Math.ceil(cx + radius + width)); x += 1) {
      const d = Math.abs(Math.hypot(x + 0.5 - cx, y + 0.5 - cy) - radius) - width / 2;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

function strokeSegment(canvas, ax, ay, bx, by, width, color) {
  const [r, g, b, a] = color;
  const minX = Math.max(0, Math.floor(Math.min(ax, bx) - width));
  const maxX = Math.min(canvas.width, Math.ceil(Math.max(ax, bx) + width));
  const minY = Math.max(0, Math.floor(Math.min(ay, by) - width));
  const maxY = Math.min(canvas.height, Math.ceil(Math.max(ay, by) + width));
  const dx = bx - ax;
  const dy = by - ay;
  const lengthSq = dx * dx + dy * dy;
  for (let y = minY; y < maxY; y += 1) {
    for (let x = minX; x < maxX; x += 1) {
      const px = x + 0.5;
      const py = y + 0.5;
      const t = lengthSq === 0 ? 0 : Math.min(1, Math.max(0, ((px - ax) * dx + (py - ay) * dy) / lengthSq));
      const d = Math.hypot(px - (ax + t * dx), py - (ay + t * dy)) - width / 2;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

function downsample(canvas, factor) {
  const width = canvas.width / factor;
  const height = canvas.height / factor;
  const out = createCanvas(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      for (let sy = 0; sy < factor; sy += 1) {
        for (let sx = 0; sx < factor; sx += 1) {
          const i = ((y * factor + sy) * canvas.width + (x * factor + sx)) * 4;
          // Weight color by alpha so transparent pixels do not darken edges.
          const alpha = canvas.data[i + 3];
          r += canvas.data[i] * alpha;
          g += canvas.data[i + 1] * alpha;
          b += canvas.data[i + 2] * alpha;
          a += alpha;
        }
      }
      const count = factor * factor;
      const o = (y * width + x) * 4;
      out.data[o] = a === 0 ? 0 : Math.round(r / a);
      out.data[o + 1] = a === 0 ? 0 : Math.round(g / a);
      out.data[o + 2] = a === 0 ? 0 : Math.round(b / a);
      out.data[o + 3] = Math.round(a / count);
    }
  }
  return out;
}

// Control color baked into assets whose kit props cannot take a runtime tint.
const CONTROL_TINT = [0x54, 0x68, 0x8c];

function save(canvas, filename, { supersampled = true, tint } = {}) {
  const final = supersampled ? downsample(canvas, SUPERSAMPLE) : canvas;
  if (tint) {
    for (let i = 0; i < final.data.length; i += 4) {
      final.data[i] = (final.data[i] * tint[0]) / 255;
      final.data[i + 1] = (final.data[i + 1] * tint[1]) / 255;
      final.data[i + 2] = (final.data[i + 2] * tint[2]) / 255;
    }
  }
  writeFileSync(join(IMAGES_DIR, filename), encodePng(final.width, final.height, Buffer.from(final.data.buffer)));
  console.log(`generated images/${filename} (${final.width}x${final.height})`);
}

// ---------------------------------------------------------------------------
// Sprite recipes
// ---------------------------------------------------------------------------

const WHITE = [255, 255, 255, 255];

// Neutral grayscale levels. With a multiplicative tint T: white -> T,
// gray(n) -> T * n / 255 (a darker shade of the same hue).
const GRAY = (n, a = 255) => [n, n, n, a];

/** Chrome used by panels, buttons and chips: fill + border + subtle top highlight. */
function chromeSprite({ size = 64, radius = 8, fill = 128, alpha = 255, borderWidth = 2 }) {
  const s = SUPERSAMPLE;
  const w = size * s;
  const c = createCanvas(w, w);
  const r = radius * s;
  const bw = borderWidth * s;
  fillRoundRect(c, s, s, w - s, w - s, r - s, GRAY(fill, alpha));
  strokeRoundRect(c, s + bw / 2, s + bw / 2, w - s - bw / 2, w - s - bw / 2, r - s - bw / 2, bw, WHITE);
  // Top inner highlight for a hint of depth under any tint.
  fillRoundRect(c, s + bw, s + bw, w - s - bw, s + bw + 2 * s, Math.max(s, r - s - bw), [255, 255, 255, 26]);
  return c;
}

function buttonSprite(fill) {
  return chromeSprite({ size: 64, radius: 6, fill });
}

function checkboxSprite(checked) {
  const c = chromeSprite({ size: 48, radius: 6, fill: 64 });
  if (checked) {
    const s = SUPERSAMPLE;
    const u = (v) => v * s;
    strokeSegment(c, u(12), u(25), u(20), u(33), u(5), WHITE);
    strokeSegment(c, u(20), u(33), u(37), u(14), u(5), WHITE);
  }
  return c;
}

function radioSprite(checked, fill) {
  const s = SUPERSAMPLE;
  const size = 48 * s;
  const c = createCanvas(size, size);
  fillCircle(c, size / 2, size / 2, 20 * s, GRAY(fill));
  strokeCircle(c, size / 2, size / 2, 18.5 * s, 3 * s, WHITE);
  if (checked) {
    fillCircle(c, size / 2, size / 2, 9 * s, WHITE);
  }
  return c;
}

function dropdownTriggerSprite(fill) {
  const c = chromeSprite({ size: 64, radius: 6, fill });
  const s = SUPERSAMPLE;
  const u = (v) => v * s;
  // Chevron sits inside the right unstretched nine-slice zone (x >= 48).
  strokeSegment(c, u(42), u(28), u(47), u(35), u(4), WHITE);
  strokeSegment(c, u(47), u(35), u(52), u(28), u(4), WHITE);
  return c;
}

function listItemSprite(fill) {
  const s = SUPERSAMPLE;
  const w = 64 * s;
  const c = createCanvas(w, w);
  fillRoundRect(c, 2 * s, 2 * s, w - 2 * s, w - 2 * s, 4 * s, GRAY(fill));
  return c;
}

function sliderTrackSprite(fill) {
  const s = SUPERSAMPLE;
  const w = 64 * s;
  const h = 40 * s;
  const c = createCanvas(w, h);
  fillRoundRect(c, 0, h / 2 - 6 * s, w, h / 2 + 6 * s, 6 * s, GRAY(fill));
  strokeRoundRect(c, s, h / 2 - 5 * s, w - s, h / 2 + 5 * s, 5 * s, 2 * s, WHITE);
  return c;
}

function sliderHandleSprite(fill) {
  const s = SUPERSAMPLE;
  const w = 40 * s;
  const c = createCanvas(w, w);
  fillCircle(c, w / 2, w / 2, 15 * s, GRAY(fill));
  strokeCircle(c, w / 2, w / 2, 14 * s, 3 * s, WHITE);
  return c;
}

function sampleImage() {
  // Colorful test image for sprite/area/filter demos: saturated quadrants,
  // a centered disc, and a hue gradient strip for hue-sensitive filters.
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

function hueToRgb(t) {
  // HSV with s = 0.72, v = 0.95.
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

function backgroundImage() {
  // 1920x1080 deep-navy vertical gradient with vignette and a faint top glow.
  const w = 1920;
  const h = 1080;
  const c = createCanvas(w, h);
  for (let y = 0; y < h; y += 1) {
    const t = y / (h - 1);
    const r = 13 + (7 - 13) * t;
    const g = 18 + (10 - 18) * t;
    const b = 30 + (15 - 30) * t;
    for (let x = 0; x < w; x += 1) {
      const i = (y * w + x) * 4;
      c.data[i] = r;
      c.data[i + 1] = g;
      c.data[i + 2] = b;
      c.data[i + 3] = 255;
    }
  }
  // Faint cool glow near the top center.
  for (let y = 0; y < h; y += 1) {
    for (let x = 0; x < w; x += 1) {
      const nx = (x / w - 0.5) * 2;
      const ny = (y / h - 0.05) * 2;
      const d = Math.hypot(nx, ny * 1.6);
      const glow = Math.max(0, 1 - d) ** 2 * 0.055;
      if (glow > 0.001) blendPixel(c, x, y, 96, 116, 176, glow * 255);
    }
  }
  // Vignette.
  for (let y = 0; y < h; y += 1) {
    for (let x = 0; x < w; x += 1) {
      const nx = (x / w - 0.5) * 2;
      const ny = (y / h - 0.5) * 2;
      const d = Math.hypot(nx, ny) / Math.SQRT2;
      const dark = Math.min(0.22, Math.max(0, (d - 0.55)) * 0.45);
      if (dark > 0) blendPixel(c, x, y, 0, 0, 0, dark * 255);
    }
  }
  return c;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

mkdirSync(IMAGES_DIR, { recursive: true });

// Background (drawn at native size, no supersampling).
{
  const bg = backgroundImage();
  writeFileSync(join(IMAGES_DIR, 'bg.png'), encodePng(bg.width, bg.height, Buffer.from(bg.data.buffer)));
  console.log('generated images/bg.png (1920x1080)');
}

save(chromeSprite({ size: 64, radius: 8, fill: 128, alpha: 250 }), 'panel.png');
save(chromeSprite({ size: 48, radius: 6, fill: 64 }), 'chip.png');

save(buttonSprite(128), 'button.png');
save(buttonSprite(166), 'button_hover.png');
save(buttonSprite(102), 'button_press.png');

save(checkboxSprite(false), 'unchecked.png');
save(checkboxSprite(true), 'checked.png');
// Hover/press reuse brightness variants of the box.
save(chromeSprite({ size: 48, radius: 6, fill: 96 }), 'unchecked_hover.png');
save(chromeSprite({ size: 48, radius: 6, fill: 48 }), 'unchecked_press.png');
{
  const hover = chromeSprite({ size: 48, radius: 6, fill: 96 });
  const s = SUPERSAMPLE;
  const u = (v) => v * s;
  strokeSegment(hover, u(12), u(25), u(20), u(33), u(5), WHITE);
  strokeSegment(hover, u(20), u(33), u(37), u(14), u(5), WHITE);
  save(hover, 'checked_hover.png');
}
{
  const press = chromeSprite({ size: 48, radius: 6, fill: 48 });
  const s = SUPERSAMPLE;
  const u = (v) => v * s;
  strokeSegment(press, u(12), u(25), u(20), u(33), u(5), WHITE);
  strokeSegment(press, u(20), u(33), u(37), u(14), u(5), WHITE);
  save(press, 'checked_press.png');
}

save(radioSprite(false, 64), 'radio_unchecked.png');
save(radioSprite(false, 96), 'radio_unchecked_hover.png');
save(radioSprite(false, 48), 'radio_unchecked_press.png');
save(radioSprite(true, 64), 'radio_checked.png');
save(radioSprite(true, 96), 'radio_checked_hover.png');
save(radioSprite(true, 48), 'radio_checked_press.png');

save(dropdownTriggerSprite(128), 'dropdown.png');
save(dropdownTriggerSprite(166), 'dropdown_hover.png');
save(dropdownTriggerSprite(102), 'dropdown_press.png');
save(chromeSprite({ size: 64, radius: 8, fill: 140 }), 'dropdown_list.png', { tint: CONTROL_TINT });
save(listItemSprite(128), 'dropdown_listitem.png');
save(listItemSprite(180), 'dropdown_listitem_hover.png');
save(listItemSprite(104), 'dropdown_listitem_press.png');

save(sliderTrackSprite(96), 'slider_track.png');
save(sliderTrackSprite(128), 'slider_track_hover.png');
save(sliderTrackSprite(72), 'slider_track_press.png');
save(sliderHandleSprite(210), 'slider_handle.png');
save(sliderHandleSprite(235), 'slider_handle_hover.png');
save(sliderHandleSprite(170), 'slider_handle_press.png');

save(sampleImage(), 'sample.png', { supersampled: false });

// 8x8 pure white pixel: tint and stretch for divider lines and accent bars.
{
  const c = createCanvas(8, 8);
  fillRect(c, 0, 0, 8, 8, WHITE);
  save(c, 'pixel.png', { supersampled: false });
}

console.log('done');
