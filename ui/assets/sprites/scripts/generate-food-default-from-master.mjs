/**
 * Rebuilds the default food drop sprite from image-model master art.
 *
 * Pixels come from source/food-default-master.png. This script only
 * chroma-keys, crops, quantizes to Sweetie 16, and packs into a 32x32 item
 * cell.
 */
import path from 'node:path';
import sharp from '../../../overlay/node_modules/sharp/dist/index.mjs';

const root = path.resolve(import.meta.dirname, '..');
const masterPath = path.join(root, 'source', 'food-default-master.png');
const outputPath = path.join(root, 'items', 'food-default-sprite-32x32.png');
const MAGENTA = [255, 0, 255];
const SWEETIE16 = [
  [0x1a, 0x1c, 0x2c],
  [0x5d, 0x27, 0x5d],
  [0xb1, 0x3e, 0x53],
  [0xef, 0x7d, 0x57],
  [0xff, 0xcd, 0x75],
  [0xa7, 0xf0, 0x70],
  [0x38, 0xb7, 0x64],
  [0x25, 0x71, 0x79],
  [0x29, 0x36, 0x6f],
  [0x3b, 0x5d, 0xc9],
  [0x41, 0xa6, 0xf6],
  [0x73, 0xef, 0xf7],
  [0xf4, 0xf4, 0xf4],
  [0x94, 0xb0, 0xc2],
  [0x56, 0x6c, 0x86],
  [0x33, 0x3c, 0x57],
];

const { data, info } = await sharp(masterPath).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
const keyed = Buffer.alloc(data.length);
let minX = info.width;
let minY = info.height;
let maxX = 0;
let maxY = 0;

for (let y = 0; y < info.height; y += 1) {
  for (let x = 0; x < info.width; x += 1) {
    const offset = (y * info.width + x) * 4;
    const r = data[offset];
    const g = data[offset + 1];
    const b = data[offset + 2];
    if (isPlatePixel(r, g, b)) {
      keyed[offset + 3] = 0;
      continue;
    }
    const [qr, qg, qb] = nearestPaletteColor(r, g, b);
    keyed[offset] = qr;
    keyed[offset + 1] = qg;
    keyed[offset + 2] = qb;
    keyed[offset + 3] = 255;
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  }
}

const crop = {
  left: Math.max(0, minX - 2),
  top: Math.max(0, minY - 2),
  width: Math.min(info.width - minX, maxX - minX + 5),
  height: Math.min(info.height - minY, maxY - minY + 5),
};

const resized = await sharp(keyed, { raw: { width: info.width, height: info.height, channels: 4 } })
  .extract(crop)
  .resize({ width: 24, height: 24, fit: 'inside', kernel: 'nearest' })
  .png()
  .toBuffer({ resolveWithObject: true });

await sharp({ create: { width: 32, height: 32, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
  .composite([{ input: resized.data, left: Math.floor((32 - resized.info.width) / 2), top: Math.floor((32 - resized.info.height) / 2) }])
  .png()
  .toFile(outputPath);

function isPlatePixel(r, g, b) {
  const dr = r - MAGENTA[0];
  const dg = g - MAGENTA[1];
  const db = b - MAGENTA[2];
  const distance = Math.sqrt(dr * dr + dg * dg + db * db);
  return distance < 165 || (r > 180 && b > 150 && g < 95);
}

function nearestPaletteColor(r, g, b) {
  let best = SWEETIE16[0];
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const color of SWEETIE16) {
    const dr = r - color[0];
    const dg = g - color[1];
    const db = b - color[2];
    const distance = dr * dr + dg * dg + db * db;
    if (distance < bestDistance) {
      best = color;
      bestDistance = distance;
    }
  }
  return best;
}
