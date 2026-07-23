#!/usr/bin/env python3
"""Merge/verify helper for the Tauri updater manifest (`latest.json`).

The release workflow builds `latest.json` separately on each platform job
(windows, linux, and each macOS arch), and every job writes the *same*
`latest.json` release asset. GitHub release assets have no atomic
compare-and-swap, so the macOS job converges instead: it downloads whatever
is currently published, unions the `platforms` map with its own, re-uploads,
and then verifies its platform keys survived. This script does the two
JSON-manipulation halves of that loop.

Usage:
  merge-updater-manifest.py merge  <remote.json> <local.json> <out.json>
  merge-updater-manifest.py verify <published.json> <local.json>

`merge` preserves every platform already published (windows, linux, the other
macOS arch) and adds/overwrites this build's macOS platform(s), taking the
top-level version/notes/pub_date from the freshly built local manifest.

`verify` exits 0 if every platform in the local manifest is present in the
published manifest, or 1 if a concurrent writer clobbered any of them.
"""

import json
import sys


def load_platforms(path):
    with open(path) as f:
        return json.load(f).get("platforms", {})


def merge(remote_path, local_path, out_path):
    with open(remote_path) as f:
        remote = json.load(f)
    with open(local_path) as f:
        local = json.load(f)

    platforms = dict(remote.get("platforms", {}))
    platforms.update(local.get("platforms", {}))
    merged = dict(local)
    merged["platforms"] = platforms

    with open(out_path, "w") as f:
        json.dump(merged, f, indent=2)


def verify(published_path, local_path):
    published = set(load_platforms(published_path))
    ours = set(load_platforms(local_path))
    missing = ours - published
    if missing:
        print("missing platforms in published manifest: " + ", ".join(sorted(missing)))
        return 1
    return 0


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    command = argv[1]
    if command == "merge" and len(argv) == 5:
        merge(argv[2], argv[3], argv[4])
        return 0
    if command == "verify" and len(argv) == 4:
        return verify(argv[2], argv[3])
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
