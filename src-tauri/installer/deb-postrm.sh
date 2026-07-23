#!/bin/sh
# Debian maintainer script: postrm (runs after the package is removed).
#
# Wired into the bundle via `tauri.conf.json` -> bundle.linux.deb.postRemoveScript.
# Goal: on a real removal, delete Tokengochi's persisted data so a later
# reinstall starts from a brand-new, empty database. dpkg calls postrm with an
# argument describing *why*:
#   remove   -> `apt-get remove`  (drop the app, but this could precede a
#               reinstall, so we keep data)
#   purge    -> `apt-get purge`   (the explicit "also delete data" action)
#   upgrade  -> in-place update    (never touch data)
# We therefore delete data ONLY on `purge`, which never fires during an upgrade.
#
# The script runs as root, but the data lives under each user's data dir
# (`dirs::data_dir()` = $XDG_DATA_HOME or ~/.local/share; see
# src-tauri/src/storage_paths.rs). We best-effort sweep every real home plus
# root's, since dpkg does not tell us which user installed the app.

set -e

if [ "$1" != "purge" ]; then
    exit 0
fi

for home in /root /home/*; do
    [ -d "$home" ] || continue
    data_dir="${XDG_DATA_HOME:-$home/.local/share}"
    case "$data_dir" in
        "$home"/*) ;;                       # honor XDG only when it lives here
        *) data_dir="$home/.local/share" ;;
    esac
    rm -rf "$data_dir/com.tokengochi.app" "$data_dir/tokengochi"
done

exit 0
