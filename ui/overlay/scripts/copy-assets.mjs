// Cross-platform replacement for the old `mkdir -p && cp *.png *.json` build
// step: `beforeBuildCommand` on Windows runs under cmd.exe, which supports
// neither `mkdir -p`/`cp` nor shell glob expansion, so that step silently
// failed every Windows build.
import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const overlayDir = dirname(dirname(fileURLToPath(import.meta.url)));
const assetsDir = join(overlayDir, "..", "assets", "sprites");
const staticDir = join(overlayDir, "..", "dashboard", "static", "overlay");
const spritesDir = join(staticDir, "sprites");
const itemsDir = join(spritesDir, "items");

mkdirSync(spritesDir, { recursive: true });
mkdirSync(itemsDir, { recursive: true });

copyFileSync(join(overlayDir, "index.html"), join(staticDir, "index.html"));

function copyMatching(sourceDir, destDir, extensions) {
  for (const entry of readdirSync(sourceDir)) {
    if (extensions.some((ext) => entry.endsWith(ext))) {
      copyFileSync(join(sourceDir, entry), join(destDir, entry));
    }
  }
}

copyMatching(join(assetsDir, "hatchling"), spritesDir, [".png", ".json"]);
copyMatching(join(assetsDir, "effects"), spritesDir, [".png", ".json"]);
copyMatching(join(assetsDir, "items"), itemsDir, [".png"]);
