/**
 * Rebuilds Task 0005 placeholder sheets from image-model master artwork.
 * Requires `npm install sharp` in ui/overlay.
 */
import fs from 'node:fs/promises';
import path from 'node:path';
import sharp from '../../overlay/node_modules/sharp/dist/index.mjs';

const root = path.resolve(import.meta.dirname);
const palette = ['#1a1c2c', '#ef7d57', '#ffcd75', '#a7f070', '#38b764', '#f4f4f4', '#94b0c2'];
const colors = palette.map((hex) => hex.match(/\w\w/g).map((part) => Number.parseInt(part, 16)));

function nearestPalette(r, g, b) {
  return colors.reduce((closest, color) => {
    const distance = (r - color[0]) ** 2 + (g - color[1]) ** 2 + (b - color[2]) ** 2;
    return distance < closest.distance ? { color, distance } : closest;
  }, { color: colors[0], distance: Infinity }).color;
}

function removePlateAndQuantize(data) {
  for (let offset = 0; offset < data.length; offset += 4) {
    const [r, g, b] = data.subarray(offset, offset + 3);
    // Magenta plate and its generated near-magenta variants.
    if (r > 170 && b > 150 && g < 130) {
      data[offset + 3] = 0;
      data[offset] = data[offset + 1] = data[offset + 2] = 0;
      continue;
    }
    const [qr, qg, qb] = nearestPalette(r, g, b);
    data[offset] = qr;
    data[offset + 1] = qg;
    data[offset + 2] = qb;
    data[offset + 3] = data[offset + 3] >= 128 ? 255 : 0;
  }
  return data;
}

async function extract(master, box, size) {
  const { data, info } = await sharp(master)
    .extract(box)
    .resize(size, size, { fit: 'contain', kernel: sharp.kernel.nearest, background: { r: 255, g: 0, b: 255, alpha: 1 } })
    .ensureAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  return sharp(removePlateAndQuantize(data), { raw: info }).png().toBuffer();
}

async function buildSheet(master, frames, cell, columns, destination, fixedRows) {
  const cells = await Promise.all(frames.map((box) => extract(master, box, cell)));
  const rows = fixedRows ?? Math.ceil(cells.length / columns);
  const composite = cells.map((input, index) => ({ input, left: (index % columns) * cell, top: Math.floor(index / columns) * cell }));
  await sharp({ create: { width: columns * cell, height: rows * cell, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
    .composite(composite)
    .png({ palette: true })
    .toFile(destination);
}

function asepriteJson(image, cell, columns, count, tags, rows = Math.ceil(count / columns)) {
  return {
    frames: Array.from({ length: count }, (_, index) => ({
      filename: `${path.basename(image, '.png')} ${index}.aseprite`,
      frame: { x: (index % columns) * cell, y: Math.floor(index / columns) * cell, w: cell, h: cell },
      duration: tags.find((tag) => index >= tag.from && index <= tag.to).duration,
    })),
    meta: {
      app: 'Tokengochi image-model pipeline', version: '1.0', image, format: 'RGBA8888',
      size: { w: columns * cell, h: rows * cell }, scale: '1',
      frameTags: tags.map(({ duration, ...tag }) => ({ ...tag, direction: 'forward' })),
    },
  };
}

const hatchlingMaster = path.join(root, 'source', 'hatchling-master.png');
const effectsMaster = path.join(root, 'source', 'effects-master.png');
const hatchlingCell = { width: 198, height: 165 };
const hatchlingFrames = [
  ...[0, 1, 2, 3].map((column) => ({ left: column * hatchlingCell.width + 19, top: 0, width: 160, height: 160 })),
  ...[0, 1, 2, 3, 4, 5].map((column) => ({ left: column * hatchlingCell.width + 19, top: 165, width: 160, height: 160 })),
  ...[0, 1, 2, 3].map((column) => ({ left: column * hatchlingCell.width + 19, top: 330, width: 160, height: 160 })),
  ...[0, 1, 2, 3, 4, 5].map((column) => ({ left: column * hatchlingCell.width + 19, top: 495, width: 160, height: 160 })),
  ...[0, 1, 2, 3, 4, 5].map((column) => ({ left: column * hatchlingCell.width + 19, top: 660, width: 160, height: 160 })),
  ...[4, 5].map((column) => ({ left: column * hatchlingCell.width + 19, top: 660, width: 160, height: 160 })),
  ...[0, 1, 2].map((column) => ({ left: column * hatchlingCell.width + 19, top: 825, width: 160, height: 160 })),
];
const effectFrames = [
  { left: 55, top: 155, width: 180, height: 180 }, { left: 345, top: 155, width: 200, height: 200 }, { left: 640, top: 140, width: 210, height: 210 },
  { left: 55, top: 470, width: 190, height: 190 }, { left: 345, top: 450, width: 210, height: 210 }, { left: 650, top: 455, width: 190, height: 190 },
  { left: 55, top: 750, width: 190, height: 190 }, { left: 340, top: 730, width: 220, height: 220 }, { left: 630, top: 710, width: 240, height: 240 }, { left: 945, top: 735, width: 210, height: 210 },
];

const hatchlingTags = [
  { name: 'idle', from: 0, to: 3, duration: 150 }, { name: 'walk', from: 4, to: 9, duration: 100 },
  { name: 'sleep', from: 10, to: 13, duration: 250 }, { name: 'eat', from: 14, to: 19, duration: 100 },
  { name: 'happy', from: 20, to: 25, duration: 100 }, { name: 'drag', from: 26, to: 27, duration: 150 },
  { name: 'react', from: 28, to: 30, duration: 100 },
];
const effectTags = [
  { name: 'zzz', from: 0, to: 2, duration: 300 }, { name: 'heart', from: 3, to: 4, duration: 200 },
  { name: 'exclaim', from: 5, to: 5, duration: 400 }, { name: 'dust', from: 6, to: 9, duration: 80 },
];

await Promise.all([
  buildSheet(hatchlingMaster, hatchlingFrames, 32, 8, path.join(root, 'hatchling', 'hatchling.png'), 5),
  buildSheet(effectsMaster, effectFrames, 16, 8, path.join(root, 'effects', 'effects.png')),
  fs.writeFile(path.join(root, 'hatchling', 'hatchling.json'), `${JSON.stringify(asepriteJson('hatchling.png', 32, 8, 31, hatchlingTags, 5), null, 2)}\n`),
  fs.writeFile(path.join(root, 'effects', 'effects.json'), `${JSON.stringify(asepriteJson('effects.png', 16, 8, 10, effectTags), null, 2)}\n`),
]);
