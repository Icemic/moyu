// Zero-dependency raster helpers shared by the gallery asset generators.
//
// The engine multiplies texel color with `tint`, so every UI sprite is drawn on
// a grayscale or neutral base and colorized by the theme at runtime. These
// helpers cover the small subset of 2D drawing the gallery assets need.

import { writeFileSync } from 'node:fs';
import { deflateSync } from 'node:zlib';

// ---------------------------------------------------------------------------
// PNG encoding
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

export function encodePng(width, height, rgba) {
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

export function createCanvas(width, height) {
  return { width, height, data: new Uint8ClampedArray(width * height * 4) };
}

export function blendPixel(canvas, x, y, r, g, b, a) {
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

export function fillRect(canvas, x0, y0, x1, y1, color) {
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
export function roundedRectSdf(px, py, x0, y0, x1, y1, radius) {
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

export function fillRoundRect(canvas, x0, y0, x1, y1, radius, color) {
  const [r, g, b, a] = color;
  for (let y = Math.max(0, Math.floor(y0 - 1)); y < Math.min(canvas.height, Math.ceil(y1 + 1)); y += 1) {
    for (let x = Math.max(0, Math.floor(x0 - 1)); x < Math.min(canvas.width, Math.ceil(x1 + 1)); x += 1) {
      const d = roundedRectSdf(x + 0.5, y + 0.5, x0, y0, x1, y1, radius);
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

export function strokeRoundRect(canvas, x0, y0, x1, y1, radius, width, color) {
  const [r, g, b, a] = color;
  for (let y = Math.max(0, Math.floor(y0 - width)); y < Math.min(canvas.height, Math.ceil(y1 + width)); y += 1) {
    for (let x = Math.max(0, Math.floor(x0 - width)); x < Math.min(canvas.width, Math.ceil(x1 + width)); x += 1) {
      const d = Math.abs(roundedRectSdf(x + 0.5, y + 0.5, x0, y0, x1, y1, radius)) - width / 2;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

export function fillCircle(canvas, cx, cy, radius, color) {
  const [r, g, b, a] = color;
  for (
    let y = Math.max(0, Math.floor(cy - radius - 1));
    y < Math.min(canvas.height, Math.ceil(cy + radius + 1));
    y += 1
  ) {
    for (
      let x = Math.max(0, Math.floor(cx - radius - 1));
      x < Math.min(canvas.width, Math.ceil(cx + radius + 1));
      x += 1
    ) {
      const d = Math.hypot(x + 0.5 - cx, y + 0.5 - cy) - radius;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

export function strokeCircle(canvas, cx, cy, radius, width, color) {
  const [r, g, b, a] = color;
  for (
    let y = Math.max(0, Math.floor(cy - radius - width));
    y < Math.min(canvas.height, Math.ceil(cy + radius + width));
    y += 1
  ) {
    for (
      let x = Math.max(0, Math.floor(cx - radius - width));
      x < Math.min(canvas.width, Math.ceil(cx + radius + width));
      x += 1
    ) {
      const d = Math.abs(Math.hypot(x + 0.5 - cx, y + 0.5 - cy) - radius) - width / 2;
      const coverage = Math.min(1, Math.max(0, 0.5 - d));
      if (coverage > 0) blendPixel(canvas, x, y, r, g, b, a * coverage);
    }
  }
}

export function strokeSegment(canvas, ax, ay, bx, by, width, color) {
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

export function fillPolygon(canvas, points, color) {
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

export function strokePolygon(canvas, points, width, color) {
  for (let i = 0; i < points.length; i += 1) {
    const [x0, y0] = points[i];
    const [x1, y1] = points[(i + 1) % points.length];
    strokeSegment(canvas, x0, y0, x1, y1, width, color);
  }
}

export function downsample(canvas, factor) {
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

export function hueToRgb(t) {
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

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/**
 * Downsample, optionally multiply by a baked tint, and write the PNG.
 */
export function savePng(canvas, filePath, { supersampled = true, supersample = 4, tint } = {}) {
  const final = supersampled ? downsample(canvas, supersample) : canvas;
  if (tint) {
    for (let i = 0; i < final.data.length; i += 4) {
      final.data[i] = (final.data[i] * tint[0]) / 255;
      final.data[i + 1] = (final.data[i + 1] * tint[1]) / 255;
      final.data[i + 2] = (final.data[i + 2] * tint[2]) / 255;
    }
  }
  writeFileSync(filePath, encodePng(final.width, final.height, Buffer.from(final.data.buffer)));
  return final;
}
