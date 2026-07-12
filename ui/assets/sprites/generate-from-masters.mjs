/**
 * Rebuilds the hatchling/effects sprite sheets from image-model master
 * artwork (`source/*-master.png`).
 *
 * Each frame's crop box below is the *exact* content bounding box in master
 * pixel coordinates - not a per-column formula. An earlier version of this
 * script sliced frames off a uniform nominal grid (fixed cell width/height,
 * fixed left/top offset), which assumed every pose sits identically inside
 * its cell. It doesn't: leaf sprouts, raised arms, and the dangling-stem
 * "drag" pose all extend different amounts, so the uniform slice clipped
 * them, and the previous "happy" row also duplicated two frames that
 * actually belong to "drag" (the master only has 4 unique happy poses, not
 * 6 - see the `happy` tag range below). The boxes here were derived by
 * connected-component analysis of the master art (grouping each body blob
 * with its nearby satellite glyphs - Z's, sparkles, corner dust dots) so
 * every frame is a tight, complete, non-duplicated crop.
 *
 * Cutout uses a single hard chroma-key threshold (distance from the sampled
 * magenta plate color), not alpha blending + palette quantization: the
 * source is AI-generated art whose antialiased magenta/foreground edge
 * pixels don't follow a clean linear blend, so estimating partial alpha and
 * un-mixing the true color left a visible magenta-tinted fringe. A hard cut
 * also matches the pixel-art / `image-rendering: pixelated` rendering this
 * sheet is used with. Frames are pasted at native master resolution (no
 * downscale) into a padded canvas - see docs/decisions/0003-canvas-sprite-rendering.md.
 *
 * Requires `npm install sharp` in ui/overlay.
 */
import fs from 'node:fs/promises';
import path from 'node:path';
import sharp from '../../overlay/node_modules/sharp/dist/index.mjs';

const root = path.resolve(import.meta.dirname);
const HARD_THRESHOLD = 200;

async function readRgba(file) {
  const { data, info } = await sharp(file).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  return { data, width: info.width, height: info.height };
}

function keyedCrop(master, box) {
  const { left, top, width, height } = box;
  const out = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const srcOffset = ((top + y) * master.width + (left + x)) * 4;
      const dstOffset = (y * width + x) * 4;
      const r = master.data[srcOffset];
      const g = master.data[srcOffset + 1];
      const b = master.data[srcOffset + 2];
      const dr = r - master.magenta[0];
      const dg = g - master.magenta[1];
      const db = b - master.magenta[2];
      const dist = Math.sqrt(dr * dr + dg * dg + db * db);
      out[dstOffset] = r;
      out[dstOffset + 1] = g;
      out[dstOffset + 2] = b;
      out[dstOffset + 3] = dist >= HARD_THRESHOLD ? 255 : 0;
    }
  }
  return { data: out, width, height };
}

async function loadMaster(file) {
  const { data, width, height } = await readRgba(file);
  const magenta = [data[(2 * width + 2) * 4], data[(2 * width + 2) * 4 + 1], data[(2 * width + 2) * 4 + 2]];
  return { data, width, height, magenta };
}

/** Builds one sheet: crops each frame tightly (via `keyedCrop`), then pastes
 * every frame into a shared, evenly-padded square cell (`cellSize`),
 * bottom-anchored so a walk/idle/eat cycle keeps a consistent "feet on the
 * ground" line despite frames having different content heights (a leaf
 * sprout, raised arm, etc.). Used for the hatchling body. */
async function buildUniformSheet(masterFile, frameSpecs, destination, { columns, pad, bottomMargin }) {
  const master = await loadMaster(masterFile);
  const crops = frameSpecs.map((spec) => ({ ...spec, crop: keyedCrop(master, spec.box) }));
  const maxW = Math.max(...crops.map((c) => c.crop.width));
  const maxH = Math.max(...crops.map((c) => c.crop.height));
  const cell = Math.ceil((Math.max(maxW, maxH) + pad * 2) / 4) * 4;
  const rows = Math.ceil(crops.length / columns);

  const composites = crops.map((c, index) => {
    const col = index % columns;
    const row = Math.floor(index / columns);
    const px = col * cell + Math.round((cell - c.crop.width) / 2);
    const py = row * cell + cell - bottomMargin - c.crop.height;
    return {
      input: c.crop.data,
      raw: { width: c.crop.width, height: c.crop.height, channels: 4 },
      left: px,
      top: py,
    };
  });

  await sharp({ create: { width: columns * cell, height: rows * cell, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
    .composite(composites)
    .png()
    .toFile(destination);

  return crops.map((c, index) => ({
    tag: c.tag,
    duration: c.duration,
    x: (index % columns) * cell,
    y: Math.floor(index / columns) * cell,
    w: cell,
    h: cell,
  }));
}

/** Builds the effects sheet: each icon (zzz/heart/exclaim/dust) keeps its
 * own tight square cell sized to its own content, packed into a grid - these
 * are independent one-off icons of very different natural sizes (a small
 * "zzz" bubble vs. a large dust cloud), not a walk cycle, so sharing one
 * canvas size the way the hatchling body frames do would either crush the
 * small icons or waste huge padding around them. */
async function buildPackedSheet(masterFile, frameSpecs, destination, { columns, pad }) {
  const master = await loadMaster(masterFile);
  const crops = frameSpecs.map((spec) => ({ ...spec, crop: keyedCrop(master, spec.box) }));
  const cellSizes = crops.map((c) => Math.ceil((Math.max(c.crop.width, c.crop.height) + pad * 2) / 4) * 4);
  const rows = Math.ceil(crops.length / columns);

  const colWidths = new Array(columns).fill(0);
  const rowHeights = new Array(rows).fill(0);
  crops.forEach((_, index) => {
    const col = index % columns;
    const row = Math.floor(index / columns);
    colWidths[col] = Math.max(colWidths[col], cellSizes[index]);
    rowHeights[row] = Math.max(rowHeights[row], cellSizes[index]);
  });
  const colX = [0];
  colWidths.forEach((w) => colX.push(colX[colX.length - 1] + w));
  const rowY = [0];
  rowHeights.forEach((h) => rowY.push(rowY[rowY.length - 1] + h));

  const composites = crops.map((c, index) => {
    const col = index % columns;
    const row = Math.floor(index / columns);
    const cellW = colWidths[col];
    const cellH = rowHeights[row];
    const px = colX[col] + Math.round((cellW - c.crop.width) / 2);
    const py = rowY[row] + Math.round((cellH - c.crop.height) / 2);
    return {
      input: c.crop.data,
      raw: { width: c.crop.width, height: c.crop.height, channels: 4 },
      left: px,
      top: py,
    };
  });

  await sharp({ create: { width: colX[colX.length - 1], height: rowY[rowY.length - 1], channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
    .composite(composites)
    .png()
    .toFile(destination);

  return crops.map((c, index) => {
    const col = index % columns;
    const row = Math.floor(index / columns);
    return { tag: c.tag, duration: c.duration, x: colX[col], y: rowY[row], w: colWidths[col], h: rowHeights[row] };
  });
}

function asepriteJson(image, frames) {
  const tagRanges = [];
  frames.forEach((frame, index) => {
    const current = tagRanges[tagRanges.length - 1];
    if (current && current.name === frame.tag) {
      current.to = index;
    } else {
      tagRanges.push({ name: frame.tag, from: index, to: index });
    }
  });
  return {
    frames: frames.map((frame, index) => ({
      filename: `${path.basename(image, '.png')} ${index}.aseprite`,
      frame: { x: frame.x, y: frame.y, w: frame.w, h: frame.h },
      duration: frame.duration,
    })),
    meta: {
      app: 'Tokengochi image-model pipeline',
      version: '1.0',
      image,
      format: 'RGBA8888',
      size: {
        w: Math.max(...frames.map((f) => f.x + f.w)),
        h: Math.max(...frames.map((f) => f.y + f.h)),
      },
      scale: '1',
      frameTags: tagRanges.map((tag) => ({ ...tag, direction: 'forward' })),
    },
  };
}

const hatchlingMaster = path.join(root, 'source', 'hatchling-master.png');
const effectsMaster = path.join(root, 'source', 'effects-master.png');

// (tag, duration-ms, [left, top, right, bottom]) - tight content bboxes in
// master pixel coordinates, derived once via connected-component analysis
// of hatchling-master.png (see task 0012 follow-up notes).
const hatchlingFrameSpecs = [
  ['idle', 150, [61, 31, 187, 168]], ['idle', 150, [265, 31, 390, 168]],
  ['idle', 150, [468, 31, 593, 168]], ['idle', 150, [672, 31, 797, 168]],
  ['walk', 100, [45, 208, 169, 343]], ['walk', 100, [251, 208, 373, 347]],
  ['walk', 100, [451, 208, 573, 346]], ['walk', 100, [658, 208, 783, 350]],
  ['walk', 100, [852, 208, 979, 346]], ['walk', 100, [1058, 208, 1182, 347]],
  ['sleep', 250, [46, 382, 178, 515]], ['sleep', 250, [256, 382, 400, 515]],
  ['sleep', 250, [466, 382, 609, 515]], ['sleep', 250, [677, 382, 828, 515]],
  ['eat', 100, [46, 548, 165, 681]], ['eat', 100, [255, 548, 374, 681]],
  ['eat', 100, [461, 548, 580, 681]], ['eat', 100, [666, 545, 783, 682]],
  ['eat', 100, [865, 548, 983, 682]], ['eat', 100, [1065, 548, 1183, 682]],
  ['happy', 100, [27, 710, 190, 832]], ['happy', 100, [250, 710, 372, 832]],
  ['happy', 100, [434, 710, 547, 839]], ['happy', 100, [616, 712, 725, 845]],
  ['drag', 150, [837, 693, 956, 852]], ['drag', 150, [1025, 694, 1132, 855]],
  ['react', 100, [40, 851, 177, 977]], ['react', 100, [232, 877, 382, 973]],
  ['react', 100, [434, 856, 559, 977]],
].map(([tag, duration, [left, top, right, bottom]]) => ({
  tag, duration, box: { left, top, width: right - left, height: bottom - top },
}));

const effectsFrameSpecs = [
  ['zzz', 300, [96, 169, 249, 344]], ['zzz', 300, [355, 100, 560, 351]], ['zzz', 300, [642, 62, 883, 352]],
  ['heart', 200, [85, 521, 216, 639]], ['heart', 200, [348, 481, 546, 649]],
  ['exclaim', 400, [646, 446, 849, 653]],
  ['dust', 80, [86, 817, 242, 947]], ['dust', 80, [349, 766, 547, 956]],
  ['dust', 80, [630, 737, 885, 959]], ['dust', 80, [975, 769, 1173, 980]],
].map(([tag, duration, [left, top, right, bottom]]) => ({
  tag, duration, box: { left, top, width: right - left, height: bottom - top },
}));

const [hatchlingFrames, effectsFrames] = await Promise.all([
  buildUniformSheet(hatchlingMaster, hatchlingFrameSpecs, path.join(root, 'hatchling', 'hatchling.png'), {
    columns: 8, pad: 10, bottomMargin: 14,
  }),
  buildPackedSheet(effectsMaster, effectsFrameSpecs, path.join(root, 'effects', 'effects.png'), {
    columns: 5, pad: 8,
  }),
]);

await Promise.all([
  fs.writeFile(
    path.join(root, 'hatchling', 'hatchling.json'),
    `${JSON.stringify(asepriteJson('hatchling.png', hatchlingFrames), null, 2)}\n`,
  ),
  fs.writeFile(
    path.join(root, 'effects', 'effects.json'),
    `${JSON.stringify(asepriteJson('effects.png', effectsFrames), null, 2)}\n`,
  ),
]);
