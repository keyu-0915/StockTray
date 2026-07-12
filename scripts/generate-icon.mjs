import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const root = path.resolve(import.meta.dirname, '..');
const iconDir = path.join(root, 'src-tauri', 'icons');
const previewPath = path.join(iconDir, 'icon-preview.png');
const icoPath = path.join(iconDir, 'icon.ico');
const sizes = [16, 24, 32, 48, 64, 128, 256];

const crcTable = new Uint32Array(256);
for (let i = 0; i < 256; i += 1) {
  let c = i;
  for (let k = 0; k < 8; k += 1) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  crcTable[i] = c >>> 0;
}

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type);
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  typeBuffer.copy(out, 4);
  data.copy(out, 8);
  out.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])), 8 + data.length);
  return out;
}

function encodePng(width, height, pixels) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  const rows = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y += 1) {
    const row = y * (width * 4 + 1);
    rows[row] = 0;
    pixels.copy(rows, row + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', header),
    chunk('IDAT', zlib.deflateSync(rows, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ]);
}

function mix(a, b, t) {
  return a + (b - a) * t;
}

function blendPixel(data, i, color, alpha) {
  const srcA = Math.max(0, Math.min(1, alpha));
  if (srcA <= 0) return;
  const dstA = data[i + 3] / 255;
  const outA = srcA + dstA * (1 - srcA);
  if (outA <= 0) return;
  data[i] = Math.round((color[0] * srcA + data[i] * dstA * (1 - srcA)) / outA);
  data[i + 1] = Math.round((color[1] * srcA + data[i + 1] * dstA * (1 - srcA)) / outA);
  data[i + 2] = Math.round((color[2] * srcA + data[i + 2] * dstA * (1 - srcA)) / outA);
  data[i + 3] = Math.round(outA * 255);
}

function roundedRectDistance(x, y, rx, ry, rw, rh, rr) {
  const qx = Math.abs(x - (rx + rw / 2)) - rw / 2 + rr;
  const qy = Math.abs(y - (ry + rh / 2)) - rh / 2 + rr;
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - rr;
}

function segmentDistance(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  const len = dx * dx + dy * dy;
  const t = len === 0 ? 0 : Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / len));
  const x = ax + t * dx;
  const y = ay + t * dy;
  return Math.hypot(px - x, py - y);
}

function drawCapsule(data, size, scale, ax, ay, bx, by, width, color, alpha) {
  const radius = width / 2;
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const px = (x + 0.5) / scale;
      const py = (y + 0.5) / scale;
      const d = segmentDistance(px, py, ax, ay, bx, by);
      const coverage = Math.max(0, Math.min(1, radius + 0.55 - d));
      if (coverage > 0) blendPixel(data, (y * size + x) * 4, color, alpha * coverage);
    }
  }
}

function drawPolyline(data, size, scale, points, width, fixedColor = null, alpha = 1) {
  const radius = width / 2;
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const px = (x + 0.5) / scale;
      const py = (y + 0.5) / scale;
      let d = Infinity;
      for (let i = 0; i < points.length - 1; i += 1) {
        d = Math.min(d, segmentDistance(px, py, points[i][0], points[i][1], points[i + 1][0], points[i + 1][1]));
      }
      const coverage = Math.max(0, Math.min(1, radius + 0.55 - d));
      if (coverage <= 0) continue;
      const t = Math.max(0, Math.min(1, (px - 42) / (214 - 42)));
      const color = fixedColor ?? [mix(73, 140, t), mix(183, 228, t), mix(240, 255, t)];
      blendPixel(data, (y * size + x) * 4, color, alpha * coverage);
    }
  }
}

function renderIcon(targetSize) {
  const sample = targetSize <= 48 ? 5 : 3;
  const high = targetSize * sample;
  const scale = high / 256;
  const data = Buffer.alloc(high * high * 4);
  for (let y = 0; y < high; y += 1) {
    for (let x = 0; x < high; x += 1) {
      const distance = roundedRectDistance((x + .5) / scale, (y + .5) / scale, 18, 18, 220, 220, 48);
      blendPixel(data, (y * high + x) * 4, [26, 105, 224], Math.max(0, Math.min(1, .7 - distance)));
    }
  }
  drawPolyline(data, high, scale, [[105, 76], [55, 76], [55, 126], [105, 126], [105, 180], [50, 180]], 22, [248, 251, 255]);
  drawCapsule(data, high, scale, 126, 76, 205, 76, 22, [248, 251, 255], 1);
  drawCapsule(data, high, scale, 166, 76, 166, 180, 22, [248, 251, 255], 1);

  const out = Buffer.alloc(targetSize * targetSize * 4);
  for (let y = 0; y < targetSize; y += 1) {
    for (let x = 0; x < targetSize; x += 1) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      for (let sy = 0; sy < sample; sy += 1) {
        for (let sx = 0; sx < sample; sx += 1) {
          const i = ((y * sample + sy) * high + (x * sample + sx)) * 4;
          r += data[i];
          g += data[i + 1];
          b += data[i + 2];
          a += data[i + 3];
        }
      }
      const n = sample * sample;
      const o = (y * targetSize + x) * 4;
      out[o] = Math.round(r / n);
      out[o + 1] = Math.round(g / n);
      out[o + 2] = Math.round(b / n);
      out[o + 3] = Math.round(a / n);
    }
  }
  return out;
}

function encodeIco(images) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  let offset = 6 + images.length * 16;
  const entries = [];
  for (const image of images) {
    const entry = Buffer.alloc(16);
    entry[0] = image.size === 256 ? 0 : image.size;
    entry[1] = image.size === 256 ? 0 : image.size;
    entry[2] = 0;
    entry[3] = 0;
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(image.png.length, 8);
    entry.writeUInt32LE(offset, 12);
    entries.push(entry);
    offset += image.png.length;
  }
  return Buffer.concat([header, ...entries, ...images.map((image) => image.png)]);
}

const images = sizes.map((size) => ({ size, png: encodePng(size, size, renderIcon(size)) }));
fs.writeFileSync(icoPath, encodeIco(images));
fs.writeFileSync(previewPath, images.at(-1).png);
console.log(`Wrote ${icoPath}`);
console.log(`Wrote ${previewPath}`);
