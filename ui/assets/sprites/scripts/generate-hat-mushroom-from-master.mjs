/**
 * Rebuilds the Mushroom Cap Sparks sink item from
 * `source/hat-mushroom-master.png`.
 *
 * The master comes from image generation on a chroma-magenta plate. This
 * script keys the plate, trims the subject, nearest-neighbor scales it into a
 * 32x32 wearable-hat sprite, quantizes to the project's Sweetie 16 subset,
 * and writes the asset descriptor/style snapshot.
 */
import fs from 'node:fs/promises';
import path from 'node:path';
import sharp from '../../../overlay/node_modules/sharp/dist/index.mjs';

const root = path.resolve(import.meta.dirname, '..');
const projectRoot = path.resolve(root, '../../..');
const masterFile = path.join(root, 'source', 'hat-mushroom-master.png');
const outputFile = path.join(root, 'items', 'hat-mushroom-sprite-32x32.png');
const descriptorFile = path.join(projectRoot, 'docs/assets/hat-mushroom.yaml');
const styleFile = path.join(projectRoot, 'docs/assets/styles/style-profile-hat-mushroom.yaml');
const plate = [255, 0, 255];
const plateThreshold = 145;
const canvas = [32, 32];
const maxSubject = [30, 22];
const palette = [
  [0x1a, 0x1c, 0x2c],
  [0xef, 0x7d, 0x57],
  [0xff, 0xcd, 0x75],
  [0xf4, 0xf4, 0xf4],
  [0x94, 0xb0, 0xc2],
  [0xb1, 0x3e, 0x53],
  [0x29, 0x36, 0x6f],
  [0x56, 0x6c, 0x86],
];

async function readRgba(file) {
  const { data, info } = await sharp(file).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  return { data, width: info.width, height: info.height };
}

function distanceToPlate(r, g, b) {
  const dr = r - plate[0];
  const dg = g - plate[1];
  const db = b - plate[2];
  return Math.sqrt(dr * dr + dg * dg + db * db);
}

function nearestPaletteColor(r, g, b) {
  let best = palette[0];
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const color of palette) {
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

function keyAndTrim(master) {
  const out = Buffer.alloc(master.width * master.height * 4);
  let minX = master.width;
  let minY = master.height;
  let maxX = -1;
  let maxY = -1;

  for (let y = 0; y < master.height; y += 1) {
    for (let x = 0; x < master.width; x += 1) {
      const offset = (y * master.width + x) * 4;
      const r = master.data[offset];
      const g = master.data[offset + 1];
      const b = master.data[offset + 2];
      const a = master.data[offset + 3];
      const opaque = a > 0 && distanceToPlate(r, g, b) > plateThreshold;
      out[offset] = r;
      out[offset + 1] = g;
      out[offset + 2] = b;
      out[offset + 3] = opaque ? 255 : 0;
      if (opaque) {
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
  }

  if (maxX < 0) {
    throw new Error('No Mushroom Cap subject found in master image');
  }

  const pad = 10;
  return {
    data: out,
    width: master.width,
    height: master.height,
    trim: {
      left: Math.max(0, minX - pad),
      top: Math.max(0, minY - pad),
      width: Math.min(master.width - Math.max(0, minX - pad), maxX - minX + 1 + pad * 2),
      height: Math.min(master.height - Math.max(0, minY - pad), maxY - minY + 1 + pad * 2),
    },
  };
}

async function quantizePng(buffer) {
  const { data, info } = await sharp(buffer).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] < 128) {
      data[i] = 0;
      data[i + 1] = 0;
      data[i + 2] = 0;
      data[i + 3] = 0;
      continue;
    }
    const color = nearestPaletteColor(data[i], data[i + 1], data[i + 2]);
    data[i] = color[0];
    data[i + 1] = color[1];
    data[i + 2] = color[2];
    data[i + 3] = 255;
  }
  return sharp(data, { raw: { width: info.width, height: info.height, channels: 4 } }).png().toBuffer();
}

function styleSnapshot() {
  return `id: tokengochi-sweetie16-v1/hat-mushroom
palette: ["#1a1c2c", "#ef7d57", "#ffcd75", "#f4f4f4", "#94b0c2", "#b13e53", "#29366f", "#566c86"]
line:
  weight: "1px native pixel outline"
  style: uniform
shading: flat-two-tone
lighting: top-left
camera: side
proportions:
  canvas: "32x32"
  max_subject: "30x22"
prompt_suffix: "strict Sweetie 16 pixel art, 1px #1a1c2c outline, flat top-left two-tone light, binary alpha, no anti-aliasing"
negative: [gradients, partial-alpha, purple-or-blue-glow, bevels, non-palette-colors]
source:
  master: ui/assets/sprites/source/hat-mushroom-master.png
  plate: "#FF00FF"
`;
}

function descriptor(bytes) {
  return `id: hat-mushroom
type: sprite
subject: mushroom cap cosmetic
description: >
  A red mushroom-dome cap with off-white spots and a pale underside band,
  sized to sit on the hatchling pet head as a wearable cosmetic.
keywords: [mushroom, cap, hat, cosmetic, sparks]
placement:
  intended_use: cosmetic overlay and Sparks shop preview
  context: Tokengochi Sparks sinks shop, overlay renderer
  do: [use at native pixel size or integer scale, preserve transparent alpha]
  dont: [do not stretch non-uniformly, do not bilinear scale]
style:
  profile: docs/assets/styles/style-profile-hat-mushroom.yaml
  art_style: Sweetie 16 pixel art
  stroke: 1px native pixel outline
  shading: flat two-tone
palette: ["#1a1c2c", "#ef7d57", "#ffcd75", "#f4f4f4", "#94b0c2", "#b13e53", "#29366f", "#566c86"]
background: transparent
dimensions:
  master: ui/assets/sprites/source/hat-mushroom-master.png
  output: 32x32
  aspect: "1:1"
safe_area: inner padded cell
accessibility:
  alt_text: "Mushroom Cap pixel-art item"
files:
  - path: ui/assets/sprites/items/hat-mushroom-sprite-32x32.png
    size: 32x32
    format: png
    bytes: ${bytes}
source:
  model: codex image generation
  prompt: "Tokengochi Mushroom Cap cosmetic item sprite, red mushroom-dome cap with off-white spots, Sweetie 16 pixel art, chroma-magenta plate"
`;
}

async function main() {
  await fs.mkdir(path.dirname(outputFile), { recursive: true });
  await fs.mkdir(path.dirname(descriptorFile), { recursive: true });
  await fs.mkdir(path.dirname(styleFile), { recursive: true });

  const master = await readRgba(masterFile);
  const keyed = keyAndTrim(master);
  const scale = Math.min(maxSubject[0] / keyed.trim.width, maxSubject[1] / keyed.trim.height);
  const resizedW = Math.max(1, Math.round(keyed.trim.width * scale));
  const resizedH = Math.max(1, Math.round(keyed.trim.height * scale));
  const left = Math.round((canvas[0] - resizedW) / 2);
  const top = Math.round((canvas[1] - resizedH) / 2);

  const scaledBuffer = await sharp(keyed.data, { raw: { width: keyed.width, height: keyed.height, channels: 4 } })
    .extract(keyed.trim)
    .resize(resizedW, resizedH, { kernel: 'nearest' })
    .png()
    .toBuffer();
  const spriteBuffer = await quantizePng(scaledBuffer);

  await sharp({
    create: {
      width: canvas[0],
      height: canvas[1],
      channels: 4,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    },
  })
    .composite([{ input: spriteBuffer, left, top }])
    .png()
    .toFile(outputFile);

  const stat = await fs.stat(outputFile);
  await fs.writeFile(styleFile, styleSnapshot());
  await fs.writeFile(descriptorFile, descriptor(stat.size));
  console.table([{ id: 'hat-mushroom', file: path.relative(projectRoot, outputFile), bytes: stat.size }]);
}

await main();
