# Tokengochi

A desktop pet that lives on your screen and grows on the tokens you burn using AI coding tools. See [docs/product.md](docs/product.md) for the full pitch and [docs/architecture.md](docs/architecture.md) for how it's built.

## Installing

Download the installer for your OS from the [latest release](https://github.com/dodoxtech/tokengochi/releases/latest).

| OS | File |
| --- | --- |
| macOS (Apple Silicon) | `Tokengochi_x.y.z_aarch64.dmg` |
| macOS (Intel) | `Tokengochi_x.y.z_x64.dmg` |
| Windows | `Tokengochi_x.y.z_x64-setup.exe` |
| Linux (Debian/Ubuntu) | `tokengochi_x.y.z_amd64.deb` |
| Linux (other distros) | `tokengochi_x.y.z_amd64.AppImage` |

> **These builds are unsigned for the MVP release** (see [ADR-0004](docs/decisions/0004-unsigned-mvp-release.md)). Your OS will warn you before first launch — this is expected:
>
> - **macOS:** Gatekeeper blocks the app ("cannot be opened because the developer cannot be verified"). Right-click the app → **Open** → **Open** again in the dialog. If that doesn't work, run `xattr -dr com.apple.quarantine /Applications/Tokengochi.app` in Terminal.
> - **Windows:** SmartScreen shows "Windows protected your PC". Click **More info** → **Run anyway**.
> - **Linux:** no OS-level warning; mark the AppImage executable (`chmod +x`) before running it.

Once installed, Tokengochi launches into onboarding: pick a starter egg, and it auto-detects Claude Code usage if present. The app lives in your system tray — closing the dashboard window hides it, quit from the tray menu.

## Updating

Tokengochi checks GitHub Releases for new versions. Open the dashboard → **Settings** → **Check for updates**. Update downloads and installs are cryptographically verified independent of OS code signing; your pet's state (level, streak, inventory) is stored locally and survives updates.

## Development

```sh
npm ci --prefix ui/dashboard
npm ci --prefix ui/overlay
cargo tauri dev   # run from src-tauri/
```

See [docs/README.md](docs/README.md) for the full documentation map, and [docs/knowledge/release-process.md](docs/knowledge/release-process.md) for how to cut a tagged release.
