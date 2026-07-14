/**
 * Rebuilds Sparks sink item sprites from the image-model master sheet at
 * `source/shop-items-master.png`.
 *
 * The master is a 3x2 chroma-magenta sheet generated to match the existing
 * Sweetie 16 hatchling/effects assets. This script keys the magenta plate,
 * trims each cell's subject, nearest-neighbor resizes it into the native
 * sprite canvas, and writes descriptor/style sidecars for future agents.
 */
import fs from 'node:fs/promises';
import path from 'node:path';
import sharp from '../../../overlay/node_modules/sharp/dist/index.mjs';

const root = path.resolve(import.meta.dirname, '..');
const projectRoot = path.resolve(root, '../../..');
const masterFile = path.join(root, 'source', 'shop-items-master.png');
const outputDir = path.join(root, 'items');
const descriptorDir = path.join(projectRoot, 'docs/assets');
const stylesDir = path.join(projectRoot, 'docs/assets/styles');
const plate = [255, 0, 255];
const palette = [
  [0x1a, 0x1c, 0x2c],
  [0xef, 0x7d, 0x57],
  [0xff, 0xcd, 0x75],
  [0xa7, 0xf0, 0x70],
  [0x38, 0xb7, 0x64],
  [0xf4, 0xf4, 0xf4],
  [0x94, 0xb0, 0xc2],
  [0xb1, 0x3e, 0x53],
  [0x29, 0x36, 0x6f],
  [0x56, 0x6c, 0x86],
];

const items = [
  {
    id: 'hat-leaf',
    label: 'Leaf Cap',
    cell: [0, 0],
    canvas: [32, 32],
    max: [30, 24],
    subject: 'leaf cap cosmetic',
    description: 'A tiny green cap with a light-green leaf sprout, sized to sit on the hatchling pet head.',
    intendedUse: 'cosmetic overlay and Sparks shop preview',
    keywords: ['leaf', 'cap', 'hat', 'cosmetic', 'sparks'],
  },
  {
    id: 'scarf-sunset',
    label: 'Sunset Scarf',
    cell: [1, 0],
    canvas: [32, 32],
    max: [30, 28],
    subject: 'sunset scarf cosmetic',
    description: 'A chunky muted-red scarf with coral dangling tails, sized to wrap around the hatchling pet neck.',
    intendedUse: 'cosmetic overlay and Sparks shop preview',
    keywords: ['scarf', 'sunset', 'cosmetic', 'sparks'],
  },
  {
    id: 'food-sushi',
    label: 'Sushi Food',
    cell: [2, 0],
    canvas: [32, 32],
    max: [24, 24],
    subject: 'sushi food skin',
    description: 'A small maki sushi bite with dark seaweed, off-white rice, and muted-red filling.',
    intendedUse: 'food drop skin and Sparks shop preview',
    keywords: ['sushi', 'food', 'skin', 'sparks'],
  },
  {
    id: 'food-banh-mi',
    label: 'Banh Mi Food',
    cell: [0, 1],
    canvas: [32, 32],
    max: [30, 22],
    subject: 'banh mi food skin',
    description: 'A warm yellow baguette sandwich with small green filling details, readable as a food drop.',
    intendedUse: 'food drop skin and Sparks shop preview',
    keywords: ['banh-mi', 'food', 'sandwich', 'skin', 'sparks'],
  },
  {
    id: 'furniture-bed',
    label: 'Tiny Bed',
    cell: [1, 1],
    canvas: [80, 40],
    max: [76, 34],
    subject: 'tiny bed furniture',
    description: 'A low navy pet bed with a cool-gray pillow, wide enough for the hatchling to walk to and sleep on.',
    intendedUse: 'overlay furniture placement and Sparks shop preview',
    keywords: ['bed', 'furniture', 'sleep', 'sparks'],
  },
  {
    id: 'furniture-plant',
    label: 'Desk Plant',
    cell: [2, 1],
    canvas: [80, 40],
    max: [42, 38],
    subject: 'desk plant furniture',
    description: 'A small terracotta desk plant with two green leaves, used as decorative overlay furniture.',
    intendedUse: 'overlay furniture placement and Sparks shop preview',
    keywords: ['plant', 'furniture', 'desk', 'sparks'],
  },
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

// A pixel is plate only if it is actually magenta-hued: both red and blue
// dominate green. Distance alone misclassifies warm reds (e.g. the sushi
// filling ~#EE4D58 sits within 200 of #FF00FF) as background.
function isPlate(r, g, b) {
  const magentaHued = r - g > 90 && b - g > 90;
  return magentaHued && distanceToPlate(r, g, b) < 360;
}

function keyedCell(master, left, top, width, height) {
  const out = Buffer.alloc(width * height * 4);
  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const srcOffset = ((top + y) * master.width + (left + x)) * 4;
      const dstOffset = (y * width + x) * 4;
      const r = master.data[srcOffset];
      const g = master.data[srcOffset + 1];
      const b = master.data[srcOffset + 2];
      const a = master.data[srcOffset + 3];
      const opaque = a > 0 && !isPlate(r, g, b);
      out[dstOffset] = r;
      out[dstOffset + 1] = g;
      out[dstOffset + 2] = b;
      out[dstOffset + 3] = opaque ? 255 : 0;
      if (opaque) {
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
  }

  if (maxX < 0) {
    throw new Error(`No subject found in cell ${left},${top}`);
  }

  const pad = 4;
  return {
    data: out,
    width,
    height,
    trim: {
      left: Math.max(0, minX - pad),
      top: Math.max(0, minY - pad),
      width: Math.min(width, maxX - minX + 1 + pad * 2),
      height: Math.min(height, maxY - minY + 1 + pad * 2),
    },
  };
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

function styleSnapshot(item) {
  return `id: tokengochi-sweetie16-v1/${item.id}
palette: ["#1a1c2c", "#ef7d57", "#ffcd75", "#a7f070", "#38b764", "#f4f4f4", "#94b0c2", "#b13e53", "#29366f"]
line:
  weight: "1px native pixel outline"
  style: uniform
shading: flat-two-tone
lighting: top-left
camera: side
proportions:
  canvas: "${item.canvas[0]}x${item.canvas[1]}"
  max_subject: "${item.max[0]}x${item.max[1]}"
prompt_suffix: "strict Sweetie 16 pixel art, 1px #1a1c2c outline, flat top-left two-tone light, binary alpha, no anti-aliasing"
negative: [gradients, partial-alpha, purple-or-blue-glow, bevels, non-palette-colors]
source:
  master: ui/assets/sprites/source/shop-items-master.png
  plate: "#FF00FF"
`;
}

function descriptor(item, file, bytes) {
  const relPath = path.relative(projectRoot, file);
  const size = `${item.canvas[0]}x${item.canvas[1]}`;
  return `id: ${item.id}
type: sprite
subject: ${item.subject}
description: >
  ${item.description}
keywords: [${item.keywords.join(', ')}]
placement:
  intended_use: ${item.intendedUse}
  context: Tokengochi Sparks sinks shop, overlay renderer
  do: [use at native pixel size or integer scale, preserve transparent alpha]
  dont: [do not stretch non-uniformly, do not bilinear scale]
style:
  profile: docs/assets/styles/style-profile-${item.id}.yaml
  art_style: Sweetie 16 pixel art
  stroke: 1px native pixel outline
  shading: flat two-tone
palette: ["#1a1c2c", "#ef7d57", "#ffcd75", "#a7f070", "#38b764", "#f4f4f4", "#94b0c2", "#b13e53", "#29366f"]
background: transparent
dimensions:
  master: ui/assets/sprites/source/shop-items-master.png
  output: ${size}
  aspect: "${item.canvas[0]}:${item.canvas[1]}"
safe_area: inner padded cell
accessibility:
  alt_text: "${item.label} pixel-art item"
files:
  - path: ${relPath}
    size: ${size}
    format: png
    bytes: ${bytes}
source:
  model: codex image generation
  prompt: "Tokengochi Sparks sink item sprite, ${item.subject}, Sweetie 16 pixel art, chroma-magenta plate"
`;
}

async function main() {
  await fs.mkdir(outputDir, { recursive: true });
  await fs.mkdir(descriptorDir, { recursive: true });
  await fs.mkdir(stylesDir, { recursive: true });

  const master = await readRgba(masterFile);
  const cellW = Math.floor(master.width / 3);
  const cellH = Math.floor(master.height / 2);
  const results = [];

  for (const item of items) {
    const [col, row] = item.cell;
    const cell = keyedCell(master, col * cellW, row * cellH, cellW, cellH);
    const scale = Math.min(item.max[0] / cell.trim.width, item.max[1] / cell.trim.height);
    const resizedW = Math.max(1, Math.round(cell.trim.width * scale));
    const resizedH = Math.max(1, Math.round(cell.trim.height * scale));
    const left = Math.round((item.canvas[0] - resizedW) / 2);
    const top = Math.round((item.canvas[1] - resizedH) / 2);
    const filename = `${item.id}-sprite-${item.canvas[0]}x${item.canvas[1]}.png`;
    const destination = path.join(outputDir, filename);

    const scaledBuffer = await sharp(cell.data, { raw: { width: cell.width, height: cell.height, channels: 4 } })
      .extract(cell.trim)
      .resize(resizedW, resizedH, { kernel: 'nearest' })
      .png()
      .toBuffer();
    const pngBuffer = await quantizePng(scaledBuffer);

    await sharp({
      create: {
        width: item.canvas[0],
        height: item.canvas[1],
        channels: 4,
        background: { r: 0, g: 0, b: 0, alpha: 0 },
      },
    })
      .composite([{ input: pngBuffer, left, top }])
      .png()
      .toFile(destination);

    const stat = await fs.stat(destination);
    await fs.writeFile(path.join(stylesDir, `style-profile-${item.id}.yaml`), styleSnapshot(item));
    await fs.writeFile(path.join(descriptorDir, `${item.id}.yaml`), descriptor(item, destination, stat.size));
    results.push({ id: item.id, file: path.relative(projectRoot, destination), bytes: stat.size });
  }

  console.table(results);
}

await main();
